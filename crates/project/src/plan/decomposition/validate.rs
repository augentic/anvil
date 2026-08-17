//! Deterministic split/leaf/budget validators. Each finding is a typed
//! `decomposition-*` or `publication-target-cycle` diagnostic.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use diagnostics::{Diagnostic, Severity};

use super::compile::edges;
use super::contraction;
use super::tree::{BoundProfile, Decomposition, Kind, MAX_DEPTH, MAX_NODES, Node, Scope};
use crate::plan::Entry;
use crate::plan::validate::finding;

/// Every blocking rule over `tree`. Shape findings first; compiled
/// leaf-graph checks run only when the containment tree is coherent.
#[must_use]
pub fn findings(tree: &Decomposition) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    out.extend(shape(tree));
    out.extend(profiles(tree));
    out.extend(budgets(tree));
    if out.iter().any(|item| item.rule_id.as_deref() == Some("decomposition-root-unknown")) {
        return out;
    }
    out.extend(splits(tree));
    out.extend(leaves(tree));
    if !out.is_empty() {
        return out;
    }
    match edges(tree) {
        Ok(edges) => {
            let entries = match projected_entries(tree, &edges) {
                Ok(entries) => entries,
                Err(err) => {
                    out.push(diag("decomposition-compile", err.to_string(), None));
                    return out;
                }
            };
            out.extend(leaf_cycles(&entries));
            out.extend(contraction::cycles(&entries));
        }
        Err(err) => out.push(diag("decomposition-compile", err.to_string(), None)),
    }
    out
}

fn projected_entries(
    tree: &Decomposition, edges: &BTreeMap<crate::name::SliceName, Vec<crate::name::SliceName>>,
) -> Result<Vec<Entry>, error::Error> {
    super::project::from_edges(tree, edges)
}

fn shape(tree: &Decomposition) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    if !tree.nodes.contains_key(&tree.root) {
        out.push(diag(
            "decomposition-root-unknown",
            format!("root `{}` is not a node", tree.root),
            None,
        ));
        return out;
    }
    if tree.nodes[&tree.root].parent.is_some() {
        out.push(diag(
            "decomposition-root-parent",
            format!("root `{}` must not declare a parent", tree.root),
            Some(tree.root.clone()),
        ));
    }
    let reachable = walk(tree, &mut out);
    for id in tree.nodes.keys() {
        if !reachable.contains(id.as_str()) {
            out.push(diag(
                "decomposition-orphan-node",
                format!("node `{id}` is not reachable from root `{}`", tree.root),
                Some(id.clone()),
            ));
        }
    }
    for (id, node) in &tree.nodes {
        if node.kind == Some(Kind::Split) && node.is_leaf() {
            out.push(diag(
                "decomposition-kind",
                format!("node `{id}` is kind `split` but has no children"),
                Some(id.clone()),
            ));
        }
        if node.kind == Some(Kind::Leaf) && !node.is_leaf() {
            out.push(diag(
                "decomposition-kind",
                format!("node `{id}` is kind `leaf` but has children"),
                Some(id.clone()),
            ));
        }
        let mut seen = BTreeSet::new();
        for scope in &node.sources {
            if !seen.insert(scope) {
                out.push(diag(
                    "decomposition-scope-dup",
                    format!("node `{id}` repeats source `{}` lead `{}`", scope.source, scope.lead),
                    Some(id.clone()),
                ));
            }
        }
        for dep in &node.depends_on {
            if !tree.nodes.contains_key(dep) {
                out.push(diag(
                    "decomposition-dep-unknown",
                    format!("node `{id}` depends-on unknown node `{dep}`"),
                    Some(id.clone()),
                ));
            }
        }
    }
    out
}

