//! RFC-96 D8 domain convergence: frontier rounds verify a composed
//! wave candidate pre-commit, complete rounds verify accepted trees;
//! durable [`DomainRound`] records are reused by identity on restart.

use std::collections::{BTreeMap, BTreeSet};

use error::Error;
use jiff::Timestamp;
use project::adapter::Resolver;
use project::build_record::BuildRecord;
use project::config::Layout;
use project::domain::{self, DomainRound, RoundKind, Verdict};
use project::handler::ExecutionPaths;
use project::journal::{self, Event, EventKind};
use project::name::SliceName;
use project::plan::decomposition::Decomposition;
use project::plan::{Plan, Status, in_scope, project_ladders};
use project::seam::{Target, Workspaces, target_id};
use project::snapshot::{CodePatch, SnapshotId};
use project::wave::{Wave, accepted_cid};

/// One failed round — the execute loop's typed stop input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// Round kind that failed.
    pub kind: RoundKind,
    /// Decomposition node id.
    pub domain: String,
    /// Operator-facing detail.
    pub detail: String,
}

/// Run the frontier round guarding `slice`'s uncommitted multi-member
/// wave (RFC-96 D8). One-member waves, plans without a decomposition,
/// and members outside the tree gate nothing (`Ok(None)`); a recorded
/// round with the same identity is reused without recomposition.
///
/// # Errors
///
/// Decomposition / record / journal I/O, composition failures, and
/// verify dispatch failures (a dispatch failure is not a verdict).
pub async fn frontier<T: Target + Workspaces, R: Resolver>(
    targets: &T, resolver: &R, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, slice: &str,
) -> Result<Option<Failure>, Error> {
    let layout = paths.layout();
    let Some(tree) = Decomposition::load_opt(&layout.decomposition_path())? else {
        return Ok(None);
    };
    let events = journal::read_union(layout)?;
    let Some((wave, wave_digest)) = live_wave(layout, &events, plan, slice)? else {
        return Ok(None);
    };
    if wave.members.len() < 2 {
        return Ok(None);
    }
    let Some(domain) = nearest_domain(&tree, &wave)? else {
        return Ok(None);
    };
    let mut records = Vec::with_capacity(wave.members.len());
    for member in &wave.members {
        let slice_dir = layout.slice_dir(member.slice.as_str());
        records.push(BuildRecord::load_for_wave(&slice_dir, &wave_digest)?);
    }
    let mut children = Vec::with_capacity(records.len());
    for record in &records {
        children.push(record.digest()?);
    }
    let touched: BTreeSet<String> =
        records.iter().flat_map(|record| record.touched.iter().cloned()).collect();
    let mut key = DomainRound {
        version: domain::VERSION,
        domain: domain.clone(),
        kind: RoundKind::Frontier,
        verdict: Verdict::Passed,
        targets: vec![wave.target.clone()],
        revision: tree.digest()?,
        authorization: wave.build_authorization.clone(),
        bases: BTreeMap::from([(wave.target.clone(), wave.base.clone())]),
        children,
        waves: vec![wave_digest],
        results: BTreeMap::new(),
        protected_inputs: closure_digest(&tree, &domain, &touched)?,
        verification_report: None,
    };
    if let Some(recorded) = DomainRound::find(layout, &wave.target, &key)? {
        return Ok(failure_of(&recorded));
    }
    let patches: Vec<CodePatch> = records.iter().map(BuildRecord::to_patch).collect();
    let candidate =
        targets.compose(wave.base.clone(), patches).await.map_err(|err| Error::Diag {
            code: "domain-frontier-compose-failed",
            detail: format!("composing the frontier candidate for `{domain}` failed: {err}"),
        })?;
    let (verdict, report) =
        verify_tree(targets, resolver, paths, plan, &wave.target, &candidate.result).await?;
    key.verdict = verdict;
    key.results.insert(wave.target.clone(), candidate.result);
    key.verification_report = Some(report);
    record_round(layout, now, &key)?;
    Ok(failure_of(&key))
}

