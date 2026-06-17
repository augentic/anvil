//! Projects a schema-validated agent reconciliation response onto
//! `plan.yaml.slices[]`, enforcing the semantic invariants the schema
//! gate (`validate_proposal_json`) cannot express. See DECISIONS.md
//! §"Lead reconciliation".

use std::collections::{BTreeSet, HashMap, HashSet};

use petgraph::algo::tarjan_scc;
use specify_diagnostics::{
    Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity, blocking, fingerprint,
};
use specify_error::{Error, Result, is_kebab};
use specify_model::discovery::Discovery;

use super::super::model::{
    Entry, Plan, SliceAuthorityOverride, SliceSourceBinding, Status, TargetRef,
};
use super::super::validate::entry_dependency_graph;
use super::catalog::{LeadCatalog, build_catalog};
use super::wire::{ProjectRef, ProposalResponse, ResponseMember, ResponseSlice};
use crate::registry::topology::Decision;

/// Outcome of a successful [`Plan::propose_from`] projection.
///
/// The reconciliation kernel returns it so the CLI handler (the
/// `propose --from` command) can emit the single D2
/// `plan.reconcile.completed` journal event without re-deriving
/// anything: [`ProposeOutcome::slice_names`] (plus its length) feeds the
/// payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposeOutcome {
    /// Slice names, in the agent's `slices[]` response order — the same
    /// order the kernel wrote `plan.yaml.slices[]`.
    pub slice_names: Vec<String>,
    /// RFC-46 D3 advisory `lead-decision-topic-overlap` review findings:
    /// one per `(lead, decision)` pair whose `topics[]` intersect on the
    /// slice's bound project. Non-blocking — they surface a lead whose
    /// topic is already governed by an accepted decision so the agent can
    /// confirm alignment before Gate 1. Empty until both leads and
    /// decisions carry topics.
    pub topic_overlaps: Vec<Diagnostic>,
}

impl Plan {
    /// Project a validated agent reconciliation response onto
    /// `plan.yaml.slices[]` (DECISIONS.md §"Lead reconciliation").
    ///
    /// `response` is assumed to have already passed JSON-Schema
    /// validation (`validate_proposal_json`) at the CLI boundary, so this
    /// method enforces only the *semantic* invariants the schema cannot
    /// express. The checks fire in this order, returning the first
    /// violation: replaceable gate (`plan-reconcile-plan-not-replaceable`); lead-orphan (`plan-reconcile-lead-orphan`); per-slice same-source fusion (`plan-reconcile-slice-source-collision`); total lead coverage (`lead-coverage-orphan`); project auto-bind / orphan (`plan-reconcile-project-binding-required`, `plan-reconcile-project-orphan`); slice-name kebab-case + collision (`plan-reconcile-slice-name-invalid`, `plan-reconcile-slice-name-collision`); `depends-on` cycle (`plan-reconcile-depends-on-cycle`); and finally a backstop [`Plan::validate`] over the projected entries that rolls the plan back on any blocking finding.
    ///
    /// On success `self.entries` is the projected slice set in response
    /// order and the returned [`ProposeOutcome`] carries the slice names
    /// for the caller's `plan.reconcile.completed` journal event.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Validation`] (exit 2) carrying the first
    /// invariant code listed above that the response violates.
    pub fn propose_from(
        &mut self, response: ProposalResponse, discovery: &Discovery, topology: &[ProjectRef],
    ) -> Result<ProposeOutcome> {
        if !self.is_replaceable() {
            return Err(Error::validation_failed(
                "plan-reconcile-plan-not-replaceable",
                "propose --from requires a replaceable plan",
                "lifecycle is approved or any entry is in-progress or done",
            ));
        }

        let catalog = build_catalog(discovery);
        let slices = response.slices.as_slice();

        check_lead_orphans(slices, &catalog)?;
        let source_sets = slice_source_sets(slices)?;
        check_coverage(&source_sets, &catalog)?;
        let bound = bind_projects(slices, topology)?;
        // Eagerly validate that every bound project's target parses as
        // `name@vN` so a corrupt topology fails at propose time, even
        // though the resolved target is not written to disk.
        for project in &bound {
            parse_project_target(project)?;
        }
        let names = slice_names(slices)?;
        check_name_collisions(&names)?;

        // RFC-46 D3: advisory topic-overlap review findings, computed
        // before `build_entries` consumes the response slices. Latent
        // (empty) until both leads and decisions carry topics.
        let topic_overlaps = topic_overlaps(slices, discovery, &bound);

        let new_entries = build_entries(response.slices, &names, &bound);

        if has_dependency_cycle(&new_entries) {
            return Err(Error::validation_failed(
                "plan-reconcile-depends-on-cycle",
                "the depends-on graph must be acyclic",
                "the projected slices form a depends-on cycle",
            ));
        }

        // Bulk replace, run the backstop validate, and roll back on any
        // blocking finding (e.g. unknown depends-on names).
        let previous = std::mem::replace(&mut self.entries, new_entries);
        if let Some(finding) = self.validate(None, None).into_iter().find(blocking) {
            self.entries = previous;
            return Err(Error::validation_failed(
                finding.rule_id.clone().unwrap_or_default(),
                String::new(),
                finding.impact,
            ));
        }

        Ok(ProposeOutcome {
            slice_names: names,
            topic_overlaps,
        })
    }
}