fn walk(tree: &Decomposition, out: &mut Vec<Diagnostic>) -> BTreeSet<String> {
    let mut reachable = BTreeSet::new();
    let mut stack = vec![tree.root.as_str()];
    while let Some(id) = stack.pop() {
        if !reachable.insert(id.to_string()) {
            out.push(diag(
                "decomposition-child-cycle",
                format!("containment cycle through `{id}`"),
                Some(id.to_string()),
            ));
            continue;
        }
        let Some(node) = tree.nodes.get(id) else {
            continue;
        };
        for child in &node.children {
            if !tree.nodes.contains_key(child) {
                out.push(diag(
                    "decomposition-child-unknown",
                    format!("node `{id}` lists unknown child `{child}`"),
                    Some(id.to_string()),
                ));
                continue;
            }
            match tree.nodes[child].parent.as_deref() {
                Some(parent) if parent == id => {}
                Some(parent) => out.push(diag(
                    "decomposition-parent-mismatch",
                    format!("node `{child}` parent `{parent}` does not match `{id}`"),
                    Some(child.clone()),
                )),
                None => out.push(diag(
                    "decomposition-parent-mismatch",
                    format!("node `{child}` is a child of `{id}` but declares no parent"),
                    Some(child.clone()),
                )),
            }
            stack.push(child.as_str());
        }
    }
    reachable
}

fn profiles(tree: &Decomposition) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (target, bound) in &tree.profiles {
        if let Err(err) = check_profile(bound) {
            out.push(diag(
                "decomposition-profile-digest",
                format!("target `{target}` profile digest does not match its body: {err}"),
                None,
            ));
        }
    }
    for (id, node) in &tree.nodes {
        if !node.is_leaf() {
            continue;
        }
        for target in node.target_set() {
            if !tree.profiles.contains_key(target) {
                out.push(diag(
                    "decomposition-profile-unknown",
                    format!("leaf `{id}` target `{target}` has no recorded profile"),
                    Some(id.clone()),
                ));
            }
        }
    }
    out
}

fn check_profile(bound: &BoundProfile) -> Result<(), error::Error> {
    let digest = bound.body().digest()?;
    if digest == bound.digest {
        Ok(())
    } else {
        Err(error::Error::Diag {
            code: "decomposition-profile-digest",
            detail: format!("recorded `{}` vs body `{digest}`", bound.digest),
        })
    }
}

fn budgets(tree: &Decomposition) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    if tree.nodes.len() > MAX_NODES {
        out.push(diag(
            "decomposition-nodes",
            format!("decomposition has {} nodes; cap is {MAX_NODES}", tree.nodes.len()),
            None,
        ));
    }
    for id in tree.nodes.keys() {
        match tree.depth(id) {
            Ok(depth) if depth > MAX_DEPTH => out.push(diag(
                "decomposition-depth",
                format!("node `{id}` sits at depth {depth}; cap is {MAX_DEPTH}"),
                Some(id.clone()),
            )),
            Err(err) => out.push(diag("decomposition-depth", err.to_string(), Some(id.clone()))),
            Ok(_) => {}
        }
    }
    out
}

fn splits(tree: &Decomposition) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (id, node) in &tree.nodes {
        if node.is_leaf() {
            continue;
        }
        out.extend(coverage(tree, id, node));
        out.extend(cross_cutting(tree, id, node));
        out.extend(containment(tree, id, node));
        if node.children.len() >= 2 {
            out.extend(reduction(tree, id, node));
            out.extend(overlap(tree, id, node));
        }
    }
    out
}

fn coverage(tree: &Decomposition, id: &str, node: &Node) -> Vec<Diagnostic> {
    let parent: BTreeSet<&Scope> = node.sources.iter().collect();
    let mut child_union = BTreeSet::new();
    for child in &node.children {
        if let Some(node) = tree.nodes.get(child) {
            child_union.extend(node.sources.iter());
        }
    }
    parent
        .difference(&child_union)
        .map(|scope| {
            diag(
                "decomposition-lead-uncovered",
                format!(
                    "node `{id}` lead `{}`/`{}` is missing from every child",
                    scope.source, scope.lead
                ),
                Some(id.to_string()),
            )
        })
        .collect()
}

fn cross_cutting(tree: &Decomposition, id: &str, node: &Node) -> Vec<Diagnostic> {
    let mut homes: BTreeMap<&Scope, Vec<&str>> = BTreeMap::new();
    for child in &node.children {
        let Some(child_node) = tree.nodes.get(child) else {
            continue;
        };
        for scope in &child_node.sources {
            homes.entry(scope).or_default().push(child.as_str());
        }
    }
    let mut out = Vec::new();
    for (scope, informed) in homes {
        if informed.len() < 2 {
            continue;
        }
        for child in &node.children {
            let Some(child_node) = tree.nodes.get(child) else {
                continue;
            };
            if informed.contains(&child.as_str()) {
                continue;
            }
            let replaced = child_node
                .sources
                .iter()
                .any(|other| other.source == scope.source && other.lead != scope.lead);
            if replaced {
                continue;
            }
            out.push(diag(
                "decomposition-lead-dropped",
                format!(
                    "cross-cutting lead `{}`/`{}` on `{id}` is missing from informed child `{child}`",
                    scope.source, scope.lead
                ),
                Some(child.clone()),
            ));
        }
    }
    out
}

