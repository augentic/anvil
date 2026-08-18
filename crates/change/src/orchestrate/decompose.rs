//! Engine-owned decomposition: one bounded judgment per open domain.
//! The author path persists after every apply and disposes failed cuts
//! (close-as-leaf or park); the candidate path stays fail-fast.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use artifacts::leads::Leads;
use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::Resolver;
use project::handler::ExecutionPaths;
use project::journal::{self, ClosedReason, Event, EventKind};
use project::name::is_kebab;
use project::plan::Plan;
use project::plan::decomposition::{
    BoundProfile, Child, Decomposition, Kind, MAX_JUDGMENTS, Node, PARTITION_VERSION,
    PartitionKind, PartitionResponse, ReviewVerdict, Scope, VERSION,
};
use project::pool;
use project::profile::{Gate, Profiles};
use project::seam::{Source, Workspaces};
use serde_json::json;

use crate::judgment::{partition, review};
use crate::orchestrate::survey::{focused_leads, survey};

/// One domain parked by a failed-cut disposition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParkedDomain {
    /// Decomposition node id.
    pub domain: String,
    /// The failed-cut finding detail.
    pub reason: String,
}

/// Result of a completed decomposition loop.
#[derive(Debug, Clone)]
pub struct Decomposed {
    /// Validated hierarchy (complete when [`Self::parked`] is empty,
    /// partial otherwise).
    pub tree: Decomposition,
    /// Close-with-rationale and estimate-caveat lines for `change.md`.
    pub caveats: Vec<String>,
    /// Domains parked by failed-cut dispositions, in park order.
    /// Always empty on the candidate (`!persist`) path.
    pub parked: Vec<ParkedDomain>,
}

/// Live loop state: the tree under construction plus the open-domain
/// queue and the disposition ledger.
struct State {
    tree: Decomposition,
    queue: VecDeque<String>,
    caveats: Vec<String>,
    parked: Vec<ParkedDomain>,
    judgments: usize,
}

/// Recursively partition the bound catalog into a complete tree.
///
/// # Errors
///
/// Budget exhaustion, definition-revision stops, invalid responses
/// after repair on the candidate path, and tree-validation failures.
pub async fn decompose<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, leads: &Leads,
) -> Result<Decomposed, Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let mut catalog = leads.clone();
    let tree = seed(plan, &catalog, provider.profiles())?;
    let queue = VecDeque::from([tree.root.clone()]);
    run(provider, paths, now, plan, &mut catalog, tree, queue, true).await
}

/// Resume authoring over a persisted partial tree: rebuild the queue
/// from open domains (`kind` unset) plus `parked` domains named by
/// park facts, then continue the drain.
///
/// # Errors
///
/// Load/parse failures and the same loop failures as [`decompose`].
pub async fn resume<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, leads: &Leads,
    parked: &[String],
) -> Result<Decomposed, Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let mut catalog = leads.clone();
    let path = paths.layout().decomposition_path();
    let (tree, queue) = if let Some(mut tree) = Decomposition::load_opt(&path)? {
        let mut queue = open_domains(&tree);
        // A final-check park names a closed node — reopen it so the
        // resumed drain re-judges that cut.
        for domain in parked {
            if tree.nodes.get(domain).is_some_and(|node| node.kind.is_some()) {
                reopen_node(&mut tree, domain, &catalog)?;
                if !queue.contains(domain) {
                    queue.push_back(domain.clone());
                }
            }
        }
        (tree, queue)
    } else {
        let tree = seed(plan, &catalog, provider.profiles())?;
        let queue = VecDeque::from([tree.root.clone()]);
        (tree, queue)
    };
    run(provider, paths, now, plan, &mut catalog, tree, queue, true).await
}

/// Open domains (no cut yet), shallowest first, id-ordered within a
/// depth — a deterministic resume queue.
fn open_domains(tree: &Decomposition) -> VecDeque<String> {
    let mut open: Vec<String> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.kind.is_none())
        .map(|(id, _)| id.clone())
        .collect();
    open.sort_by_key(|id| (tree.depth(id).unwrap_or(usize::MAX), id.clone()));
    open.into()
}

/// Re-decompose the nearest domain of `leaf` against `catalog` without
/// writing `leads.md` or `decomposition.yaml`.
///
/// # Errors
///
/// Missing decomposition, unknown leaf, and the same loop failures as
/// [`decompose`].
pub async fn nearest<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, leaf: &str,
    catalog: &mut Leads,
) -> Result<Decomposed, Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let mut tree = Decomposition::load(&paths.layout().decomposition_path())?;
    let start = reopen(&mut tree, leaf, catalog)?;
    tree.leads_digest = project::snapshot::SnapshotId::from_digest(&catalog.digest_hex()?);
    let queue = VecDeque::from([start]);
    run(provider, paths, now, plan, catalog, tree, queue, false).await
}

