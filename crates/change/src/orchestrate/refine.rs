//! The refinement drain behind `emery plan refine` (RFC-91 D7),
//! fanning independent leaves onto the bounded pool (RFC-96 D5). No
//! epoch, no durable claims, no gap gate, no target build operations.

use std::collections::{BTreeMap, BTreeSet};

use artifacts::leads::{Lead, Leads};
use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::Resolver;
use project::config::{Layout, ProjectConfig};
use project::handler::ExecutionPaths;
use project::journal::{self, Event, EventKind, ParkReason};
use project::plan::{
    Entry, Plan, Proposal, Status, collect_events, in_scope, plan_gaps_body, project_ladders,
};
use project::pool;
use project::profile::Profiles;
use project::seam::{Shelf, Source, Target, Workspaces};
use project::slice::SliceMetadata;
use slice::refinement::{self, Dependency, Freshness, Live};

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
    /// Payload-free beyond the stop identity — re-running the drain is
    /// the resume path.
    Stopped {
        /// Slice the stop is parked on.
        slice: String,
        /// The failing refinement's error detail.
        detail: String,
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
/// - Plan / leads-catalog / config load and freshness-projection failures.
///
/// Per-slice refinement failures do **not** surface here — they return
/// as [`RefineOutcome::Stopped`].
pub async fn refine<
    P: Model + Profiles + Resolver + Source + Workspaces,
    S: Source + Workspaces,
    T: Target + Shelf,
    R: Resolver,
>(
    caps: super::Capabilities<'_, P, S, T, R>, paths: &ExecutionPaths, now: Timestamp,
    selectors: &[String],
) -> Result<RefineOutcome, Error> {
    let layout = paths.layout();
    if !paths.is_detached() {
        drop(ProjectConfig::load(paths.project_root())?);
    }
    let plan = Plan::load(&layout.plan_path())?;
    let _marker = GuestMarker::acquire(layout, now)?;
    let catalog = Leads::load(&layout.leads_path())?;
    let inventory = catalog.leads();

    let ordered = topological(layout, &plan)?;
    // One shared freshness cache per drain: baseline, journaled
    // post-merge baseline, source trees, and target binding hold still;
    // manifest digests are never cached (the drain rewrites them).
    let mut live = Live::new();
    let targets = target_set(layout, &plan, &ordered, inventory, selectors, &mut live)?;
    let claims = pool::Claims::default();

    // Independent leaves refine concurrently in rounds (RFC-96 D5);
    // outcomes join in topological order — never completion order —
    // and dependents dispatch in a later round over fresh pins.
    let mut refined: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut handled: BTreeSet<String> = BTreeSet::new();
    loop {
        let Round {
            leaves: round,
            stop: pending_stop,
        } = next_round(
            caps.resolver,
            paths,
            &plan,
            &ordered,
            &targets,
            inventory,
            &mut handled,
            &mut skipped,
            &mut live,
        )?;

        if round.is_empty() {
            if let Some(stop) = pending_stop {
                return Ok(stop);
            }
            break;
        }
        let jobs: Vec<pool::Job<'_, slice::orchestrate::RefineOutcome, Error>> = round
            .iter()
            .map(|leaf| {
                tracing::info!("refine {} …", leaf.name);
                pool::Job {
                    claim: pool::Claim {
                        item: leaf.name.clone(),
                        operation: "refine".to_string(),
                        attempt: 1,
                    },
                    budget: pool::budget::JUDGMENT,
                    future: Box::pin(slice::orchestrate::refine(
                        caps,
                        paths,
                        now,
                        &leaf.name,
                        &leaf.target,
                        leaf.dependencies.clone(),
                        &leaf.adapter.manifest.inputs,
                    )),
                }
            })
            .collect();
        let outcomes = pool::run(pool::cap(), &claims, pool::OnFailure::Drain, jobs).await;

        for (leaf, outcome) in round.iter().zip(outcomes) {
            let name = leaf.name.as_str();
            match settle_leaf(outcome, name) {
                Ok(slice::orchestrate::RefineOutcome::Refined { .. }) => {
                    tracing::info!("refine {name} — completed");
                    refined.push(name.to_string());
                    handled.insert(name.to_string());
                }
                Ok(escalation @ slice::orchestrate::RefineOutcome::Escalated { .. }) => {
                    return persist_escalation(
                        caps.model, paths, now, &plan, layout, name, escalation,
                    )
                    .await;
                }
                Err(err) => {
                    tracing::info!("refine {name} — stopped: {err}");
                    return Ok(RefineOutcome::Stopped {
                        slice: name.to_string(),
                        detail: err.to_string(),
                    });
                }
            }
        }
        if let Some(stop) = pending_stop {
            return Ok(stop);
        }
    }

    let events = collect_events(layout)?;
    let gaps = !plan_gaps_body(&plan, layout, &events)?.is_empty();
    Ok(RefineOutcome::Completed {
        plan: plan.name.to_string(),
        refined,
        skipped,
        gaps,
    })
}

/// One dispatched round member: the leaf's resolved target,
/// dependency pins, and bound adapter.
struct RoundLeaf {
    name: String,
    target: String,
    dependencies: Vec<Dependency>,
    adapter: project::adapter::ResolvedTarget,
}

/// One round's dispatch set plus the stop that closes the drain once
/// the current round settles.
struct Round {
    leaves: Vec<RoundLeaf>,
    stop: Option<RefineOutcome>,
}

/// Select the next concurrent round: every targeted, unhandled leaf
/// whose targeted predecessors are all handled. Fresh leaves are
/// skipped in place; an unready predecessor or a parked boundary
/// proposal is the drain's stop — leaves after it never dispatch.
#[expect(
    clippy::too_many_arguments,
    reason = "the round selector reads the drain's full working state"
)]
fn next_round(
    resolver: &impl Resolver, paths: &ExecutionPaths, plan: &Plan, ordered: &[&Entry],
    targets: &BTreeSet<String>, inventory: &[Lead], handled: &mut BTreeSet<String>,
    skipped: &mut Vec<String>, live: &mut Live,
) -> Result<Round, Error> {
    let layout = paths.layout();
    let mut leaves: Vec<RoundLeaf> = Vec::new();
    for entry in ordered {
        let name = entry.name.as_str();
        if !targets.contains(name) || handled.contains(name) {
            continue;
        }
        // Re-entry resumes missing/stale work; fresh leaves are never
        // repeated.
        if let Freshness::Fresh { .. } =
            refinement::freshness_with(layout, plan, entry, inventory, live)?
        {
            skipped.push(name.to_string());
            handled.insert(name.to_string());
            continue;
        }
        // A targeted predecessor still awaiting refinement defers this
        // leaf to a later round.
        if entry
            .depends_on
            .iter()
            .any(|dep| targets.contains(dep.as_str()) && !handled.contains(dep.as_str()))
        {
            continue;
        }
        // Dependent refinement requires every direct predecessor
        // currently fresh (RFC-91 D3); the fresh digests become the
        // ordered dependency pins.
        let dependencies = match predecessor_pins(layout, plan, entry, inventory, live)? {
            Ok(dependencies) => dependencies,
            Err(detail) => {
                return Ok(Round {
                    leaves,
                    stop: Some(RefineOutcome::Stopped {
                        slice: name.to_string(),
                        detail,
                    }),
                });
            }
        };
        if Proposal::boundary_for(layout, name)?.is_some() {
            return Ok(Round {
                leaves,
                stop: Some(RefineOutcome::Stopped {
                    slice: name.to_string(),
                    detail: format!(
                        "inert boundary proposal already parks `{name}`; apply it with emery \
                         plan amend --proposal before re-refining this leaf"
                    ),
                }),
            });
        }
        // The recorded `metadata.yaml` target is authoritative once the
        // slice exists; only absence falls through to a fresh resolve.
        let target = match SliceMetadata::load_opt(&layout.slice_dir(name))? {
            Some(meta) => meta.target,
            None => project::target_policy::fresh(resolver, paths, entry, name, "refining")?,
        };
        let adapter = leaf_adapter(resolver, paths, entry)?;
        leaves.push(RoundLeaf {
            name: name.to_string(),
            target,
            dependencies,
            adapter,
        });
    }
    Ok(Round { leaves, stop: None })
}

