//! Deterministic projector: terminal domains → `plan.yaml.slices`.

use std::collections::BTreeMap;

use error::Error;

use super::compile::edges;
use super::tree::Decomposition;
use crate::name::SliceName;
use crate::plan::model::{Entry, Plan, SliceSourceBinding};

/// Project terminal domains into plan entries (byte-stable).
///
/// Leaves emit in preorder of `children` arrays. Each entry carries
/// the required `target` (never omitted, never a `project` field) and
/// compiled `depends-on` edges, sorted.
///
/// # Errors
///
/// Tree lookup failures from the domain-dependency compiler.
pub fn slices(tree: &Decomposition) -> Result<Vec<Entry>, Error> {
    from_edges(tree, &edges(tree)?)
}

/// Build entries from an already-compiled leaf graph.
///
/// # Errors
///
/// `decomposition-leaf-incomplete` when a leaf has no target;
/// node-lookup failures.
pub(super) fn from_edges(
    tree: &Decomposition, edges: &BTreeMap<SliceName, Vec<SliceName>>,
) -> Result<Vec<Entry>, Error> {
    let mut out = Vec::new();
    collect(tree, &tree.root, edges, &mut out)?;
    Ok(out)
}

fn collect(
    tree: &Decomposition, id: &str, edges: &BTreeMap<SliceName, Vec<SliceName>>,
    out: &mut Vec<Entry>,
) -> Result<(), Error> {
    let node = tree.node(id)?;
    if node.is_leaf() {
        let slice = tree.leaf_slice(id)?;
        let target = node
            .target_set()
            .into_iter()
            .next()
            .ok_or_else(|| Error::Diag {
                code: "decomposition-leaf-incomplete",
                detail: format!("leaf `{id}` must bind exactly one target"),
            })?
            .to_string();
        let mut entry = Entry::named(slice.clone(), target);
        entry.sources = node
            .sources
            .iter()
            .map(|scope| SliceSourceBinding::structured(&scope.source, &scope.lead))
            .collect();
        let mut depends = edges.get(&slice).cloned().unwrap_or_default();
        depends.sort();
        entry.depends_on = depends;
        out.push(entry);
        return Ok(());
    }
    for child in &node.children {
        collect(tree, child, edges, out)?;
    }
    Ok(())
}

/// Exact-projection check: `plan.slices` matches [`slices`].
///
/// Compares name, target, contributing sources, and `depends-on`.
/// Review fields (`description`, divergence, …) are not projection.
///
/// # Errors
///
/// `decomposition-plan-drift` when the plan is not the projection;
/// compile failures from [`slices`].
pub fn matches_plan(tree: &Decomposition, plan: &Plan) -> Result<(), Error> {
    let expected = slices(tree)?;
    if expected.len() != plan.entries.len() {
        return drift(format!(
            "plan has {} slices; decomposition projects {}",
            plan.entries.len(),
            expected.len()
        ));
    }
    for (want, got) in expected.iter().zip(plan.entries.iter()) {
        if want.name != got.name {
            return drift(format!("projected slice `{}`, plan has `{}`", want.name, got.name));
        }
        if want.target != got.target {
            return drift(format!(
                "slice `{}` target `{}` drifted from projected `{}`",
                got.name, got.target, want.target
            ));
        }
        if want.sources != got.sources {
            return drift(format!("slice `{}` sources drifted from the projection", got.name));
        }
        let mut want_deps = want.depends_on.clone();
        let mut got_deps = got.depends_on.clone();
        want_deps.sort();
        got_deps.sort();
        if want_deps != got_deps {
            return drift(format!("slice `{}` depends-on drifted from the projection", got.name));
        }
    }
    Ok(())
}

const fn drift(detail: String) -> Result<(), Error> {
    Err(Error::Diag {
        code: "decomposition-plan-drift",
        detail,
    })
}