#[expect(
    clippy::too_many_arguments,
    reason = "the loop carries the live tree, catalog, and persist mode"
)]
async fn run<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, catalog: &mut Leads,
    tree: Decomposition, queue: VecDeque<String>, persist: bool,
) -> Result<Decomposed, Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let mut state = State {
        tree,
        queue,
        caveats: Vec::new(),
        parked: Vec::new(),
        judgments: 0,
    };
    // Final-check findings inlined into the one reopen re-judgment.
    let mut notes: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut final_rejudged = false;

    loop {
        drain(provider, paths, now, plan, catalog, &mut state, persist, &notes).await?;
        if !state.parked.is_empty() {
            // A parked domain leaves the tree partial by design — the
            // caller publishes closed leaves and stops for the operator.
            return Ok(done(state));
        }
        match state.tree.check() {
            Ok(()) => return Ok(done(state)),
            Err(err) if !persist => return Err(err),
            Err(err) => {
                // With deferral a violation can first surface on the
                // complete tree: reopen the offending parent, re-judge
                // once with findings inlined, else park — never abort.
                let findings = project::plan::decomposition::findings(&state.tree);
                let offender = offending_domain(&state.tree, &findings);
                if final_rejudged {
                    park(paths, now, &mut state, &offender, err.to_string())?;
                    return Ok(done(state));
                }
                final_rejudged = true;
                notes.insert(
                    offender.clone(),
                    findings.iter().map(|item| item.impact.clone()).collect(),
                );
                reopen_node(&mut state.tree, &offender, catalog)?;
                save_tree(paths, &state.tree)?;
                state.queue.push_back(offender);
            }
        }
    }
}

fn done(state: State) -> Decomposed {
    Decomposed {
        tree: state.tree,
        caveats: state.caveats,
        parked: state.parked,
    }
}

/// The cut to redo when the complete tree fails validation: the parent
/// of the first finding's node (the finding names the violating child
/// or leaf), else the root.
fn offending_domain(tree: &Decomposition, findings: &[diagnostics::Diagnostic]) -> String {
    findings
        .first()
        .and_then(|item| item.slice.as_deref())
        .and_then(|id| tree.nodes.get(id))
        .and_then(|node| node.parent.clone())
        .unwrap_or_else(|| tree.root.clone())
}

#[expect(
    clippy::too_many_arguments,
    reason = "the round driver carries the live loop state and persist mode"
)]
async fn drain<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, catalog: &mut Leads,
    state: &mut State, persist: bool, notes: &BTreeMap<String, Vec<String>>,
) -> Result<(), Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let claims = pool::Claims::default();

    // Sibling domains own distinct subtrees, so their partition
    // judgments run concurrently in rounds (RFC-96 D5); responses join
    // and apply in queue order — never completion order.
    while !state.queue.is_empty() {
        let remaining = MAX_JUDGMENTS.saturating_sub(state.judgments);
        if remaining == 0 {
            return Err(budget_exhausted());
        }
        let take = state.queue.len().min(remaining);
        let round: Vec<String> = state.queue.drain(..take).collect();
        let requests = round_requests(state, catalog, plan, &round, notes)?;
        state.judgments += round.len();

        // Each job records its last parsed answer so a failed cut can
        // still gate the deterministic close-as-leaf fallback on the
        // answer's assessment.
        let last_answers: Vec<Mutex<Option<PartitionResponse>>> =
            round.iter().map(|_| Mutex::new(None)).collect();
        let jobs = round_jobs(provider, plan, &state.tree, &round, &requests, &last_answers);
        let outcomes = pool::run(pool::cap(), &claims, pool::OnFailure::Drain, jobs).await;

        for ((id, outcome), slot) in round.iter().zip(outcomes).zip(&last_answers) {
            // A sibling's disposition may have closed or pruned this
            // domain (parent fallback subsumes its children).
            if state.tree.nodes.get(id.as_str()).is_none_or(|node| node.kind.is_some()) {
                continue;
            }
            let response = match settle_partition(outcome, id) {
                Ok(response) => response,
                Err(err) if persist => {
                    // A never-run sibling is not a failed cut: the
                    // domain stays open in the persisted tree — park
                    // it so re-entry resumes it instead of aborting.
                    if never_ran(&err) {
                        park(paths, now, state, id, err.to_string())?;
                        continue;
                    }
                    let last = slot.lock().ok().and_then(|mut slot| slot.take());
                    dispose(provider, paths, now, plan, catalog, state, id, err, last).await?;
                    continue;
                }
                Err(err) => return Err(err),
            };
            check_targets(plan, &response)?;
            // Join-order re-validation: siblings validated against the
            // round-start clone, so an earlier join can invalidate this
            // one. Not a model error — route the disposition ladder.
            let mut trial = state.tree.clone();
            if let Err(err) = apply(&mut trial, id, &response).and_then(|()| check_progress(&trial))
            {
                if persist {
                    dispose(provider, paths, now, plan, catalog, state, id, err, Some(response))
                        .await?;
                    continue;
                }
                return Err(err);
            }
            match response.kind {
                PartitionKind::Split => {
                    let children: Vec<String> =
                        response.children.iter().map(|child| child.id.clone()).collect();
                    apply(&mut state.tree, id, &response)?;
                    tracing::info!("partition {id} — split {}", children.len());
                    if persist {
                        save_tree(paths, &state.tree)?;
                    }
                    for child in children {
                        state.queue.push_back(child);
                    }
                }
                PartitionKind::Leaf => {
                    match close_or_review(
                        provider, paths, now, plan, catalog, persist, state, id, &response,
                    )
                    .await
                    {
                        Ok(Settled::Closed) => {
                            tracing::info!("partition {id} — leaf");
                            if persist {
                                save_tree(paths, &state.tree)?;
                            }
                        }
                        Ok(Settled::Requeued) => tracing::info!("partition {id} — requeued"),
                        Err(err)
                            if persist && err.variant_str().as_ref() == "plan-author-unready" =>
                        {
                            park(paths, now, state, id, err.to_string())?;
                        }
                        Err(err) => return Err(err),
                    }
                }
            }
        }
    }
    Ok(())
}

