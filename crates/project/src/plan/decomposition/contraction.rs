//! Contract the compiled leaf graph onto distinct targets.
//!
//! Same-target edges disappear. An SCC or self-loop is `publication-target-cycle`.

use std::collections::HashMap;

use diagnostics::{Diagnostic, Severity};
use petgraph::algo::tarjan_scc;
use petgraph::graph::DiGraph;

use crate::plan::Entry;
use crate::plan::validate::finding;

/// Reject target-contraction cycles over `entries`.
///
/// An acyclic leaf graph can still contract to `target-a → target-b`
/// and `target-b → target-a` through different leaves.
#[must_use]
pub fn cycles(entries: &[Entry]) -> Vec<Diagnostic> {
    let graph = contract(entries);
    let mut out = Vec::new();
    for scc in tarjan_scc(&graph) {
        let cycle = match scc.len() {
            0 => continue,
            1 => {
                let node = scc[0];
                if graph.find_edge(node, node).is_some() {
                    vec![graph[node].to_string(), graph[node].to_string()]
                } else {
                    continue;
                }
            }
            _ => {
                let mut names: Vec<String> = scc.iter().map(|&n| graph[n].to_string()).collect();
                names.sort_unstable();
                let head = names[0].clone();
                names.push(head);
                names
            }
        };
        let pretty = cycle.join(" → ");
        out.push(finding(
            "publication-target-cycle",
            Severity::Important,
            format!("target-contraction cycle: {pretty}"),
            None,
        ));
    }
    out
}

fn contract(entries: &[Entry]) -> DiGraph<&str, ()> {
    let mut graph = DiGraph::new();
    let mut idx = HashMap::new();
    for entry in entries {
        let target = entry.target.as_str();
        idx.entry(target).or_insert_with(|| graph.add_node(target));
    }
    let by_name: HashMap<&str, &str> =
        entries.iter().map(|entry| (entry.name.as_str(), entry.target.as_str())).collect();
    for entry in entries {
        let Some(&to) = idx.get(entry.target.as_str()) else {
            continue;
        };
        for dep in &entry.depends_on {
            let Some(pred_target) = by_name.get(dep.as_str()) else {
                continue;
            };
            if *pred_target == entry.target.as_str() {
                continue;
            }
            if let Some(&from) = idx.get(pred_target) {
                graph.update_edge(from, to, ());
            }
        }
    }
    graph
}