/// Fold one pool outcome into the drain's per-leaf surface, in
/// topological order.
fn settle_leaf(
    outcome: pool::Outcome<slice::orchestrate::RefineOutcome, Error>, name: &str,
) -> Result<slice::orchestrate::RefineOutcome, Error> {
    match outcome {
        pool::Outcome::Settled(result) => result,
        pool::Outcome::TimedOut => Err(Error::Diag {
            code: "plan-refine-timeout",
            detail: format!(
                "refinement of `{name}` exceeded its inactivity budget; re-run the drain"
            ),
        }),
        pool::Outcome::Rejected | pool::Outcome::Cancelled | pool::Outcome::Skipped => {
            Err(Error::Diag {
                code: "plan-refine-cancelled",
                detail: format!(
                    "refinement of `{name}` did not run (a sibling refinement failed first)"
                ),
            })
        }
    }
}

async fn persist_escalation<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, layout: Layout<'_>,
    name: &str, escalation: slice::orchestrate::RefineOutcome,
) -> Result<RefineOutcome, Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    tracing::info!("refine {name} — boundary-escalation");
    match super::escalate::persist(provider, paths, now, plan, escalation).await {
        Ok(digest) => Ok(RefineOutcome::Stopped {
            slice: name.to_string(),
            detail: format!(
                "boundary-escalation wrote inert proposal `{digest}`; planning artifacts unchanged"
            ),
        }),
        Err(err) => {
            if matches!(
                &err,
                Error::Validation { code, .. }
                    if code.as_ref() == "plan-refine-budget-exhausted"
            ) {
                journal_budget_park(layout, now, name)?;
            }
            tracing::info!("refine {name} — stopped: {err}");
            Ok(RefineOutcome::Stopped {
                slice: name.to_string(),
                detail: err.to_string(),
            })
        }
    }
}