fn containment(tree: &Decomposition, id: &str, node: &Node) -> Vec<Diagnostic> {
    let parent = node.target_set();
    if parent.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for child in &node.children {
        let Some(child_node) = tree.nodes.get(child) else {
            continue;
        };
        for target in child_targets(tree, child, child_node) {
            if !parent.contains(target.as_str()) {
                out.push(diag(
                    "decomposition-target-escape",
                    format!("child `{child}` target `{target}` is outside parent `{id}`"),
                    Some(child.clone()),
                ));
            }
        }
    }
    out
}

fn child_targets(tree: &Decomposition, id: &str, node: &Node) -> BTreeSet<String> {
    let declared = node.target_set();
    if !declared.is_empty() {
        return declared.into_iter().map(str::to_string).collect();
    }
    tree.terminals(id).ok().map_or_else(BTreeSet::new, |ids| {
        ids.iter()
            .filter_map(|leaf| tree.nodes.get(leaf))
            .flat_map(Node::target_set)
            .map(str::to_string)
            .collect()
    })
}

fn reduction(tree: &Decomposition, id: &str, node: &Node) -> Vec<Diagnostic> {
    let parent = measure(tree, id, node);
    let mut out = Vec::new();
    for child in &node.children {
        let Some(child_node) = tree.nodes.get(child) else {
            continue;
        };
        let measured = measure(tree, child, child_node);
        if measured >= parent {
            let relation = if measured == parent { "tied on every dimension" } else { "grew" };
            out.push(diag(
                "decomposition-non-reducing",
                format!(
                    "child `{child}` does not strictly reduce the scope of `{id}`: \
                     child ({measured}) vs parent ({parent}); {relation}"
                ),
                Some(child.clone()),
            ));
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Measure {
    leads: usize,
    targets: usize,
    paths: usize,
}

impl std::fmt::Display for Measure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} lead{}, {} target{}, {} path{}",
            self.leads,
            plural(self.leads),
            self.targets,
            plural(self.targets),
            self.paths,
            plural(self.paths)
        )
    }
}

const fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

fn measure(tree: &Decomposition, id: &str, node: &Node) -> Measure {
    Measure {
        leads: node.sources.iter().collect::<BTreeSet<_>>().len(),
        targets: child_targets(tree, id, node).len(),
        paths: ownership(tree, id, node).len(),
    }
}

fn ownership(tree: &Decomposition, id: &str, node: &Node) -> BTreeSet<String> {
    if !node.ownership.is_empty() {
        return node.ownership.iter().cloned().collect();
    }
    tree.terminals(id).ok().map_or_else(BTreeSet::new, |ids| {
        ids.iter()
            .filter_map(|leaf| tree.nodes.get(leaf))
            .flat_map(|leaf| leaf.ownership.iter().cloned())
            .collect()
    })
}

fn overlap(tree: &Decomposition, id: &str, node: &Node) -> Vec<Diagnostic> {
    let siblings: Vec<&str> = node.children.iter().map(String::as_str).collect();
    let graph = sibling_graph(tree, &siblings);
    let mut out = Vec::new();
    for (index, left) in siblings.iter().enumerate() {
        let Some(left_node) = tree.nodes.get(*left) else {
            continue;
        };
        let left_own = ownership(tree, left, left_node);
        for right in siblings.iter().skip(index + 1) {
            let Some(right_node) = tree.nodes.get(*right) else {
                continue;
            };
            let right_own = ownership(tree, right, right_node);
            if !envelopes_overlap(&left_own, &right_own) {
                continue;
            }
            if ordered(&graph, left, right) || fan_in(&graph, &siblings, left, right) {
                continue;
            }
            out.push(diag(
                "decomposition-overlap",
                format!(
                    "siblings `{left}` and `{right}` of `{id}` overlap in ownership without \
                     an explicit order or fan-in child"
                ),
                Some((*left).to_string()),
            ));
        }
    }
    out
}