/// One pool job per round member: the partition judgment with the
/// tentative-tree tail, each announced as `partition {id} …`.
fn round_jobs<'a, P: Model>(
    provider: &'a P, plan: &'a Plan, tree: &'a Decomposition, round: &'a [String],
    requests: &'a [serde_json::Value], last_answers: &'a [Mutex<Option<PartitionResponse>>],
) -> Vec<pool::Job<'a, PartitionResponse, Error>> {
    round
        .iter()
        .zip(requests)
        .zip(last_answers)
        .map(|((id, request), slot)| {
            tracing::info!("partition {id} …");
            pool::Job {
                claim: pool::Claim {
                    item: id.clone(),
                    operation: "partition".to_string(),
                    attempt: 1,
                },
                budget: pool::budget::PARTITION,
                future: Box::pin(partition::partition(provider, request, move |answer| {
                    if let Ok(mut last) = slot.lock() {
                        *last = Some(answer.clone());
                    }
                    check_targets(plan, answer)?;
                    let mut trial = tree.clone();
                    apply(&mut trial, id, answer)?;
                    check_progress(&trial)
                })),
            }
        })
        .collect()
}

/// One partition request per round member, numbered in queue order.
fn round_requests(
    state: &State, catalog: &Leads, plan: &Plan, round: &[String],
    notes: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<serde_json::Value>, Error> {
    round
        .iter()
        .enumerate()
        .map(|(offset, id)| {
            partition_request(&state.tree, id, catalog, plan, state.judgments + offset + 1, notes)
        })
        .collect()
}

/// Dispose one failed cut after the repair budget (author path only):
/// close the domain as a leaf through the profile gate when it
/// satisfies leaf shape, else park that domain and keep draining.
/// Fatal classes (definition revision, target escape, unparseable
/// envelope, …) propagate unchanged.
#[expect(
    clippy::too_many_arguments,
    reason = "the disposition ladder shares the drain's loop state"
)]
async fn dispose<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, catalog: &mut Leads,
    state: &mut State, id: &str, err: Error, last: Option<PartitionResponse>,
) -> Result<(), Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let Some(code) = failed_cut_code(&err) else {
        return Err(err);
    };
    let reason = if code == "decomposition-non-reducing" {
        ClosedReason::NonReducingFallback
    } else {
        ClosedReason::RepairExhausted
    };
    let detail = err.to_string();

    if let Some(response) = fallback_leaf(&state.tree, id, last.as_ref()) {
        let mut trial = state.tree.clone();
        let fits = apply(&mut trial, id, &response).and_then(|()| check_progress(&trial)).is_ok();
        if fits {
            match close_or_review(provider, paths, now, plan, catalog, true, state, id, &response)
                .await
            {
                Ok(Settled::Closed) => {
                    journal::append_one(
                        paths.layout(),
                        &Event::new(
                            now,
                            EventKind::DomainPartitionClosed {
                                domain: id.to_string(),
                                reason,
                                findings: vec![detail.clone()],
                            },
                        ),
                    )?;
                    state
                        .caveats
                        .push(format!("`{id}` closed as a leaf after a failed cut: {detail}"));
                    save_tree(paths, &state.tree)?;
                    return Ok(());
                }
                // The review widened the catalog and requeued the
                // domain — a fresh judgment gets the new detail.
                Ok(Settled::Requeued) => return Ok(()),
                Err(gate) if gate.variant_str().as_ref() == "plan-author-unready" => {}
                Err(gate) => return Err(gate),
            }
        }
    }
    if reason == ClosedReason::NonReducingFallback
        && parent_fallback(provider, paths, now, plan, catalog, state, id, last.as_ref(), &detail)
            .await?
    {
        return Ok(());
    }
    park(paths, now, state, id, detail)
}