/// RFC-46 D3 — join each slice's lead `topics[]` against its bound
/// project's accepted-decision `topics[]`, emitting an advisory
/// `lead-decision-topic-overlap` review finding per intersecting
/// `(lead, decision)` pair.
///
/// The CLI never groups on topics; this surfaces a lead whose topic is
/// already governed by an accepted decision so the agent can confirm the
/// slice aligns (or flags a contradiction) before Gate 1. Non-blocking
/// `kind: review`; degrades to an empty vec until both surveyed leads and
/// baseline decisions carry topics.
fn topic_overlaps(
    slices: &[ResponseSlice], discovery: &Discovery, bound: &[&ProjectRef],
) -> Vec<Diagnostic> {
    let mut lead_topics: HashMap<(&str, &str), &[String]> = HashMap::new();
    for lead in discovery.leads() {
        lead_topics.insert((lead.source.as_str(), lead.lead.as_str()), lead.topics.as_slice());
    }

    let mut out = Vec::new();
    for (idx, slice) in slices.iter().enumerate() {
        let project = bound[idx];
        if project.decisions.is_empty() {
            continue;
        }
        for member in &slice.sources {
            let topics = lead_topics.get(&(member.source.as_str(), member.lead.as_str())).copied();
            let Some(topics) = topics else { continue };
            for topic in topics {
                for decision in &project.decisions {
                    if decision.topics.iter().any(|d| d == topic) {
                        out.push(topic_overlap_finding(&slice.name, member, topic, decision));
                    }
                }
            }
        }
    }
    out
}

/// Build one advisory `lead-decision-topic-overlap` review finding.
fn topic_overlap_finding(
    slice: &str, member: &ResponseMember, topic: &str, decision: &Decision,
) -> Diagnostic {
    let message = format!(
        "lead ({}, {}) topic '{topic}' is governed by accepted decision {} ('{}'); \
         confirm slice '{slice}' aligns with it",
        member.source, member.lead, decision.id, decision.title
    );
    let mut diagnostic = Diagnostic::finding(
        "lead-decision-topic-overlap".to_string(),
        message.clone(),
        message,
        Severity::Suggestion,
        DiagnosticKind::Review,
        DiagnosticSource::Deterministic,
        Artifact::Plan,
        None,
    );
    diagnostic.slice = Some(slice.to_string());
    diagnostic.fingerprint = fingerprint(&diagnostic);
    diagnostic
}

/// A `(source, lead)` catalog identity.
type LeadPair = (String, String);

/// The order-insensitive `sources[]` membership of one slice.
type SliceMembership = BTreeSet<LeadPair>;

