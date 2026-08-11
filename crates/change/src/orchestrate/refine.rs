//! The serial refinement drain behind `emery plan refine` (RFC-91 D7):
//! a refinement-specific deterministic selector over the closed plan.
//! No epoch, no claims, no gap gate, and no target build operations.

use std::collections::{BTreeMap, BTreeSet};

use artifacts::discovery::{Discovery, Lead};
use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::Resolver;
use project::config::{Layout, ProjectConfig};
use project::handler::ExecutionPaths;
use project::plan::{
    Entry, Plan, Status, collect_events, in_scope, plan_gaps_body, project_ladders,
};
use project::seam::{Source, Target, Workspaces};
use project::slice::SliceMetadata;
use slice::refinement::{self, Dependency, Freshness};

use super::execute::GuestMarker;

/// How one [`refine`] drain ended. Both arms are successful returns —
/// a stop is the drain's typed halt surface, not an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefineOutcome {
    /// Every targeted leaf holds a fresh refinement manifest.
    Completed {
        /// Plan name from `plan.yaml.name`.
        plan: String,
        /// Slices refined by this run, in drain order.
        refined: Vec<String>,
        /// Targeted slices skipped because their manifest was already
        /// fresh, in drain order.
        skipped: Vec<String>,
        /// Whether the in-scope gap inventory is non-empty — fresh
        /// manifests may carry `[unknown]` / `[conflict]` /
        /// `[divergence]` review outputs.
        gaps: bool,
    },
    /// The drain halted on the first failed refinement (or an
    /// unrefineable predecessor); prior successful manifests stay.
    Stopped {
        /// Slice the stop is parked on.
        slice: String,
        /// The failing refinement's error detail.
        detail: String,
        /// Slices refined before the stop, in drain order.
        refined: Vec<String>,
    },
}

/// Run the serial refinement drain: walk in-scope plan entries in
/// topological order (plan order breaks ties) and refine every
/// targeted leaf whose manifest is missing or stale.
///
/// Without `selectors`, every in-scope leaf is targeted. With
/// selectors, the selected leaves are targeted plus their
/// stale-or-missing predecessor closure. Re-entry is safe: fresh
/// manifests are skipped, so a re-run resumes missing or stale work.
///
/// # Errors
///
/// - `guest-marker-held` (exit 2) when another guest run holds the
///   marker.
/// - [`Error::Argument`] when a `--slice` selector names no in-scope
///   plan entry.
/// - Plan / discovery / config load and freshness-projection failures.
///
/// Per-slice refinement failures do **not** surface here — they return
/// as [`RefineOutcome::Stopped`].
pub async fn refine<P: Model, S: Source, T: Target + Workspaces, R: Resolver>(
    caps: super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp,
    selectors: &[String],
) -> Result<RefineOutcome, Error> {
    let layout = Layout::new(paths.project_root());
    let config = ProjectConfig::load(layout.project_dir())?;
    let adapter = project::target_policy::project_adapter(caps.resolver, &config, paths)?;
    let plan = Plan::load(&layout.plan_path())?;
    let _marker = GuestMarker::acquire(layout, now)?;
    let discovery = Discovery::load(&layout.discovery_path())?;
    let inventory = discovery.leads();

    let ordered = topological(layout, &plan)?;
    let targets = target_set(layout, &plan, &ordered, inventory, selectors)?;

    let mut refined: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for entry in &ordered {
        let name = entry.name.as_str();
        if !targets.contains(name) {
            continue;
        }
        // Re-entry resumes missing/stale work; fresh leaves are never
        // repeated.
        if let Freshness::Fresh { .. } = refinement::freshness(layout, &plan, entry, inventory)? {
            skipped.push(name.to_string());
            continue;
        }
        // Dependent refinement requires every direct predecessor
        // currently fresh (RFC-91 D3); the fresh digests become the
        // ordered dependency pins.
        let dependencies = match predecessor_pins(layout, &plan, entry, inventory)? {
            Ok(dependencies) => dependencies,
            Err(detail) => {
                return Ok(RefineOutcome::Stopped {
                    slice: name.to_string(),
                    detail,
                    refined,
                });
            }
        };
        let target = match project::target_policy::resumed(layout, name) {
            Ok(target) => target,
            Err(_) => project::target_policy::fresh(caps.resolver, paths, entry, name, "refining")?,
        };
        tracing::info!("refine {name} …");
        match slice::orchestrate::refine(
            caps,
            paths,
            now,
            name,
            &target,
            dependencies,
            &adapter.manifest.inputs,
        )
        .await
        {
            Ok(_) => {
                tracing::info!("refine {name} — completed");
                refined.push(name.to_string());
            }
            Err(err) => {
                // Stop on the first failed refinement; prior manifests
                // stay and a re-run resumes here.
                tracing::info!("refine {name} — stopped: {err}");
                return Ok(RefineOutcome::Stopped {
                    slice: name.to_string(),
                    detail: err.to_string(),
                    refined,
                });
            }
        }
    }

    let gaps = !plan_gaps_body(&plan, layout)?.is_empty();
    Ok(RefineOutcome::Completed {
        plan: plan.name.to_string(),
        refined,
        skipped,
        gaps,
    })
}