/// A non-reducing tie against the parent means the parent's cut cannot
/// reduce — the parent's whole scope is one slice. Prune the cut and
/// close the parent as a leaf through the same profile gate. Returns
/// `true` when the disposition settled (closed, requeued, or parked at
/// the parent).
#[expect(
    clippy::too_many_arguments,
    reason = "the disposition ladder shares the drain's loop state"
)]
async fn parent_fallback<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, catalog: &mut Leads,
    state: &mut State, id: &str, last: Option<&PartitionResponse>, detail: &str,
) -> Result<bool, Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let Some(parent) = state.tree.nodes.get(id).and_then(|node| node.parent.clone()) else {
        return Ok(false);
    };
    // Build the candidate before reopening: the closed split node still
    // carries the ownership and sources the leaf needs.
    let Some(response) = fallback_leaf(&state.tree, &parent, last) else {
        return Ok(false);
    };
    let mut trial = state.tree.clone();
    reopen_node(&mut trial, &parent, catalog)?;
    let fits = apply(&mut trial, &parent, &response).and_then(|()| check_progress(&trial)).is_ok();
    if !fits {
        return Ok(false);
    }
    reopen_node(&mut state.tree, &parent, catalog)?;
    // Subsumed children may still sit in the queue.
    state
        .queue
        .retain(|queued| state.tree.nodes.get(queued).is_some_and(|node| node.kind.is_none()));
    match close_or_review(provider, paths, now, plan, catalog, true, state, &parent, &response)
        .await
    {
        Ok(Settled::Closed) => {
            journal::append_one(
                paths.layout(),
                &Event::new(
                    now,
                    EventKind::DomainPartitionClosed {
                        domain: parent.clone(),
                        reason: ClosedReason::NonReducingFallback,
                        findings: vec![detail.to_string()],
                    },
                ),
            )?;
            state.caveats.push(format!(
                "`{parent}` closed as a leaf after a failed cut of `{id}`: {detail}"
            ));
            save_tree(paths, &state.tree)?;
            Ok(true)
        }
        Ok(Settled::Requeued) => Ok(true),
        Err(gate) if gate.variant_str().as_ref() == "plan-author-unready" => {
            park(paths, now, state, &parent, detail.to_string())?;
            Ok(true)
        }
        Err(gate) => Err(gate),
    }
}

/// A partition that never produced a judgment: skipped after a
/// sibling failure or dropped on its inactivity budget.
fn never_ran(err: &Error) -> bool {
    matches!(
        err,
        Error::Diag { code, .. }
            if *code == "plan-author-partition-cancelled"
                || *code == "plan-author-partition-timeout"
    )
}

/// Closed failed-cut class: the cut is illegal but the domain is
/// recoverable. Everything else stays fatal.
fn failed_cut_code(err: &Error) -> Option<&str> {
    let code: &str = match err {
        Error::Validation { code, .. } => code.as_ref(),
        Error::Diag { code, .. } => code,
        _ => return None,
    };
    matches!(
        code,
        "decomposition-non-reducing"
            | "decomposition-lead-uncovered"
            | "decomposition-lead-dropped"
            | "decomposition-overlap"
    )
    .then_some(code)
}

/// The deterministic close-as-leaf candidate: the domain's own scope
/// as a terminal leaf. `None` when the domain cannot satisfy leaf
/// shape (multiple targets, no ownership, a repeated source, or no
/// parsed assessment to gate on).
fn fallback_leaf(
    tree: &Decomposition, id: &str, last: Option<&PartitionResponse>,
) -> Option<PartitionResponse> {
    let node = tree.nodes.get(id)?;
    let assessment = last.map(|response| response.assessment)?;
    if !is_kebab(id) || node.ownership.is_empty() {
        return None;
    }
    let targets = node.target_set();
    if targets.len() != 1 {
        return None;
    }
    let mut seen = std::collections::BTreeSet::new();
    for scope in &node.sources {
        if !seen.insert(scope.source.as_str()) {
            return None;
        }
    }
    Some(PartitionResponse {
        version: PARTITION_VERSION,
        kind: PartitionKind::Leaf,
        children: Vec::new(),
        target: targets.into_iter().next().map(str::to_string),
        slice: Some(id.to_string()),
        ownership: node.ownership.clone(),
        acceptance: Some(format!("`{id}` delivers its bound leads as one reviewable unit.")),
        sources: node.sources.clone(),
        depends_on: node.depends_on.clone(),
        rationale: Some("engine fallback: closed as a leaf after a failed cut".into()),
        assessment,
    })
}

/// Park `id`: journal the fact and record it on the loop state. The
/// node stays open in the persisted partial tree — no proposal exists;
/// the resume path is `plan author` re-entry.
fn park(
    paths: &ExecutionPaths, now: Timestamp, state: &mut State, id: &str, reason: String,
) -> Result<(), Error> {
    tracing::info!("partition {id} — parked");
    journal::append_one(
        paths.layout(),
        &Event::new(
            now,
            EventKind::PlanAuthorParked {
                domain: id.to_string(),
                reason: reason.clone(),
            },
        ),
    )?;
    state.parked.push(ParkedDomain {
        domain: id.to_string(),
        reason,
    });
    save_tree(paths, &state.tree)
}