fn journal_budget_park(layout: Layout<'_>, now: Timestamp, slice: &str) -> Result<(), Error> {
    journal::append_one(
        layout,
        &Event::new(
            now,
            EventKind::SliceRefinementParked {
                slice_name: slice.into(),
                reason: ParkReason::BudgetExhausted,
                proposal: None,
            },
        ),
    )
}

/// Ordered `(slice, refinement-digest)` pins over `entry.depends_on`.
/// Returns `Err(detail)` (a typed stop, not a hard error) when a
/// direct predecessor's manifest is not currently fresh. A predecessor
/// whose slice tree was archived by merge or `plan drop` pins its
/// archived manifest digest — an accepted predecessor satisfies
/// "predecessor refined" a fortiori (RFC-91 D3).
fn predecessor_pins(
    layout: Layout<'_>, plan: &Plan, entry: &Entry, inventory: &[Lead], live: &mut Live,
) -> Result<Result<Vec<Dependency>, String>, Error> {
    let mut dependencies = Vec::with_capacity(entry.depends_on.len());
    for dep in &entry.depends_on {
        let Some(dep_entry) = plan.entries.iter().find(|e| e.name == *dep) else {
            return Ok(Err(format!(
                "predecessor `{dep}` has no plan entry; fix the entry's depends-on list"
            )));
        };
        match refinement::freshness_with(layout, plan, dep_entry, inventory, live)? {
            Freshness::Fresh { digest } => dependencies.push(Dependency {
                slice: dep.as_str().to_string(),
                refinement: digest,
            }),
            Freshness::Missing => {
                // The live tree has no manifest: an accepted (merged or
                // dropped) predecessor's archived manifest is the pin.
                match refinement::predecessor_digest(layout, dep.as_str())? {
                    Some(digest) => dependencies.push(Dependency {
                        slice: dep.as_str().to_string(),
                        refinement: digest,
                    }),
                    None => {
                        return Ok(Err(format!(
                            "predecessor `{dep}` has no refinement manifest (live or archived) — \
                             dependent refinement requires predecessor refined"
                        )));
                    }
                }
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
            in_scope(plan, entry, meta.as_ref(), &events)
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
    live: &mut Live,
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
            refinement::freshness_with(layout, plan, entry, inventory, live)?,
            Freshness::Fresh { .. }
        );
        if dep_included || !fresh {
            targets.insert(name.to_string());
        }
    }
    Ok(targets)
}

fn leaf_adapter(
    resolver: &impl Resolver, paths: &ExecutionPaths, entry: &Entry,
) -> Result<project::adapter::ResolvedTarget, Error> {
    let layout = paths.layout();
    let plan = Plan::load(&layout.plan_path())?;
    let binding = plan.target(&entry.target)?;
    resolver.resolve_target(&binding.adapter.selector(), paths)
}
