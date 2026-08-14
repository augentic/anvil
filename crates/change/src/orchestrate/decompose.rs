//! Engine-owned decomposition: one bounded judgment per open domain.

use std::collections::{BTreeMap, VecDeque};

use artifacts::leads::Leads;
use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::Resolver;
use project::handler::ExecutionPaths;
use project::name::is_kebab;
use project::plan::Plan;
use project::plan::decomposition::{
    BoundProfile, Child, Decomposition, Kind, MAX_JUDGMENTS, Node, PartitionKind,
    PartitionResponse, ReviewVerdict, Scope, VERSION,
};
use project::profile::{Gate, Profiles};
use project::seam::{Source, Workspaces};
use serde_json::json;

use crate::judgment::{partition, review};
use crate::orchestrate::survey::{focused_leads, survey};

/// Result of a completed decomposition loop.
#[derive(Debug, Clone)]
pub struct Decomposed {
    /// Validated hierarchy.
    pub tree: Decomposition,
    /// Close-with-rationale and estimate-caveat lines for `change.md`.
    pub caveats: Vec<String>,
}

/// Recursively partition the bound catalog into a complete tree.
///
/// # Errors
///
/// Budget exhaustion, unready leaves, definition-revision stops,
/// invalid responses after repair, and tree-validation failures.
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
    mut tree: Decomposition, mut queue: VecDeque<String>, persist: bool,
) -> Result<Decomposed, Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let mut judgments = 0_usize;
    let mut caveats = Vec::new();

    while let Some(id) = queue.pop_front() {
        if judgments >= MAX_JUDGMENTS {
            return Err(budget_exhausted());
        }
        judgments += 1;
        let request = partition_request(&tree, &id, catalog, plan, judgments)?;
        let response = partition::partition(provider, &request, |answer| {
            check_targets(plan, answer)?;
            let mut trial = tree.clone();
            apply(&mut trial, &id, answer)?;
            check_progress(&trial)
        })
        .await?;
        check_targets(plan, &response)?;
        match response.kind {
            PartitionKind::Split => {
                let children: Vec<String> =
                    response.children.iter().map(|child| child.id.clone()).collect();
                apply(&mut tree, &id, &response)?;
                for child in children {
                    queue.push_back(child);
                }
            }
            PartitionKind::Leaf => {
                close_or_review(
                    provider,
                    paths,
                    now,
                    plan,
                    catalog,
                    persist,
                    &mut tree,
                    &mut queue,
                    &mut caveats,
                    &mut judgments,
                    &id,
                    &response,
                )
                .await?;
            }
        }
    }

    tree.check()?;
    Ok(Decomposed { tree, caveats })
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
    prune_descendants(tree, &nearest);
    let owned = tree.node(&nearest)?.sources.clone();
    let sources = close_sources(&owned, catalog);
    let node = tree.nodes.get_mut(&nearest).ok_or_else(|| Error::Diag {
        code: "decomposition-node-unknown",
        detail: format!("no decomposition node `{nearest}`"),
    })?;
    node.children.clear();
    node.kind = None;
    node.slice = None;
    node.acceptance = None;
    node.ownership.clear();
    node.sources = sources;
    Ok(nearest)
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

#[expect(
    clippy::too_many_arguments,
    reason = "leaf-readiness carries the live tree, catalog, and persist mode"
)]
async fn close_or_review<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, plan: &Plan, catalog: &mut Leads,
    persist: bool, tree: &mut Decomposition, queue: &mut VecDeque<String>,
    caveats: &mut Vec<String>, judgments: &mut usize, id: &str, response: &PartitionResponse,
) -> Result<(), Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let profile = leaf_profile(provider, plan, response, tree.node(id)?)?;
    if !profile.exceeds(&response.assessment, Gate::SliceSplit)? {
        apply(tree, id, response)?;
        return Ok(());
    }
    if *judgments >= MAX_JUDGMENTS {
        return Err(budget_exhausted());
    }
    *judgments += 1;
    let request = review_request(tree, id, response, profile, catalog)?;
    let review = review::review(provider, &request, |answer| check_focus(catalog, answer)).await?;
    match review.verdict {
        ReviewVerdict::Close => {
            apply(tree, id, response)?;
            if let Some(rationale) = review.rationale.or_else(|| response.rationale.clone()) {
                caveats.push(format!("`{id}` closed above the slice-split threshold: {rationale}"));
            }
            Ok(())
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
            queue.push_front(id.to_string());
            Ok(())
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

/// Validate an in-progress cut. Open children are not yet leaves, so
/// their `decomposition-leaf-incomplete` findings stay deferred until
/// the domain is partitioned. Claimed leaves and every split rule
/// still fail the repair loop.
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
    if item.rule_id.as_deref() != Some("decomposition-leaf-incomplete") {
        return true;
    }
    item.slice
        .as_deref()
        .and_then(|id| tree.nodes.get(id))
        .is_some_and(|node| node.kind == Some(Kind::Leaf))
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
) -> Result<serde_json::Value, Error> {
    let node = tree.node(id)?;
    let leads: Vec<serde_json::Value> = catalog
        .leads()
        .iter()
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
    Ok(json!({
        "domain": id,
        "depth": tree.depth(id)?,
        "sources": node.sources,
        "targets": node.target_set().into_iter().collect::<Vec<_>>(),
        "parent": node.parent,
        "leads": leads,
        "plan-targets": plan.targets.keys().collect::<Vec<_>>(),
        "judgments-remaining": MAX_JUDGMENTS.saturating_sub(used),
    }))
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