fn save_tree(paths: &ExecutionPaths, tree: &Decomposition) -> Result<(), Error> {
    tree.save(&paths.layout().decomposition_path())
}

/// Fold one pool outcome into the decomposition error surface, in
/// queue order.
fn settle_partition(
    outcome: pool::Outcome<PartitionResponse, Error>, id: &str,
) -> Result<PartitionResponse, Error> {
    match outcome {
        pool::Outcome::Settled(result) => result,
        pool::Outcome::TimedOut => Err(Error::Diag {
            code: "plan-author-partition-timeout",
            detail: format!("partition of `{id}` exceeded its inactivity budget; re-run"),
        }),
        pool::Outcome::Rejected | pool::Outcome::Cancelled | pool::Outcome::Skipped => {
            Err(Error::Diag {
                code: "plan-author-partition-cancelled",
                detail: format!(
                    "partition of `{id}` did not run (a sibling judgment failed first)"
                ),
            })
        }
    }
}

fn budget_exhausted() -> Error {
    Error::validation_failed(
        "plan-author-budget-exhausted",
        "decomposition stays within the compiled judgment budget",
        format!("decomposition parked after {MAX_JUDGMENTS} judgments"),
    )
}

fn reopen(tree: &mut Decomposition, leaf: &str, catalog: &Leads) -> Result<String, Error> {
    let leaf_id = tree
        .nodes
        .iter()
        .find(|(_, node)| node.slice.as_ref().is_some_and(|name| name.as_str() == leaf))
        .map(|(id, _)| id.clone())
        .ok_or_else(|| Error::Diag {
            code: "decomposition-node-unknown",
            detail: format!("no decomposition leaf `{leaf}`"),
        })?;
    let nearest = tree
        .nodes
        .get(&leaf_id)
        .and_then(|node| node.parent.clone())
        .unwrap_or_else(|| tree.root.clone());
    reopen_node(tree, &nearest, catalog)?;
    Ok(nearest)
}

/// Reset `id` to an open domain: prune its subtree, clear its cut, and
/// re-close its sources over the catalog.
fn reopen_node(tree: &mut Decomposition, id: &str, catalog: &Leads) -> Result<(), Error> {
    prune_descendants(tree, id);
    let owned = tree.node(id)?.sources.clone();
    let sources = close_sources(&owned, catalog);
    let node = tree.nodes.get_mut(id).ok_or_else(|| Error::Diag {
        code: "decomposition-node-unknown",
        detail: format!("no decomposition node `{id}`"),
    })?;
    node.children.clear();
    node.kind = None;
    node.slice = None;
    node.acceptance = None;
    node.ownership.clear();
    node.sources = sources;
    Ok(())
}

fn prune_descendants(tree: &mut Decomposition, id: &str) {
    let children = tree.nodes.get(id).map(|node| node.children.clone()).unwrap_or_default();
    for child in children {
        prune_descendants(tree, &child);
        tree.nodes.remove(&child);
    }
}

fn close_sources(owned: &[Scope], catalog: &Leads) -> Vec<Scope> {
    let mut keep: std::collections::BTreeSet<(String, String)> =
        owned.iter().map(|scope| (scope.source.clone(), scope.lead.clone())).collect();
    let mut grew = true;
    while grew {
        grew = false;
        for lead in catalog.leads() {
            let pair = (lead.source.clone(), lead.lead.clone());
            if keep.contains(&pair) {
                continue;
            }
            let parent = lead.parent.as_ref().or(lead.focus.as_ref());
            if parent.is_some_and(|parent| keep.contains(&(lead.source.clone(), parent.clone()))) {
                keep.insert(pair);
                grew = true;
            }
        }
    }
    catalog
        .leads()
        .iter()
        .filter(|lead| keep.contains(&(lead.source.clone(), lead.lead.clone())))
        .map(|lead| Scope::new(&lead.source, &lead.lead))
        .collect()
}

/// How [`close_or_review`] settled the leaf.
enum Settled {
    /// The leaf was applied to the live tree.
    Closed,
    /// A focus verdict widened the catalog and requeued the domain.
    Requeued,
}