/// Lead-orphan: every cited `(source, lead)` must name a current
/// catalog row (`plan-reconcile-lead-orphan`).
fn check_lead_orphans(slices: &[ResponseSlice], catalog: &LeadCatalog) -> Result<()> {
    for slice in slices {
        for member in &slice.sources {
            if !catalog.contains(&member.source, &member.lead) {
                return Err(Error::validation_failed(
                    "plan-reconcile-lead-orphan",
                    "every cited source binding must name a surveyed lead",
                    format!(
                        "({}, {}) is not in the discovery.md lead catalog",
                        member.source, member.lead
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Per-slice source set, enforcing at-most-one-lead-per-source
/// (`plan-reconcile-slice-source-collision`). A slice that names the
/// same source twice is malformed regardless of slice membership.
fn slice_source_sets(slices: &[ResponseSlice]) -> Result<Vec<SliceMembership>> {
    let mut out = Vec::with_capacity(slices.len());
    for slice in slices {
        let mut set = BTreeSet::new();
        let mut keys: HashSet<&str> = HashSet::new();
        for member in &slice.sources {
            if !keys.insert(member.source.as_str()) {
                return Err(Error::validation_failed(
                    "plan-reconcile-slice-source-collision",
                    "a slice names at most one lead per source",
                    format!(
                        "slice '{}' names source '{}' more than once",
                        slice.name, member.source
                    ),
                ));
            }
            set.insert((member.source.clone(), member.lead.clone()));
        }
        out.push(set);
    }
    Ok(out)
}

/// Total lead coverage: every surveyed lead is referenced by at least
/// one slice (`lead-coverage-orphan`). A lead may legally appear in
/// more than one slice — that is fan-out, not a double-count — so
/// coverage is the only invariant: nothing surveyed is left unplanned.
fn check_coverage(source_sets: &[SliceMembership], catalog: &LeadCatalog) -> Result<()> {
    let mut covered: HashSet<(&str, &str)> = HashSet::new();
    for set in source_sets {
        for pair in set {
            covered.insert((pair.0.as_str(), pair.1.as_str()));
        }
    }
    for (source, lead) in catalog.iter() {
        if !covered.contains(&(source, lead)) {
            return Err(Error::validation_failed(
                "lead-coverage-orphan",
                "every surveyed lead must be referenced by at least one slice",
                format!("lead ({source}, {lead}) is unaccounted for by any slice"),
            ));
        }
    }
    Ok(())
}

/// Bind a single slice to its project in `topology`: an explicit
/// `project` must exist (`plan-reconcile-project-orphan`); an omitted
/// `project` auto-binds the sole project or fails when several exist
/// (`plan-reconcile-project-binding-required`). `slice_name` labels the
/// diagnostics.
///
/// This is the single project-binding rule shared by the propose kernel
/// ([`bind_projects`]) and the read-time target resolver
/// ([`resolve_target`]) so the two cannot drift.
fn resolve_project_binding<'a>(
    slice_name: &str, project: Option<&str>, topology: &'a [ProjectRef],
) -> Result<&'a ProjectRef> {
    match project {
        Some(name) => topology.iter().find(|p| p.name.as_str() == name).ok_or_else(|| {
            Error::validation_failed(
                "plan-reconcile-project-orphan",
                "a bound project must exist in the request topology",
                format!("slice '{slice_name}' binds unknown project '{name}'"),
            )
        }),
        None if topology.len() == 1 => Ok(&topology[0]),
        None => Err(Error::validation_failed(
            "plan-reconcile-project-binding-required",
            "a slice may omit project only when exactly one project exists",
            format!(
                "slice '{slice_name}' omits project but {} projects are available",
                topology.len()
            ),
        )),
    }
}

/// Bind each slice to a project via [`resolve_project_binding`].
fn bind_projects<'a>(
    slices: &[ResponseSlice], topology: &'a [ProjectRef],
) -> Result<Vec<&'a ProjectRef>> {
    slices
        .iter()
        .map(|slice| resolve_project_binding(&slice.name, slice.project.as_deref(), topology))
        .collect()
}

/// Resolve a single slice [`Entry`]'s target adapter from the project
/// topology.
///
/// A slice binds only a `project`; the target adapter (`name@vN`) is
/// derived here from the bound project's [`ProjectRef::target`]. This is
/// the single read-time resolver every consumer (`specify plan next`,
/// slice `metadata.yaml` population, the build request) routes through,
/// so `plan.yaml` never needs to store the denormalised target.
///
/// Binding mirrors the propose kernel's project binding: an explicit
/// [`Entry::project`] must exist in `topology`; an omitted project
/// auto-binds the sole topology project.
///
/// # Errors
///
/// - `plan-reconcile-project-orphan` when the named project is absent
///   from `topology`.
/// - `plan-reconcile-project-binding-required` when `project` is omitted
///   but more than one project exists.
/// - `plan-target-malformed` when the bound project's target does not
///   parse as `name@<semver>` (an internal inconsistency — topology
///   targets are pre-validated).
pub fn resolve_target(entry: &Entry, topology: &[ProjectRef]) -> Result<TargetRef> {
    let project = resolve_project_binding(&entry.name, entry.project.as_deref(), topology)?;
    parse_project_target(project)
}

/// Parse a [`ProjectRef`]'s `name@<semver>` target into a [`TargetRef`].
///
/// Topology targets are pre-validated, so a parse failure is an internal
/// inconsistency surfaced as `plan-target-malformed`.
fn parse_project_target(project: &ProjectRef) -> Result<TargetRef> {
    TargetRef::parse(&project.target).map_err(|err| {
        Error::validation_failed(
            "plan-target-malformed",
            "a project target must parse as name@<semver>",
            err.to_string(),
        )
    })
}

/// Collect each slice's explicit `name`, validating kebab-case
/// (`plan-reconcile-slice-name-invalid`). The agent names every slice
/// and the kernel writes the name verbatim — there is no derivation.
fn slice_names(slices: &[ResponseSlice]) -> Result<Vec<String>> {
    let mut names = Vec::with_capacity(slices.len());
    for slice in slices {
        if !is_kebab(&slice.name) {
            return Err(Error::validation_failed(
                "plan-reconcile-slice-name-invalid",
                "a slice name must be kebab-case",
                format!("slice name '{}' is not kebab-case", slice.name),
            ));
        }
        names.push(slice.name.clone());
    }
    Ok(names)
}

/// Reject clashing slice names (`plan-reconcile-slice-name-collision`).
/// With names agent-supplied on every slice, this is the sole
/// uniqueness gate over slice names.
fn check_name_collisions(names: &[String]) -> Result<()> {
    let mut seen: HashSet<&str> = HashSet::new();
    for name in names {
        if !seen.insert(name.as_str()) {
            return Err(Error::validation_failed(
                "plan-reconcile-slice-name-collision",
                "explicit slice names must be unique",
                format!("two slices resolve to the name '{name}'"),
            ));
        }
    }
    Ok(())
}

/// Project the validated response into `plan.yaml.slices[]` entries in
/// response order, consuming the response's slices.
///
/// Each entry binds only its `project`; the target adapter is resolved
/// on demand from the topology via [`resolve_target`] and is not written
/// to disk.
fn build_entries(
    slices: Vec<ResponseSlice>, names: &[String], bound: &[&ProjectRef],
) -> Vec<Entry> {
    slices
        .into_iter()
        .enumerate()
        .map(|(idx, slice)| Entry {
            name: names[idx].clone().into(),
            project: Some(bound[idx].name.clone()),
            status: Status::Pending,
            depends_on: slice.depends_on.into_iter().map(Into::into).collect(),
            sources: slice
                .sources
                .into_iter()
                .map(|m| SliceSourceBinding::structured(m.source, m.lead))
                .collect(),
            context: Vec::new(),
            description: None,
            divergence: slice.divergence,
            disagreements: slice.disagreements,
            authority_override: SliceAuthorityOverride::default(),
        })
        .collect()
}

/// `true` when the projected entries' `depends-on` graph contains a
/// cycle (including a self-loop), reusing the same Tarjan-SCC detection
/// as the `cycle-in-depends-on` doctor diagnostic.
fn has_dependency_cycle(entries: &[Entry]) -> bool {
    let graph = entry_dependency_graph(entries);
    tarjan_scc(&graph)
        .into_iter()
        .any(|scc| scc.len() > 1 || (scc.len() == 1 && graph.find_edge(scc[0], scc[0]).is_some()))
}
