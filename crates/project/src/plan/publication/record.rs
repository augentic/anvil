//! The typed publication projection (RFC-95 D7): members, forge
//! state, derived order, and the verification verdict. Byte-stable
//! over unchanged plan, facts, and forge state.

use std::collections::{BTreeMap, BTreeSet};

use error::Error;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::super::model::{Entry, Plan};
use super::{Member, members};
use crate::config::Layout;
use crate::journal::Event;
use crate::seam::{Forge, PrState, PullRequest};

/// The publication-set record: the wire type behind the
/// `publication.schema.json` golden. Plan-backed records are derived;
/// external producers (RFC-95 D8) author the same shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Record {
    /// Change identity — the plan name.
    pub change: String,
    /// One row per publication member, in first-appearance order.
    pub members: Vec<MemberRecord>,
    /// Whole-set verdict.
    pub verification: Verification,
    /// Failing members and reasons; empty when verified.
    pub failures: Vec<FailureRecord>,
}

/// One publication member's projected state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MemberRecord {
    /// Target key under `plan.yaml.targets`.
    pub target: String,
    /// Repository URL from the binding locator, minus the revision.
    pub repository: String,
    /// The forge's merge commit; `null` until merged.
    pub merge_commit: Option<String>,
    /// Publication branch, `change/<plan>`.
    pub branch: String,
    /// Pull-request URL; `null` while unpublished.
    pub pull_request: Option<String>,
    /// Observed pull-request base branch (recorded, not gated — D10).
    pub base: Option<String>,
    /// Forge-derived publication state.
    pub publication: PublicationState,
    /// D4 rank in the contracted DAG; omitted for unrelated members.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<u32>,
}

/// Closed publication state: `unpublished` is the engine's zero-match
/// projection; the rest are forge states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PublicationState {
    /// No matching pull request exists yet.
    Unpublished,
    /// The pull request is open.
    Open,
    /// The pull request merged.
    Merged,
    /// The pull request closed without merging.
    Closed,
}

/// Closed whole-set verification verdict (D7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Verification {
    /// Every member merged in order under matching trailers.
    Verified,
    /// Members remain unpublished or unmerged; nothing failed closed.
    Pending,
    /// A check failed closed (ambiguous match, closed pull request,
    /// out-of-order landing, or a `merged-at` tie).
    Unverified,
}

/// One failing member and its stable reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FailureRecord {
    /// Failing member's target key.
    pub member: String,
    /// Stable reason string.
    pub reason: String,
}

/// One archive-time projection: the wire record plus the forge
/// `merged-at` observations the landed facts carry (not wire fields —
/// the record shape is D7's, `merged-at` rides the journal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    /// The D7 wire record.
    pub record: Record,
    /// Forge `merged-at` per merged member target.
    pub merged_at: BTreeMap<String, String>,
}

/// Project the publication set over the plan, the fact union, and one
/// forge read per member (RFC-95 D7 / D10).
///
/// # Errors
///
/// Plan / slice-metadata failures, and forge transport or auth
/// failures — distinct outcomes, never folded into `unverified`.
pub async fn project<F: Forge>(
    forge: &F, plan: &Plan, layout: Layout<'_>, events: &[Event],
) -> Result<Projection, Error> {
    let members = members(plan, layout, events)?;
    let entries = super::in_scope_entries(plan, layout, events)?;
    let ranks = ranks(&entries);
    let branch = format!("change/{}", plan.name);
    let current_digest = Plan::file_digest(layout)?;
    let mut rows = Vec::with_capacity(members.len());
    let mut failures = Vec::new();
    let mut merged: BTreeMap<String, String> = BTreeMap::new();
    for member in &members {
        let digest =
            member.materialized.as_ref().map_or(current_digest.as_str(), |m| &m.plan_digest);
        let pulls = forge.find(member.repository.clone(), branch.clone()).await.map_err(|err| {
            Error::Diag {
                code: "publication-forge-failed",
                detail: format!("target `{}`: {err}", member.target),
            }
        })?;
        let matching: Vec<&PullRequest> =
            pulls.iter().filter(|pull| trailers_match(&pull.body, &plan.name, digest)).collect();
        let (row, merged_at) = member_row(member, &branch, &matching, &ranks, &mut failures);
        if let Some(at) = merged_at {
            merged.insert(member.target.clone(), at);
        }
        rows.push(row);
    }
    order_failures(&entries, &merged, &mut failures);
    failures.sort_by(|a, b| (&a.member, &a.reason).cmp(&(&b.member, &b.reason)));
    failures.dedup();
    let verification = verdict(&failures);
    Ok(Projection {
        record: Record {
            change: plan.name.to_string(),
            members: rows,
            verification,
            failures,
        },
        merged_at: merged,
    })
}