/// Record complete rounds for every domain whose children and
/// dependencies completed (RFC-96 D8), bottom-up; the root's passing
/// round is the drain gate. Recorded rounds are reused by identity.
/// Returns the first failed round, or `None` when everything recorded
/// passes (or nothing gates).
///
/// # Errors
///
/// Decomposition / record / journal I/O and verify dispatch failures.
pub async fn complete<T: Target + Workspaces, R: Resolver>(
    targets: &T, resolver: &R, paths: &ExecutionPaths, now: Timestamp, plan: &Plan,
) -> Result<Option<Failure>, Error> {
    let layout = paths.layout();
    let Some(tree) = Decomposition::load_opt(&layout.decomposition_path())? else {
        return Ok(None);
    };
    let events = journal::read_union(layout)?;
    let ladders = project_ladders(plan, &events);
    let revision = tree.digest()?;
    let mut rounds: BTreeMap<String, DomainRound> = BTreeMap::new();
    for id in post_order(&tree)? {
        let node = tree.node(&id)?;
        if node.is_leaf() {
            continue;
        }
        if !children_done(&tree, plan, layout, &ladders, &rounds, node, &events)? {
            continue;
        }
        let domain_targets = domain_targets(&tree, plan, &id)?;
        let round = if domain_targets.len() == 1 {
            let target = &domain_targets[0];
            let Some(accepted) = accepted_cid(layout, &events, target)? else {
                continue;
            };
            verified_round(
                targets, resolver, paths, now, plan, &tree, &revision, &events, &id, target,
                &accepted, &rounds,
            )
            .await?
        } else {
            aggregate_round(layout, now, &tree, &revision, &events, &id, &domain_targets, &rounds)?
        };
        let failed = failure_of(&round);
        rounds.insert(id, round);
        if failed.is_some() {
            return Ok(failed);
        }
    }
    Ok(None)
}

/// One verified single-target complete round: reuse by identity, else
/// verify the accepted tree and record.
#[expect(clippy::too_many_arguments, reason = "one-shot assembly of the round's bound inputs")]
async fn verified_round<T: Target + Workspaces, R: Resolver>(
    targets: &T, resolver: &R, paths: &ExecutionPaths, now: Timestamp, plan: &Plan,
    tree: &Decomposition, revision: &SnapshotId, events: &[Event], id: &str, target: &str,
    accepted: &SnapshotId, rounds: &BTreeMap<String, DomainRound>,
) -> Result<DomainRound, Error> {
    let layout = paths.layout();
    let touched = terminal_touched(tree, layout, id)?;
    let mut key = DomainRound {
        version: domain::VERSION,
        domain: id.to_string(),
        kind: RoundKind::Complete,
        verdict: Verdict::Passed,
        targets: vec![target.to_string()],
        revision: revision.clone(),
        authorization: covering_epoch(events),
        bases: BTreeMap::from([(target.to_string(), accepted.clone())]),
        children: child_round_digests(tree, id, rounds, layout, target)?,
        waves: committed_waves(events, target),
        results: BTreeMap::from([(target.to_string(), accepted.clone())]),
        protected_inputs: closure_digest(tree, id, &touched)?,
        verification_report: None,
    };
    if let Some(recorded) = DomainRound::find(layout, target, &key)? {
        return Ok(recorded);
    }
    let (verdict, report) = verify_tree(targets, resolver, paths, plan, target, accepted).await?;
    key.verdict = verdict;
    key.verification_report = Some(report);
    record_round(layout, now, &key)?;
    Ok(key)
}