#[expect(
    clippy::too_many_arguments,
    reason = "leaf-readiness carries the live loop state and persist mode"
)]
async fn close_or_review<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, catalog: &mut Leads,
    persist: bool, state: &mut State, id: &str, response: &PartitionResponse,
) -> Result<Settled, Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let profile = leaf_profile(provider, plan, response, state.tree.node(id)?)?;
    if !profile.exceeds(&response.assessment, Gate::SliceSplit)? {
        apply(&mut state.tree, id, response)?;
        return Ok(Settled::Closed);
    }
    if state.judgments >= MAX_JUDGMENTS {
        return Err(budget_exhausted());
    }
    state.judgments += 1;
    let request = review_request(&state.tree, id, response, profile, catalog)?;
    let review = review::review(provider, &request, |answer| check_focus(catalog, answer)).await?;
    match review.verdict {
        ReviewVerdict::Close => {
            apply(&mut state.tree, id, response)?;
            if let Some(rationale) = review.rationale.or_else(|| response.rationale.clone()) {
                state
                    .caveats
                    .push(format!("`{id}` closed above the slice-split threshold: {rationale}"));
            }
            Ok(Settled::Closed)
        }
        ReviewVerdict::Focus => {
            if review.focus.is_empty() {
                return Err(Error::validation_failed(
                    "plan-author-focus-empty",
                    "a focus verdict names at least one catalog parent",
                    format!("boundary review of `{id}` named no parents"),
                ));
            }
            for parent in &review.focus {
                focus_parent(provider, paths, now, plan, catalog, persist, parent).await?;
            }
            state.queue.push_front(id.to_string());
            Ok(Settled::Requeued)
        }
        ReviewVerdict::Unready => Err(Error::validation_failed(
            "plan-author-unready",
            "an over-envelope leaf that cannot split blocks authoring",
            review.rationale.unwrap_or_else(|| {
                format!("domain `{id}` exceeds the target envelope and cannot split")
            }),
        )),
    }
}

async fn focus_parent<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, catalog: &mut Leads,
    persist: bool, parent: &project::plan::FocusParent,
) -> Result<(), Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let Some(binding) = plan.sources.get(&parent.source) else {
        return Err(Error::Diag {
            code: "plan-source-unknown",
            detail: format!("focus source `{}` is not a plan binding", parent.source),
        });
    };
    if persist {
        if binding.locator.is_some() {
            survey(
                provider,
                provider,
                provider,
                paths,
                now,
                &parent.source,
                None,
                Some(parent.lead.as_str()),
            )
            .await?;
            *catalog = Leads::load(&paths.layout().leads_path())?;
        }
        return Ok(());
    }
    if binding.locator.is_some() {
        let children = focused_leads(
            provider,
            provider,
            provider,
            paths,
            now,
            &parent.source,
            binding,
            catalog,
            &parent.lead,
        )
        .await?;
        catalog.merge_leads(&parent.source, children);
    }
    Ok(())
}

fn seed(
    plan: &Plan, leads: &Leads, table: &project::profile::Table,
) -> Result<Decomposition, Error> {
    let profiles = bound_profiles(plan, table)?;
    let sources: Vec<Scope> =
        leads.leads().iter().map(|lead| Scope::new(&lead.source, &lead.lead)).collect();
    if sources.is_empty() {
        return Err(Error::validation_failed(
            "plan-reconcile-empty-catalog",
            "decomposition requires at least one catalog lead",
            "leads.md carries no leads",
        ));
    }
    let targets: Vec<String> = plan.targets.keys().cloned().collect();
    let mut root = Node::split(Vec::new());
    root.sources = sources;
    root.targets = targets;
    root.kind = None;
    Ok(Decomposition {
        version: VERSION,
        leads_digest: plan.leads_digest.clone().ok_or_else(|| Error::Diag {
            code: "plan-leads-digest-missing",
            detail: "plan.yaml has no leads-digest".into(),
        })?,
        profiles,
        root: "root".into(),
        nodes: BTreeMap::from([("root".into(), root)]),
    })
}

fn bound_profiles(
    plan: &Plan, table: &project::profile::Table,
) -> Result<BTreeMap<String, BoundProfile>, Error> {
    let mut out = BTreeMap::new();
    for (id, row) in &plan.targets {
        let pin = row.model_capability_profile.as_ref().ok_or_else(|| Error::Diag {
            code: "plan-profile-missing",
            detail: format!("target `{id}` has no model-capability-profile pin"),
        })?;
        out.insert(id.clone(), BoundProfile::capture(table.pinned(pin)?)?);
    }
    Ok(out)
}

fn leaf_profile<'a, P: Profiles>(
    provider: &'a P, plan: &Plan, response: &PartitionResponse, node: &Node,
) -> Result<&'a project::profile::Profile, Error> {
    let target = response
        .target
        .as_deref()
        .or(node.target.as_deref())
        .or_else(|| node.targets.first().map(String::as_str))
        .or_else(|| plan.targets.keys().next().map(String::as_str))
        .ok_or_else(|| Error::Diag {
            code: "plan-author-definition-revision",
            detail: "leaf names no target and the plan has none".into(),
        })?;
    let pin =
        plan.targets.get(target).and_then(|row| row.model_capability_profile.as_ref()).ok_or_else(
            || {
                Error::validation_failed(
                    "plan-author-definition-revision",
                    "a leaf binds a target from the reviewed wave",
                    format!("leaf target `{target}` is not in the reviewed wave"),
                )
            },
        )?;
    provider.profiles().pinned(pin)
}

