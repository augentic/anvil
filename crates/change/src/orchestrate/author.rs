//! The plan-authoring orchestrator behind `/emery:plan`: scaffold →
//! survey fan-out → reconciliation judgment → persist → review prose →
//! `plan validate` doctor sweep.
//!
//! The run exits with the plan authored and the outcome carrying the
//! literal execute hint — the orchestrator never runs the plan
//! (execution stays operator-only).

use std::collections::BTreeMap;

use artifacts::atomic::bytes_write;
use artifacts::discovery::Discovery;
use diagnostics::has_blocking;
use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::Resolver;
use project::config::{Layout, Mutation, ProjectConfig, with_state};
use project::handler::ExecutionPaths;
use project::journal::{self, Event, EventKind};
use project::name::SliceName;
use project::plan::{
    GateProse, Plan, ProjectRef, ProposalResponse, SourceBinding, apply_greenfield_seed,
    author_gate, build_request, resolve_topology,
};
use project::registry::Registry;
use project::seam::Source;

use super::SurveyedSource;
use crate::judgment::propose::{self, GateContext};

/// The result of a completed [`author`]: the authored plan with its
/// proposed slices, plus the literal execute hint for the operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorOutcome {
    /// The authored plan's name.
    pub plan: String,
    /// Per-source survey results, in plan-binding order.
    pub surveyed: Vec<SurveyedSource>,
    /// Proposed slice names written to `plan.yaml.slices[]`, in the
    /// agent's response order.
    pub slices: Vec<String>,
    /// The literal closing hint the `/emery:plan` skill prints —
    /// execution stays operator-only, so the orchestrator relays the
    /// command instead of running it.
    pub hint: String,
}

/// Author one plan end-to-end: scaffold → survey fan-out → reconcile →
/// project slices → persist review prose → validate → exit for
/// operator review.
///
/// `bindings` is the desugared `--source` / `--intent` map (the shape
/// the scaffold kernel hands `Plan::init`).
///
/// # Errors
///
/// - `plan-author-workspace-unsupported` (exit 2) when the plan root is
///   a workspace — the skill's workspace routing has no in-guest
///   counterpart, mirroring the execute loop's refusal.
/// - `change-name-not-kebab` / `plan-already-exists` from the scaffold
///   kernel's gates.
/// - survey fan-out failures from [`super::survey_all`] (earlier
///   sources stay merged — the native partial-progress posture).
/// - the judgment leg's model / schema / kernel / gate-prose failures
///   once the repair budget is exhausted.
/// - `plan-structural-errors` when the doctor sweep finds blocking
///   findings after the write.
#[tracing::instrument(name = "plan.author", skip_all, fields(plan = %name))]
pub async fn author<P: Model, S: Source, R: Resolver>(
    caps: super::Capabilities<'_, P, S, (), R>, paths: &ExecutionPaths, now: Timestamp, name: &str,
    bindings: BTreeMap<String, SourceBinding>, force: bool,
) -> Result<AuthorOutcome, Error> {
    // Authoring never dispatches the target seam — the bundle carries
    // the unit placeholder (see `Capabilities::sans_targets`).
    let super::Capabilities {
        model,
        sources,
        resolver,
        ..
    } = caps;
    let layout = Layout::new(paths.project_root());
    refuse_workspace(layout)?;
    tracing::info!("plan authoring started");
    // Ensure every binding up front — before the scaffold write and
    // the survey fan-out — so an unresolvable adapter (missing pin,
    // `emery_floor`) fails fast with nothing on disk. Bindings
    // persist as typed: a bare name stays bare in `plan.yaml` (the
    // deployment resolves it local-first on every dispatch); only an
    // explicit package pin stamps a `version`.
    for binding in bindings.values() {
        resolver.ensure_source(&binding.selector(), paths).await?;
    }
    scaffold(layout, name, bindings, force)?;
    let surveyed = super::survey_all(sources, resolver, paths, now).await?;

    let discovery = Discovery::load(&layout.discovery_path())?;
    let topology = load_topology(resolver, paths)?;
    let request = build_request(&discovery, &topology)?;

    let plan = Plan::load(&layout.plan_path())?;
    let context = GateContext {
        plan: plan.name.as_str(),
        sources: &plan.sources,
    };
    // The check is the kernel-projection dry run against a throwaway
    // clone plus the gate-prose round-trip, so a grouping the kernel
    // would reject — or prose that would corrupt discovery.md — is
    // repaired in-loop rather than surfacing after the call.
    let mut response = propose::reconcile(model, &request, Some(context), |candidate| {
        let mut throwaway = plan.clone();
        throwaway.propose_from(candidate.clone(), &discovery, &topology)?;
        check_gate(candidate, name, &discovery)
    })
    .await?;
    let Some(gate) = response.gate.take() else {
        // Unreachable — the check refused gate-less answers — but a
        // typed refusal beats a panic if the invariant ever slips.
        return Err(gate_missing());
    };

    // The accepted projection runs inside the atomic write loop:
    // `propose_from` replaces `plan.entries`, `with_state` writes
    // `plan.yaml` on Ok and rolls back on any Err.
    let outcome = with_state::<Plan, _, _>(layout, "plan.yaml", |plan| {
        plan.propose_from(response, &discovery, &topology).map(Mutation::changed)
    })?;
    tracing::info!(slices = outcome.slice_names.len(), "plan written");

    // Only after the write commits: emit the reconcile event.
    let event = Event::new(
        now,
        EventKind::PlanReconcileCompleted {
            plan_name: plan.name.clone(),
            slice_count: outcome.slice_names.len(),
            slice_names: outcome.slice_names.iter().map(SliceName::from).collect(),
        },
    );
    journal::append_one(layout, &event)?;

    persist_gate_prose(layout, name, &gate, discovery)?;
    validate(layout)?;

    Ok(AuthorOutcome {
        plan: name.to_string(),
        surveyed,
        slices: outcome.slice_names,
        hint: gate_hint(name),
    })
}