/// One member row plus its zero / one / several failures; the second
/// element is the forge `merged-at` when the member's pull merged.
fn member_row(
    member: &Member, branch: &str, matching: &[&PullRequest], ranks: &BTreeMap<String, u32>,
    failures: &mut Vec<FailureRecord>,
) -> (MemberRecord, Option<String>) {
    let mut row = MemberRecord {
        target: member.target.clone(),
        repository: member.repository.clone(),
        merge_commit: None,
        branch: branch.to_string(),
        pull_request: None,
        base: None,
        publication: PublicationState::Unpublished,
        order: ranks.get(&member.target).copied(),
    };
    let mut merged_at = None;
    match matching {
        [] => failures.push(fail(&member.target, "unpublished")),
        [pull] => {
            row.pull_request = Some(pull.url.clone());
            row.base = Some(pull.base.clone());
            row.publication = match pull.state {
                PrState::Open => PublicationState::Open,
                PrState::Merged => PublicationState::Merged,
                PrState::Closed => PublicationState::Closed,
            };
            match pull.state {
                PrState::Merged => {
                    row.merge_commit.clone_from(&pull.merge_commit);
                    merged_at.clone_from(&pull.merged_at);
                }
                PrState::Open => failures.push(fail(&member.target, "unmerged")),
                PrState::Closed => {
                    failures.push(fail(&member.target, "closed without merging"));
                }
            }
        }
        several => failures.push(fail(
            &member.target,
            &format!("{} pull requests match the trailers", several.len()),
        )),
    }
    (row, merged_at)
}

/// Both D3 trailer lines, matched line-anchored on the body.
fn trailers_match(body: &str, plan: &str, digest: &str) -> bool {
    let change = format!("Emery-Change: {plan}");
    let covering = format!("Emery-Change-Digest: {digest}");
    let mut has_change = false;
    let mut has_digest = false;
    for line in body.lines().map(str::trim) {
        has_change |= line == change;
        has_digest |= line == covering;
    }
    has_change && has_digest
}

/// D4 landing-order checks over the contracted edges: each ordered
/// pair of merged members must land strictly in order — an equal
/// `merged-at` fails (D7, no tie-break).
fn order_failures(
    entries: &[Entry], merged: &BTreeMap<String, String>, failures: &mut Vec<FailureRecord>,
) {
    for (before, after) in contracted_edges(entries) {
        let (Some(first), Some(second)) = (merged.get(&before), merged.get(&after)) else {
            continue;
        };
        match parse_time(first).cmp(&parse_time(second)) {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => failures
                .push(fail(&after, &format!("merged-at ties with `{before}` on an ordered pair"))),
            std::cmp::Ordering::Greater => {
                failures.push(fail(&after, &format!("landed before its dependency `{before}`")));
            }
        }
    }
}

/// Parse one forge `merged-at` for comparison; unparseable times
/// compare as raw strings via a stable fallback.
fn parse_time(raw: &str) -> (Option<jiff::Timestamp>, String) {
    (raw.parse().ok(), raw.to_string())
}

/// Kahn topological ranks over the contracted in-scope DAG with a
/// sorted ready set (D7 — a closed algorithm, byte-stable output).
///
/// Only members touched by at least one cross-target edge rank;
/// unrelated members carry no order. Never panics: edge endpoints are
/// inserted as nodes before the in-degree walk.
#[must_use]
#[expect(clippy::missing_panics_doc, reason = "edge endpoints are always inserted as nodes")]
pub fn ranks(entries: &[Entry]) -> BTreeMap<String, u32> {
    let edges = contracted_edges(entries);
    let mut nodes: BTreeSet<String> = BTreeSet::new();
    for (from, to) in &edges {
        nodes.insert(from.clone());
        nodes.insert(to.clone());
    }
    let mut indegree: BTreeMap<&str, usize> = nodes.iter().map(|node| (node.as_str(), 0)).collect();
    for (_, to) in &edges {
        *indegree.get_mut(to.as_str()).expect("edge endpoints are nodes") += 1;
    }
    let mut ready: BTreeSet<&str> =
        indegree.iter().filter_map(|(node, degree)| (*degree == 0).then_some(*node)).collect();
    let mut ranks = BTreeMap::new();
    let mut next = 1_u32;
    while let Some(node) = ready.pop_first() {
        ranks.insert(node.to_string(), next);
        next += 1;
        for (from, to) in &edges {
            if from == node {
                let degree = indegree.get_mut(to.as_str()).expect("edge endpoints are nodes");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(to.as_str());
                }
            }
        }
    }
    // A contracted cycle never ranks; archive rejects it before
    // projecting (`publication-target-cycle`).
    ranks
}

/// The cross-target edges of the in-scope leaf graph, deduped and
/// sorted: `(dependency target, dependent target)`.
fn contracted_edges(entries: &[Entry]) -> Vec<(String, String)> {
    let target_of: BTreeMap<&str, &str> =
        entries.iter().map(|entry| (entry.name.as_str(), entry.target.as_str())).collect();
    let mut edges = BTreeSet::new();
    for entry in entries {
        for dep in &entry.depends_on {
            if let Some(from) = target_of.get(dep.as_str())
                && *from != entry.target
            {
                edges.insert(((*from).to_string(), entry.target.clone()));
            }
        }
    }
    edges.into_iter().collect()
}

/// The D7 verdict: hard failures are `unverified`; unpublished /
/// unmerged alone is `pending`; an empty list is `verified`.
fn verdict(failures: &[FailureRecord]) -> Verification {
    if failures.is_empty() {
        return Verification::Verified;
    }
    let pending_only = failures
        .iter()
        .all(|failure| failure.reason == "unpublished" || failure.reason == "unmerged");
    if pending_only { Verification::Pending } else { Verification::Unverified }
}

fn fail(member: &str, reason: &str) -> FailureRecord {
    FailureRecord {
        member: member.to_string(),
        reason: reason.to_string(),
    }
}