/// Validate an in-progress cut over **known data only**. Open children
/// are not yet leaves and have not declared ownership, so their
/// `decomposition-leaf-incomplete` and `decomposition-non-reducing`
/// findings stay deferred until the domain is partitioned. Claimed
/// leaves and every other split rule still fail the repair loop; the
/// full rule set is enforced at `tree.check()` on the complete tree.
fn check_progress(tree: &Decomposition) -> Result<(), Error> {
    let findings: Vec<_> = project::plan::decomposition::findings(tree)
        .into_iter()
        .filter(|item| keep_progress(tree, item))
        .collect();
    let Some(first) = findings.first() else {
        return Ok(());
    };
    let detail = findings.iter().map(|item| item.impact.clone()).collect::<Vec<_>>().join("; ");
    Err(Error::Validation {
        code: first.rule_id.clone().unwrap_or_default().into(),
        detail,
    })
}

fn keep_progress(tree: &Decomposition, item: &diagnostics::Diagnostic) -> bool {
    let named = |id: Option<&str>| id.and_then(|id| tree.nodes.get(id));
    match item.rule_id.as_deref() {
        Some("decomposition-leaf-incomplete") => {
            named(item.slice.as_deref()).is_some_and(|node| node.kind == Some(Kind::Leaf))
        }
        // An open child inherits the parent's full scope until it is
        // partitioned, so a `Measure` tie against the parent is
        // structural, not a bad split.
        Some("decomposition-non-reducing") => {
            named(item.slice.as_deref()).is_none_or(|node| node.kind.is_some())
        }
        _ => true,
    }
}

fn check_targets(plan: &Plan, response: &PartitionResponse) -> Result<(), Error> {
    let mut named = Vec::new();
    if let Some(target) = &response.target {
        named.push(target.as_str());
    }
    for child in &response.children {
        if let Some(target) = &child.target {
            named.push(target.as_str());
        }
        for target in &child.targets {
            named.push(target.as_str());
        }
    }
    for target in named {
        if !plan.targets.contains_key(target) {
            return Err(Error::validation_failed(
                "plan-author-definition-revision",
                "a partition binds only a target from the reviewed wave",
                format!("target `{target}` is not in the reviewed wave"),
            ));
        }
    }
    Ok(())
}

fn check_focus(catalog: &Leads, review: &project::plan::BoundaryReview) -> Result<(), Error> {
    for parent in &review.focus {
        if !catalog
            .leads()
            .iter()
            .any(|lead| lead.source == parent.source && lead.lead == parent.lead)
        {
            return Err(Error::validation_failed(
                "leads-lead-unknown",
                "a focus parent names a catalog row",
                format!("no lead `{}` for source `{}` in leads.md", parent.lead, parent.source),
            ));
        }
    }
    Ok(())
}

fn apply(tree: &mut Decomposition, id: &str, response: &PartitionResponse) -> Result<(), Error> {
    match response.kind {
        PartitionKind::Split => apply_split(tree, id, response),
        PartitionKind::Leaf => apply_leaf(tree, id, response),
    }
}

fn apply_split(
    tree: &mut Decomposition, id: &str, response: &PartitionResponse,
) -> Result<(), Error> {
    if response.children.is_empty() {
        return Err(Error::validation_failed(
            "decomposition-kind",
            "a split names at least one child",
            format!("split of `{id}` named no children"),
        ));
    }
    let parent_sources = tree.node(id)?.sources.clone();
    let parent_targets =
        tree.node(id)?.target_set().into_iter().map(str::to_string).collect::<Vec<_>>();
    let mut child_ids = Vec::with_capacity(response.children.len());
    for child in &response.children {
        if !is_kebab(&child.id) {
            return Err(Error::validation_failed(
                "decomposition-node-unknown",
                "a child id must be kebab-case",
                format!("child id `{}` is not kebab-case", child.id),
            ));
        }
        child_ids.push(child.id.clone());
        tree.nodes
            .insert(child.id.clone(), child_node(child, id, &parent_sources, &parent_targets));
    }
    let parent = tree.nodes.get_mut(id).ok_or_else(|| Error::Diag {
        code: "decomposition-node-unknown",
        detail: format!("no decomposition node `{id}`"),
    })?;
    parent.children = child_ids;
    parent.kind = Some(Kind::Split);
    parent.slice = None;
    parent.acceptance = None;
    Ok(())
}

fn child_node(
    child: &Child, parent: &str, fallback_sources: &[Scope], fallback_targets: &[String],
) -> Node {
    let sources =
        if child.sources.is_empty() { fallback_sources.to_vec() } else { child.sources.clone() };
    let mut node = Node {
        parent: Some(parent.into()),
        sources,
        target: child.target.clone(),
        targets: child.targets.clone(),
        ownership: child.ownership.clone(),
        depends_on: child.depends_on.clone(),
        kind: None,
        ..Node::default()
    };
    if node.target.is_none() && node.targets.is_empty() {
        if fallback_targets.len() == 1 {
            node.target = fallback_targets.first().cloned();
        } else {
            node.targets = fallback_targets.to_vec();
        }
    }
    node
}