/// The literal closing hint the `/emery:plan` skill prints.
fn gate_hint(name: &str) -> String {
    format!(
        "Plan `{name}` is authored. Review it, then run `emery plan execute` \
         to drive the slices (running it is your approval)."
    )
}

/// Refuse workspace-routed plan authoring: the `/emery:plan` skill
/// syncs workspace slots before surveying, and the guest collapse has
/// no counterpart yet — the shared [`super::routing`] classification
/// with this operation's own refusal code.
fn refuse_workspace(layout: Layout<'_>) -> Result<(), Error> {
    let Some(subject) = super::routing::classify(layout, None)?.refusal_subject() else {
        return Ok(());
    };
    Err(Error::validation_failed(
        "plan-author-workspace-unsupported",
        "the guest plan-authoring collapse runs single-project plans only",
        format!(
            "{subject}; workspace routing (slot sync) has no in-guest counterpart — author \
             workspace plans through the native /emery:plan skill"
        ),
    ))
}

/// The plan scaffold via the shared [`project::plan::scaffold`]
/// kernel, plus the immediate atomic save. `force` opts into
/// recreating any existing plan. No `--authority-override` surface —
/// override pre-seeding needs slice rows that do not exist yet.
fn scaffold(
    layout: Layout<'_>, name: &str, bindings: BTreeMap<String, SourceBinding>, force: bool,
) -> Result<(), Error> {
    let plan_path = layout.plan_path();
    project::plan::scaffold(&plan_path, name, bindings, force)?.save(&plan_path)
}

/// Resolve the project topology the request embeds, minus the
/// operator-facing `greenfield-seed-shadowed` advisories (the seed
/// projection itself still applies).
fn load_topology(
    resolver: &impl Resolver, paths: &ExecutionPaths,
) -> Result<Vec<ProjectRef>, Error> {
    let layout = Layout::new(paths.project_root());
    let config = ProjectConfig::load(layout.project_dir())?;
    let mut topology = resolve_topology(resolver, &config, paths)?;
    if let Some(registry) = Registry::load(layout.project_dir())? {
        let _shadowed =
            apply_greenfield_seed(&mut topology, &registry, layout.project_dir(), config.workspace);
    }
    Ok(topology)
}

/// The gate-prose leg of the repair-loop check: the answer must carry
/// the `gate` object and its discovery preamble must round-trip
/// through the discovery parser without corrupting the inventory.
fn check_gate(
    candidate: &ProposalResponse, name: &str, discovery: &Discovery,
) -> Result<(), Error> {
    let Some(gate) = &candidate.gate else {
        return Err(gate_missing());
    };
    let mut probe = discovery.clone();
    probe.set_preamble(&discovery_preamble(name, gate))
}

fn gate_missing() -> Error {
    Error::validation_failed(
        "plan-author-gate-missing",
        "the reconciliation answer carries the review prose",
        "add the `gate` object (`change`, `discovery-summary`, `discovery-source-inventory`) \
         alongside `slices[]`",
    )
}

/// Persist the review prose: `change.md` framed under the
/// deterministic `# Change — <name>` heading, and the `discovery.md`
/// preamble through the validated [`Discovery::set_preamble`] writer
/// (the lead inventory rides through untouched).
fn persist_gate_prose(
    layout: Layout<'_>, name: &str, gate: &GateProse, mut discovery: Discovery,
) -> Result<(), Error> {
    let brief = format!("# Change — {name}\n\n{}\n", gate.change.trim());
    bytes_write(&layout.change_brief_path(), brief.as_bytes())?;

    discovery.set_preamble(&discovery_preamble(name, gate))?;
    discovery.write_atomic(&layout.discovery_path())
}

/// Compose the deterministic `discovery.md` three-section frame around
/// the model-authored section bodies.
fn discovery_preamble(name: &str, gate: &GateProse) -> String {
    format!(
        "# Discovery — {name}\n\n## Summary\n\n{}\n\n## Source inventory\n\n{}",
        gate.discovery_summary.trim(),
        gate.discovery_source_inventory.trim()
    )
}

/// The post-write author gate — the doctor sweep minus the stdout
/// report surface (the blocking decision and error code match the
/// native verb).
fn validate(layout: Layout<'_>) -> Result<(), Error> {
    let plan = Plan::load(&layout.plan_path())?;
    let findings = author_gate(&plan, &layout.slices_dir(), layout.project_dir())?;
    if has_blocking(&findings) {
        return Err(Error::validation_failed(
            "plan-structural-errors",
            "plan must be free of structural errors",
            "run 'emery plan validate' for detail",
        ));
    }
    Ok(())
}
