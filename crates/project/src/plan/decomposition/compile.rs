//! Expand domain→domain `depends-on` into exit-leaf → entry-leaf edges.

use std::collections::{BTreeMap, BTreeSet};

use error::Error;

use super::tree::Decomposition;
use crate::name::SliceName;

/// Compiled leaf `depends-on` graph, keyed by terminal slice name.
///
/// Authored leaf-to-leaf edges copy through. A `depends-on` whose
/// either end is an internal domain expands to every exit leaf of the
/// predecessor times every entry leaf of the successor. Exit leaves
/// have no successor inside that domain; entry leaves have no
/// predecessor inside it.
///
/// # Errors
///
/// `decomposition-node-unknown` / `decomposition-not-leaf` when a
/// `depends-on` names a missing node or a leaf has no slice mapping.
pub fn edges(tree: &Decomposition) -> Result<BTreeMap<SliceName, Vec<SliceName>>, Error> {
    let mut edges: BTreeMap<SliceName, BTreeSet<SliceName>> = BTreeMap::new();
    for id in tree.nodes.keys() {
        if tree.node(id)?.is_leaf() {
            edges.entry(tree.leaf_slice(id)?).or_default();
        }
    }
    for (id, node) in &tree.nodes {
        if !node.is_leaf() {
            continue;
        }
        let slice = tree.leaf_slice(id)?;
        for dep in &node.depends_on {
            if tree.node(dep)?.is_leaf() {
                edges.entry(slice.clone()).or_default().insert(tree.leaf_slice(dep)?);
            }
        }
    }
    let mut order: Vec<String> = tree.nodes.keys().cloned().collect();
    order.sort_by_key(|id| (std::cmp::Reverse(tree.depth(id).unwrap_or(0)), id.clone()));
    for id in order {
        let deps = tree.node(&id)?.depends_on.clone();
        for pred in deps {
            if tree.node(&id)?.is_leaf() && tree.node(&pred)?.is_leaf() {
                continue;
            }
            let exits = exits(tree, &pred, &edges)?;
            let entries = entries(tree, &id, &edges)?;
            for entry in entries {
                for exit in &exits {
                    edges.entry(entry.clone()).or_default().insert(exit.clone());
                }
            }
        }
    }
    Ok(edges.into_iter().map(|(name, set)| (name, set.into_iter().collect())).collect())
}

fn terminals(tree: &Decomposition, id: &str) -> Result<Vec<SliceName>, Error> {
    tree.terminals(id)?.into_iter().map(|leaf| tree.leaf_slice(&leaf)).collect()
}

fn exits(
    tree: &Decomposition, id: &str, edges: &BTreeMap<SliceName, BTreeSet<SliceName>>,
) -> Result<Vec<SliceName>, Error> {
    let inside: BTreeSet<SliceName> = terminals(tree, id)?.into_iter().collect();
    Ok(inside
        .iter()
        .filter(|leaf| {
            !edges.iter().any(|(succ, deps)| inside.contains(succ) && deps.contains(*leaf))
        })
        .cloned()
        .collect())
}

fn entries(
    tree: &Decomposition, id: &str, edges: &BTreeMap<SliceName, BTreeSet<SliceName>>,
) -> Result<Vec<SliceName>, Error> {
    let inside: BTreeSet<SliceName> = terminals(tree, id)?.into_iter().collect();
    Ok(inside
        .iter()
        .filter(|leaf| {
            edges.get(*leaf).is_none_or(|deps| deps.iter().all(|dep| !inside.contains(dep)))
        })
        .cloned()
        .collect())
}