/// Ordered `(slice, refinement-digest)` pins over `entry.depends_on`.
/// Returns `Err(detail)` (a typed stop, not a hard error) when a
/// direct predecessor's manifest is not currently fresh.
fn predecessor_pins(
    layout: Layout<'_>, plan: &Plan, entry: &Entry, inventory: &[Lead],
) -> Result<Result<Vec<Dependency>, String>, Error> {
    let mut dependencies = Vec::with_capacity(entry.depends_on.len());
    for dep in &entry.depends_on {
        let Some(dep_entry) = plan.entries.iter().find(|e| e.name == *dep) else {
            return Ok(Err(format!(
                "predecessor `{dep}` has no plan entry; fix the entry's depends-on list"
            )));
        };
        match refinement::freshness(layout, plan, dep_entry, inventory)? {
            Freshness::Fresh { digest } => dependencies.push(Dependency {
                slice: dep.as_str().to_string(),
                refinement: digest,
            }),
            Freshness::Missing => {
                return Ok(Err(format!(
                    "predecessor `{dep}` has no refinement manifest — dependent refinement \
                     requires predecessor refined; run `emery plan refine` without selectors or \
                     include the predecessor"
                )));
            }
            Freshness::Stale { reasons } => {
                return Ok(Err(format!(
                    "predecessor `{dep}` refinement is stale ({}) — re-refine it first",
                    reasons.first().map_or("drifted", String::as_str)
                )));
            }
        }
    }
    Ok(Ok(dependencies))
}

/// In-scope plan entries in topological order over `depends_on`, with
/// plan order as the tiebreak. Leaves projected `done` from the fact
/// union (merged, slice tree archived) are finished work and never
/// re-enter the drain. The plan is validated acyclic; any residue from
/// a malformed graph is appended in plan order so the drain still
/// reports per-slice failures instead of spinning.
fn topological<'a>(layout: Layout<'_>, plan: &'a Plan) -> Result<Vec<&'a Entry>, Error> {
    let events = collect_events(layout)?;
    let ladders = project_ladders(plan, &events);
    let mut remaining: Vec<&Entry> = plan
        .entries
        .iter()
        .filter(|entry| {
            let meta = SliceMetadata::load(&layout.slice_dir(entry.name.as_str())).ok();
            in_scope(plan, entry, meta.as_ref())
                && ladders.get(&entry.name).copied() != Some(Status::Done)
        })
        .collect();
    let names: BTreeSet<&str> = remaining.iter().map(|e| e.name.as_str()).collect();
    let mut emitted: BTreeSet<&str> = BTreeSet::new();
    let mut ordered = Vec::with_capacity(remaining.len());
    while !remaining.is_empty() {
        let Some(index) = remaining.iter().position(|entry| {
            entry
                .depends_on
                .iter()
                .all(|dep| emitted.contains(dep.as_str()) || !names.contains(dep.as_str()))
        }) else {
            break;
        };
        let entry = remaining.remove(index);
        emitted.insert(entry.name.as_str());
        ordered.push(entry);
    }
    ordered.extend(remaining);
    Ok(ordered)
}

/// The targeted leaf set. Empty `selectors` targets every in-scope
/// leaf; otherwise the selected leaves plus every transitive
/// predecessor that is stale or missing — or whose own predecessor is
/// included — so the selected work stays coherent.
///
/// # Errors
///
/// [`Error::Argument`] when a selector names no in-scope entry;
/// freshness-projection failures.
fn target_set(
    layout: Layout<'_>, plan: &Plan, ordered: &[&Entry], inventory: &[Lead], selectors: &[String],
) -> Result<BTreeSet<String>, Error> {
    if selectors.is_empty() {
        return Ok(ordered.iter().map(|entry| entry.name.as_str().to_string()).collect());
    }
    let by_name: BTreeMap<&str, &Entry> =
        ordered.iter().map(|entry| (entry.name.as_str(), *entry)).collect();
    let mut targets: BTreeSet<String> = BTreeSet::new();
    let mut closure: BTreeSet<&str> = BTreeSet::new();
    for selector in selectors {
        let Some(entry) = by_name.get(selector.as_str()) else {
            return Err(Error::Argument {
                flag: "--slice",
                detail: format!("`{selector}` names no in-scope plan entry"),
            });
        };
        targets.insert(selector.clone());
        let mut frontier: Vec<&str> =
            entry.depends_on.iter().map(project::name::SliceName::as_str).collect();
        while let Some(dep) = frontier.pop() {
            if !closure.insert(dep) {
                continue;
            }
            if let Some(dep_entry) = by_name.get(dep) {
                frontier.extend(dep_entry.depends_on.iter().map(project::name::SliceName::as_str));
            }
        }
    }
    // In topological order, a predecessor joins the target set when its
    // manifest is stale/missing or a deeper included predecessor will
    // re-refine and thereby invalidate it.
    for entry in ordered {
        let name = entry.name.as_str();
        if !closure.contains(name) {
            continue;
        }
        let dep_included = entry.depends_on.iter().any(|dep| targets.contains(dep.as_str()));
        let fresh = matches!(
            refinement::freshness(layout, plan, entry, inventory)?,
            Freshness::Fresh { .. }
        );
        if dep_included || !fresh {
            targets.insert(name.to_string());
        }
    }
    Ok(targets)
}