fn apply_leaf(
    tree: &mut Decomposition, id: &str, response: &PartitionResponse,
) -> Result<(), Error> {
    let slice = response.slice.clone().filter(|name| is_kebab(name)).ok_or_else(|| {
        Error::validation_failed(
            "decomposition-leaf-incomplete",
            "a leaf names a kebab-case slice",
            format!("leaf `{id}` has no kebab-case slice mapping"),
        )
    })?;
    if id == tree.root {
        let child_id = slice.clone();
        let parent_sources = tree.node(id)?.sources.clone();
        let parent_targets =
            tree.node(id)?.target_set().into_iter().map(str::to_string).collect::<Vec<_>>();
        let mut leaf = leaf_node(response, Some(id), &parent_sources, &parent_targets, &slice);
        leaf.parent = Some(id.into());
        tree.nodes.insert(child_id.clone(), leaf);
        let root = tree.nodes.get_mut(id).ok_or_else(|| Error::Diag {
            code: "decomposition-node-unknown",
            detail: format!("no decomposition node `{id}`"),
        })?;
        root.children = vec![child_id];
        root.kind = Some(Kind::Split);
        root.slice = None;
        root.acceptance = None;
        root.ownership.clear();
        return Ok(());
    }
    let parent = tree.node(id)?.parent.clone();
    let fallback_sources = tree.node(id)?.sources.clone();
    let fallback_targets =
        tree.node(id)?.target_set().into_iter().map(str::to_string).collect::<Vec<_>>();
    let leaf = leaf_node(response, parent.as_deref(), &fallback_sources, &fallback_targets, &slice);
    tree.nodes.insert(id.to_string(), leaf);
    Ok(())
}

fn leaf_node(
    response: &PartitionResponse, parent: Option<&str>, fallback_sources: &[Scope],
    fallback_targets: &[String], slice: &str,
) -> Node {
    let sources = if response.sources.is_empty() {
        fallback_sources.to_vec()
    } else {
        response.sources.clone()
    };
    let target = response
        .target
        .clone()
        .or_else(|| (fallback_targets.len() == 1).then(|| fallback_targets[0].clone()));
    Node {
        parent: parent.map(str::to_string),
        sources,
        target,
        targets: Vec::new(),
        ownership: response.ownership.clone(),
        depends_on: response.depends_on.clone(),
        kind: Some(Kind::Leaf),
        slice: Some(slice.into()),
        acceptance: response.acceptance.clone(),
        ..Node::default()
    }
}

fn partition_request(
    tree: &Decomposition, id: &str, catalog: &Leads, plan: &Plan, used: usize,
    notes: &BTreeMap<String, Vec<String>>,
) -> Result<serde_json::Value, Error> {
    let node = tree.node(id)?;
    // Only the domain's contributing sources reach the request: a
    // focused child may substitute a different lead from the same
    // source, so same-source catalog rows ride along; foreign-source
    // rows are noise the cut cannot bind.
    let contributing: std::collections::BTreeSet<&str> =
        node.sources.iter().map(|scope| scope.source.as_str()).collect();
    let leads: Vec<serde_json::Value> = catalog
        .leads()
        .iter()
        .filter(|lead| contributing.contains(lead.source.as_str()))
        .map(|lead| {
            json!({
                "source": lead.source,
                "lead": lead.lead,
                "synopsis": lead.synopsis,
                "parent": lead.parent,
                "focus": lead.focus,
                "topics": lead.topics,
            })
        })
        .collect();
    let mut request = json!({
        "domain": id,
        "depth": tree.depth(id)?,
        "sources": node.sources,
        "targets": node.target_set().into_iter().collect::<Vec<_>>(),
        "parent": node.parent,
        "parent-measure": project::plan::decomposition::scope_measure(tree, id),
        "leads": leads,
        "plan-targets": plan.targets.keys().collect::<Vec<_>>(),
        "judgments-remaining": MAX_JUDGMENTS.saturating_sub(used),
    });
    if let Some(rows) = notes.get(id) {
        request["findings"] = json!(rows);
    }
    Ok(request)
}

fn review_request(
    tree: &Decomposition, id: &str, response: &PartitionResponse,
    profile: &project::profile::Profile, catalog: &Leads,
) -> Result<serde_json::Value, Error> {
    let node = tree.node(id)?;
    let score = profile.score(&response.assessment)?;
    let leads: Vec<serde_json::Value> = catalog
        .leads()
        .iter()
        .map(|lead| json!({ "source": lead.source, "lead": lead.lead, "synopsis": lead.synopsis }))
        .collect();
    Ok(json!({
        "domain": id,
        "assessment": response.assessment,
        "score": score,
        "threshold": profile.thresholds.slice_split,
        "sources": if response.sources.is_empty() { &node.sources } else { &response.sources },
        "target": response.target,
        "leads": leads,
    }))
}