fn sibling_graph<'a>(
    tree: &'a Decomposition, siblings: &'a [&str],
) -> BTreeMap<&'a str, Vec<&'a str>> {
    let set: BTreeSet<&str> = siblings.iter().copied().collect();
    let mut graph = BTreeMap::new();
    for id in siblings {
        let deps = tree.nodes.get(*id).map_or_else(Vec::new, |node| {
            node.depends_on.iter().map(String::as_str).filter(|dep| set.contains(dep)).collect()
        });
        graph.insert(*id, deps);
    }
    graph
}

fn ordered(graph: &BTreeMap<&str, Vec<&str>>, left: &str, right: &str) -> bool {
    reachable(graph, left, right) || reachable(graph, right, left)
}

fn fan_in(graph: &BTreeMap<&str, Vec<&str>>, siblings: &[&str], left: &str, right: &str) -> bool {
    siblings.iter().any(|sink| {
        *sink != left
            && *sink != right
            && reachable(graph, sink, left)
            && reachable(graph, sink, right)
    })
}

fn reachable(graph: &BTreeMap<&str, Vec<&str>>, from: &str, to: &str) -> bool {
    let mut seen = HashSet::new();
    let mut stack = vec![from];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        if id == to && id != from {
            return true;
        }
        if let Some(next) = graph.get(id) {
            stack.extend(next.iter().copied());
        }
    }
    false
}

fn envelopes_overlap(left: &BTreeSet<String>, right: &BTreeSet<String>) -> bool {
    left.iter().any(|a| right.iter().any(|b| globs_overlap(a, b)))
}

fn globs_overlap(left: &str, right: &str) -> bool {
    let a = normalize(left);
    let b = normalize(right);
    a == b || path_prefix(&a, &b) || path_prefix(&b, &a)
}

fn normalize(pattern: &str) -> String {
    pattern
        .trim_end_matches('/')
        .trim_end_matches("/**")
        .trim_end_matches("/*")
        .trim_end_matches('*')
        .trim_end_matches('/')
        .to_string()
}

fn path_prefix(prefix: &str, path: &str) -> bool {
    !prefix.is_empty() && (path == prefix || path.starts_with(&format!("{prefix}/")))
}

fn leaves(tree: &Decomposition) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut slices = BTreeSet::new();
    for (id, node) in &tree.nodes {
        if !node.is_leaf() {
            continue;
        }
        if node.target_set().len() != 1 {
            out.push(diag(
                "decomposition-leaf-incomplete",
                format!("leaf `{id}` must bind exactly one target"),
                Some(id.clone()),
            ));
        }
        if node.ownership.is_empty() {
            out.push(diag(
                "decomposition-leaf-incomplete",
                format!("leaf `{id}` has no ownership manifest"),
                Some(id.clone()),
            ));
        }
        if node.acceptance.as_ref().is_none_or(String::is_empty) {
            out.push(diag(
                "decomposition-leaf-incomplete",
                format!("leaf `{id}` has no acceptance boundary"),
                Some(id.clone()),
            ));
        }
        if node.slice.is_none() {
            out.push(diag(
                "decomposition-leaf-incomplete",
                format!("leaf `{id}` has no terminal slice mapping"),
                Some(id.clone()),
            ));
        } else if let Some(slice) = &node.slice
            && !slices.insert(slice.as_str())
        {
            out.push(diag(
                "decomposition-slice-dup",
                format!("slice `{}` is mapped from more than one leaf", slice.as_str()),
                Some(id.clone()),
            ));
        }
        let mut sources = HashSet::new();
        for scope in &node.sources {
            if !sources.insert(scope.source.as_str()) {
                out.push(diag(
                    "decomposition-source-dup",
                    format!("leaf `{id}` binds source `{}` more than once", scope.source),
                    Some(id.clone()),
                ));
            }
        }
    }
    out
}

fn leaf_cycles(entries: &[Entry]) -> Vec<Diagnostic> {
    crate::plan::doctor::detect(entries)
        .into_iter()
        .map(|mut item| {
            item.rule_id = Some("decomposition-leaf-cycle".into());
            item.fingerprint = diagnostics::fingerprint(&item);
            item
        })
        .collect()
}

fn diag(code: &'static str, detail: impl Into<String>, node: Option<String>) -> Diagnostic {
    finding(code, Severity::Important, detail, node)
}
