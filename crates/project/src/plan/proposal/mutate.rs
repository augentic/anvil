//! Domain-mutation + reprojection for `plan add` / `amend` / `remove`
//! once a `decomposition.yaml` exists.

use error::Error;

use super::apply::{commit_tree, overlay};
use crate::config::Layout;
use crate::plan::decomposition::{self, Decomposition, Kind, Node, Scope, slices};
use crate::plan::model::{Entry, Plan};

/// Whether `layout` has a decomposition to reproject through.
#[must_use]
pub fn present(layout: Layout<'_>) -> bool {
    layout.decomposition_path().is_file()
}

/// Append `entry` as a leaf under the unique eligible parent.
///
/// # Errors
///
/// `plan-mutation-ambiguous` when no unique parent exists; tree
/// validation or projection failures.
pub fn add(layout: Layout<'_>, plan: &mut Plan, entry: &Entry) -> Result<(), Error> {
    let mut tree = Decomposition::load(&layout.decomposition_path())?;
    let parent = unique_parent(&tree, &entry.target)?;
    let id = entry.name.to_string();
    if tree.nodes.contains_key(&id) {
        return Err(Error::Diag {
            code: "plan-entry-duplicate-name",
            detail: format!("plan already contains an entry named `{id}`"),
        });
    }
    let deps = entry
        .depends_on
        .iter()
        .map(|dep| tree.leaf_id(dep.as_str()).map(str::to_string))
        .collect::<Result<Vec<_>, _>>()?;
    let mut leaf = Node::leaf(&entry.target, entry.name.clone());
    leaf.parent = Some(parent.clone());
    leaf.kind = Some(Kind::Leaf);
    leaf.sources = scopes(entry);
    leaf.depends_on = deps;
    leaf.ownership =
        if entry.context.is_empty() { vec![format!("{id}/**")] } else { entry.context.clone() };
    leaf.acceptance = entry.description.clone().or_else(|| Some(id.clone()));
    tree.nodes.insert(id.clone(), leaf);
    tree.node_mut(&parent)?.children.push(id);
    commit(layout, plan, &tree)?;
    if let Some(got) = plan.entries.iter_mut().find(|row| row.name == entry.name) {
        got.description.clone_from(&entry.description);
        got.context.clone_from(&entry.context);
        got.authority_override.clone_from(&entry.authority_override);
        got.allow_composition_replace = entry.allow_composition_replace;
        plan.save(&layout.plan_path())?;
    }
    Ok(())
}

/// Apply a topology patch (sources / depends-on) to the named leaf.
///
/// Review-only fields are not this kernel — callers mutate the entry
/// directly when the decomposition is absent or the edit is not
/// projection.
///
/// # Errors
///
/// Unknown leaf, validation, or projection failures.
pub fn amend(
    layout: Layout<'_>, plan: &mut Plan, name: &str, sources: Option<Vec<Scope>>,
    depends_on: Option<Vec<String>>,
) -> Result<(), Error> {
    let mut tree = Decomposition::load(&layout.decomposition_path())?;
    let id = tree.leaf_id(name)?.to_string();
    let deps = depends_on
        .map(|names| {
            names
                .into_iter()
                .map(|dep| tree.leaf_id(&dep).map(str::to_string))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    let node = tree.node_mut(&id)?;
    if let Some(sources) = sources {
        node.sources = sources;
    }
    if let Some(depends_on) = deps {
        node.depends_on = depends_on;
    }
    commit(layout, plan, &tree)
}

/// Remove the named leaf when the remaining tree stays unambiguous.
///
/// # Errors
///
/// `plan-mutation-ambiguous` when removal would collapse a non-root
/// split or leave an incomplete tree; unknown leaf.
pub fn remove(layout: Layout<'_>, plan: &mut Plan, name: &str) -> Result<(), Error> {
    let mut tree = Decomposition::load(&layout.decomposition_path())?;
    let id = tree.leaf_id(name)?.to_string();
    let parent =
        tree.node(&id)?.parent.clone().ok_or_else(|| ambiguous("cannot remove the root"))?;
    let referencers: Vec<String> = tree
        .nodes
        .iter()
        .filter(|(other, node)| *other != &id && node.depends_on.iter().any(|dep| dep == &id))
        .map(|(other, _)| other.clone())
        .collect();
    if !referencers.is_empty() {
        return Err(Error::validation_failed(
            "plan-remove-entry-referenced",
            "plan remove refuses when another entry depends on the target",
            format!("slice '{name}' is listed in depends-on by: {}", referencers.join(", ")),
        ));
    }
    tree.nodes.remove(&id);
    let root = tree.root.clone();
    let parent_node = tree.node_mut(&parent)?;
    parent_node.children.retain(|child| child != &id);
    if parent != root && parent_node.children.len() < 2 {
        return Err(ambiguous("removing this leaf would collapse a non-root split"));
    }
    commit(layout, plan, &tree)
}

fn commit(layout: Layout<'_>, plan: &mut Plan, tree: &Decomposition) -> Result<(), Error> {
    tree.check().map_err(|err| match err {
        Error::Validation { detail, .. } => Error::validation_failed(
            "plan-mutation-ambiguous",
            "plan add/amend/remove reproject a valid decomposition",
            detail,
        ),
        other => other,
    })?;
    let projected = slices(tree)?;
    let entries = overlay(&plan.entries, projected);
    let mut trial = plan.clone();
    trial.entries.clone_from(&entries);
    decomposition::matches_plan(tree, &trial)?;
    tree.save(&layout.decomposition_path())?;
    commit_tree(layout, plan, entries)?;
    Ok(())
}

fn unique_parent(tree: &Decomposition, target: &str) -> Result<String, Error> {
    let mut candidates: Vec<String> = tree
        .nodes
        .iter()
        .filter(|(_, node)| {
            !node.is_leaf() && (node.target_set().is_empty() || node.target_set().contains(target))
        })
        .map(|(id, _)| id.clone())
        .collect();
    if candidates.len() == 1 {
        return Ok(candidates.remove(0));
    }
    let leaves_only: Vec<String> = candidates
        .iter()
        .filter(|id| {
            tree.nodes.get(*id).is_some_and(|node| {
                !node.children.is_empty()
                    && node
                        .children
                        .iter()
                        .all(|child| tree.nodes.get(child).is_some_and(Node::is_leaf))
            })
        })
        .cloned()
        .collect();
    if leaves_only.len() == 1 {
        return Ok(leaves_only[0].clone());
    }
    Err(ambiguous("no unique parent domain can accept this slice"))
}

fn scopes(entry: &Entry) -> Vec<Scope> {
    entry
        .sources
        .iter()
        .map(|binding| Scope::new(binding.source(), binding.lead(entry.name.as_str())))
        .collect()
}

fn ambiguous(detail: &str) -> Error {
    Error::validation_failed(
        "plan-mutation-ambiguous",
        "plan add/amend/remove refuse when no unambiguous hierarchy edit exists",
        detail,
    )
}