/// One multi-target aggregate round: ordered child verdicts, no
/// composition and no verification (RFC-96 D8).
#[expect(clippy::too_many_arguments, reason = "one-shot assembly of the round's bound inputs")]
fn aggregate_round(
    layout: Layout<'_>, now: Timestamp, tree: &Decomposition, revision: &SnapshotId,
    events: &[Event], id: &str, domain_targets: &[String], rounds: &BTreeMap<String, DomainRound>,
) -> Result<DomainRound, Error> {
    let mut children = Vec::new();
    let mut verdict = Verdict::Passed;
    for child in &tree.node(id)?.children {
        if let Some(round) = rounds.get(child) {
            children.push(round.digest()?);
            if round.verdict == Verdict::Failed {
                verdict = Verdict::Failed;
            }
        }
    }
    let mut bases = BTreeMap::new();
    for target in domain_targets {
        if let Some(accepted) = accepted_cid(layout, events, target)? {
            bases.insert(target.clone(), accepted);
        }
    }
    let key = DomainRound {
        version: domain::VERSION,
        domain: id.to_string(),
        kind: RoundKind::Complete,
        verdict,
        targets: domain_targets.to_vec(),
        revision: revision.clone(),
        authorization: covering_epoch(events),
        results: bases.clone(),
        bases,
        children,
        waves: Vec::new(),
        protected_inputs: closure_digest(tree, id, &BTreeSet::new())?,
        verification_report: None,
    };
    if let Some(target) = domain_targets.first()
        && let Some(recorded) = DomainRound::find(layout, target, &key)?
    {
        return Ok(recorded);
    }
    record_round(layout, now, &key)?;
    Ok(key)
}

/// Persist one round under every bound target and journal
/// `domain.convergence.recorded` per target.
fn record_round(layout: Layout<'_>, now: Timestamp, round: &DomainRound) -> Result<(), Error> {
    let digest = round.write(layout)?;
    for target in &round.targets {
        journal::append_one(
            layout,
            &Event::new(
                now,
                EventKind::DomainConvergenceRecorded {
                    target: target.clone(),
                    domain: round.domain.clone(),
                    kind: round.kind,
                    digest: digest.to_string(),
                    verdict: round.verdict,
                },
            ),
        )?;
    }
    Ok(())
}

/// Verify `snapshot` for `target`: one read-only workspace, one
/// `target.verify` dispatch, verdict from blocking findings. Returns
/// the verdict and the phase report's content digest.
async fn verify_tree<T: Target + Workspaces, R: Resolver>(
    targets: &T, resolver: &R, paths: &ExecutionPaths, plan: &Plan, target: &str,
    snapshot: &SnapshotId,
) -> Result<(Verdict, SnapshotId), Error> {
    let binding = plan.target(target)?;
    let adapter = resolver.resolve_target(&binding.adapter.selector(), paths)?;
    let id = target_id(&adapter.manifest);
    let workspace = targets.prepare(snapshot.clone(), false).await.map_err(|err| Error::Diag {
        code: "domain-verify-prepare-failed",
        detail: format!("preparing the domain verification workspace failed: {err}"),
    })?;
    let dispatched = targets.verify(id.clone(), workspace.clone()).await;
    if let Err(err) = targets.discard(workspace.id.clone()).await {
        tracing::warn!(workspace = %workspace.id, "domain verify workspace discard failed: {err}");
    }
    let report = dispatched.map_err(|err| Error::Diag {
        code: "domain-verify-dispatch-failed",
        detail: format!("the domain `verify` dispatch for `{id}` failed: {err}"),
    })?;
    let verdict = if report.has_blocking() { Verdict::Failed } else { Verdict::Passed };
    let yaml = project::fs::yaml(&report)?;
    let digest = SnapshotId::from_digest(&diagnostics::digest::sha256_hex(yaml.as_bytes()));
    Ok((verdict, digest))
}

/// The typed failure for a failed round, `None` on a pass.
fn failure_of(round: &DomainRound) -> Option<Failure> {
    (round.verdict == Verdict::Failed).then(|| Failure {
        kind: round.kind,
        domain: round.domain.clone(),
        detail: match round.kind {
            RoundKind::Frontier => format!(
                "domain `{}` frontier verification failed over the composed wave candidate; \
                 the wave is parked — repair the members (re-refine or amend) to retract it",
                round.domain
            ),
            RoundKind::Complete => format!(
                "domain `{}` complete-round verification failed over the accepted tree; \
                 dependants, drain, and publication are blocked until an authorized repair \
                 advances the epoch",
                round.domain
            ),
        },
    })
}

/// The slice's newest open, uncommitted wave and its digest.
fn live_wave(
    layout: Layout<'_>, events: &[Event], plan: &Plan, slice: &str,
) -> Result<Option<(Wave, SnapshotId)>, Error> {
    let Some(entry) = plan.entries.iter().find(|entry| entry.name.as_str() == slice) else {
        return Ok(None);
    };
    let committed: BTreeSet<&str> = events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::TargetMergeWaveCommitted { digest, .. } => Some(digest.as_str()),
            _ => None,
        })
        .collect();
    let newest = events.iter().rev().find_map(|event| match &event.kind {
        EventKind::TargetWaveOpened {
            target,
            digest,
            members,
        } if *target == entry.target && members.iter().any(|m| m.as_str() == slice) => {
            Some(digest.clone())
        }
        _ => None,
    });
    let Some(digest) = newest else {
        return Ok(None);
    };
    if committed.contains(digest.as_str()) {
        return Ok(None);
    }
    let wave = Wave::load(layout, &entry.target, &digest)?;
    Ok(Some((wave, SnapshotId::parse(&digest)?)))
}

/// Nearest common ancestor domain of the wave's member leaves; `None`
/// when any member maps to no decomposition leaf.
fn nearest_domain(tree: &Decomposition, wave: &Wave) -> Result<Option<String>, Error> {
    let mut common: Option<Vec<String>> = None;
    for member in &wave.members {
        let Ok(leaf) = tree.leaf_id(member.slice.as_str()) else {
            return Ok(None);
        };
        let path = tree.ancestry(leaf)?;
        common = Some(match common {
            None => path,
            Some(prior) => {
                prior.into_iter().zip(path).take_while(|(a, b)| a == b).map(|(a, _)| a).collect()
            }
        });
    }
    Ok(common.and_then(|path| path.last().cloned()))
}

/// Whether every child (leaf merged / domain round recorded and
/// passed) and every dependency of `node` completed.
fn children_done(
    tree: &Decomposition, plan: &Plan, layout: Layout<'_>,
    ladders: &std::collections::HashMap<SliceName, Status>, rounds: &BTreeMap<String, DomainRound>,
    node: &project::plan::decomposition::Node, events: &[Event],
) -> Result<bool, Error> {
    for id in node.children.iter().chain(&node.depends_on) {
        let child = tree.node(id)?;
        if child.is_leaf() {
            if !leaf_done(tree, plan, layout, ladders, id, events)? {
                return Ok(false);
            }
        } else if rounds.get(id).is_none_or(|round| round.verdict == Verdict::Failed) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether a leaf's slice merged. Out-of-scope leaves (dropped
/// slices, absent entries) contribute nothing and count as done.
fn leaf_done(
    tree: &Decomposition, plan: &Plan, layout: Layout<'_>,
    ladders: &std::collections::HashMap<SliceName, Status>, id: &str, events: &[Event],
) -> Result<bool, Error> {
    let slice = tree.leaf_slice(id)?;
    let Some(entry) = plan.entries.iter().find(|entry| entry.name == slice) else {
        return Ok(true);
    };
    let meta =
        project::slice::SliceMetadata::load_optional(&layout.slice_dir(entry.name.as_str()))?;
    if !in_scope(plan, entry, meta.as_ref(), events) {
        return Ok(true);
    }
    Ok(ladders.get(&slice).copied() == Some(Status::Done))
}

/// The domain's bound target set: declared bindings, else the union
/// of its terminal leaves' plan targets, in canonical order.
fn domain_targets(tree: &Decomposition, plan: &Plan, id: &str) -> Result<Vec<String>, Error> {
    let declared: BTreeSet<String> =
        tree.node(id)?.target_set().iter().map(ToString::to_string).collect();
    if !declared.is_empty() {
        return Ok(declared.into_iter().collect());
    }
    let mut derived = BTreeSet::new();
    for terminal in tree.terminals(id)? {
        let slice = tree.leaf_slice(&terminal)?;
        if let Some(entry) = plan.entries.iter().find(|entry| entry.name == slice) {
            derived.insert(entry.target.clone());
        }
    }
    Ok(derived.into_iter().collect())
}

/// Digests of the recorded rounds for `id`'s child domains — the
/// in-memory pass first, then the durable store (restart reuse).
fn child_round_digests(
    tree: &Decomposition, id: &str, rounds: &BTreeMap<String, DomainRound>, layout: Layout<'_>,
    target: &str,
) -> Result<Vec<SnapshotId>, Error> {
    let mut digests = Vec::new();
    for child in &tree.node(id)?.children {
        if tree.node(child)?.is_leaf() {
            continue;
        }
        if let Some(round) = rounds.get(child) {
            digests.push(round.digest()?);
        } else if let Some(round) = DomainRound::load_all(layout, target)?
            .into_iter()
            .rev()
            .find(|round| round.domain == *child && round.kind == RoundKind::Complete)
        {
            digests.push(round.digest()?);
        }
    }
    Ok(digests)
}

/// The target's committed-wave chain digests, in fact order.
fn committed_waves(events: &[Event], target: &str) -> Vec<SnapshotId> {
    events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::TargetMergeWaveCommitted {
                target: committed,
                digest,
                ..
            } if committed == target => SnapshotId::parse(digest).ok(),
            _ => None,
        })
        .collect()
}

/// Union of the touched paths on each terminal leaf's newest build
/// record — the contributing patches a closure entry must survive.
fn terminal_touched(
    tree: &Decomposition, layout: Layout<'_>, id: &str,
) -> Result<BTreeSet<String>, Error> {
    let mut touched = BTreeSet::new();
    for terminal in tree.terminals(id)? {
        let slice = tree.leaf_slice(&terminal)?;
        let slice_dir = layout.slice_dir(slice.as_str());
        if let Ok(record) = BuildRecord::load_latest(&slice_dir) {
            touched.extend(record.touched.iter().cloned());
        }
    }
    Ok(touched)
}

/// The domain's protected-input closure digest over every descendant
/// node's declared sets minus `touched` (RFC-96 D8).
fn closure_digest(
    tree: &Decomposition, id: &str, touched: &BTreeSet<String>,
) -> Result<SnapshotId, Error> {
    let mut ids = Vec::new();
    collect_subtree(tree, id, &mut ids)?;
    let mut declared = Vec::with_capacity(ids.len());
    for node_id in &ids {
        let node = tree.node(node_id)?;
        declared.push((node.covered.as_slice(), node.oracles.as_slice()));
    }
    domain::protected_closure(&declared, touched).digest()
}

/// Every node id in `id`'s subtree, excluding `id` itself.
fn collect_subtree(tree: &Decomposition, id: &str, out: &mut Vec<String>) -> Result<(), Error> {
    for child in &tree.node(id)?.children {
        out.push(child.clone());
        collect_subtree(tree, child, out)?;
    }
    Ok(())
}

/// Post-order (children before parents) over the tree from the root.
fn post_order(tree: &Decomposition) -> Result<Vec<String>, Error> {
    fn walk(tree: &Decomposition, id: &str, out: &mut Vec<String>) -> Result<(), Error> {
        for child in &tree.node(id)?.children {
            walk(tree, child, out)?;
        }
        out.push(id.to_string());
        Ok(())
    }
    let mut out = Vec::new();
    walk(tree, &tree.root, &mut out)?;
    Ok(out)
}

/// Newest `plan.execute.started` epoch ref, else an unbound
/// `{ writer, sequence: 0 }` for breakout runs.
fn covering_epoch(events: &[Event]) -> journal::FactEpochRef {
    events
        .iter()
        .rev()
        .find_map(|event| match event.kind {
            EventKind::PlanExecuteStarted { .. } => Some(journal::FactEpochRef {
                writer: event.writer.clone(),
                sequence: event.sequence,
            }),
            _ => None,
        })
        .unwrap_or_else(|| journal::FactEpochRef {
            writer: journal::writer_id(),
            sequence: 0,
        })
}
