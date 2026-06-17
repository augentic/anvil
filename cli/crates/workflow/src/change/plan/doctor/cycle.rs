//! Cycle detection for the `cycle-in-depends-on` diagnostic.

use petgraph::algo::tarjan_scc;
use specify_diagnostics::{Diagnostic, Severity};

use super::CYCLE;
use crate::change::plan::core::Entry;
use crate::change::plan::core::validate::{entry_dependency_graph, plan_finding_structured};

/// One [`CYCLE`] diagnostic per cycle in the depends-on graph.
///
/// Self-loops are emitted too. Cycles are deduplicated by sorted
/// node-set so every distinct cycle surfaces exactly once. The cycle
/// path is sorted alphabetically with the first node repeated at the
/// end — matches the convention used by the doctor message text — and
/// is carried verbatim on the diagnostic's structured evidence under
/// `cycle`.
#[must_use]
pub fn detect(changes: &[Entry]) -> Vec<Diagnostic> {
    let graph = entry_dependency_graph(changes);

    let mut out = Vec::new();
    for scc in tarjan_scc(&graph) {
        let cycle_names: Vec<String> = match scc.len() {
            0 => continue,
            1 => {
                let n = scc[0];
                if graph.find_edge(n, n).is_some() {
                    vec![graph[n].to_string(), graph[n].to_string()]
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
        let pretty = cycle_names.join(" → ");
        out.push(plan_finding_structured(
            CYCLE,
            Severity::Important,
            format!("dependency cycle: {pretty}"),
            None,
            "dependency cycle",
            serde_json::json!({ "cycle": cycle_names }),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::detect;
    use crate::change::plan::core::{Status, change_with_deps};

    fn node_names(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("n{i}")).collect()
    }

    proptest! {
        // Edges that only point from an earlier node to a later one form
        // a DAG by construction — the detector must never flag it.
        #[test]
        fn dag_never_flagged(
            adj in prop::collection::vec(prop::collection::vec(any::<bool>(), 6), 2..6),
        ) {
            let n = adj.len();
            let names = node_names(n);
            let entries: Vec<_> = (0..n)
                .map(|i| {
                    let deps: Vec<&str> =
                        (0..i).filter(|&j| adj[i][j]).map(|j| names[j].as_str()).collect();
                    change_with_deps(&names[i], Status::Pending, &deps)
                })
                .collect();
            prop_assert!(detect(&entries).is_empty());
        }

        // A directed ring n0 → n1 → … → n0 always contains a cycle.
        #[test]
        fn ring_always_flagged(n in 2_usize..6) {
            let names = node_names(n);
            let entries: Vec<_> = (0..n)
                .map(|i| {
                    let dep = names[(i + 1) % n].as_str();
                    change_with_deps(&names[i], Status::Pending, &[dep])
                })
                .collect();
            prop_assert!(!detect(&entries).is_empty());
        }

        // A self-dependency is a one-node cycle and must be flagged.
        #[test]
        fn self_loop_flagged(n in 1_usize..6, at in 0_usize..6) {
            let names = node_names(n);
            let target = at % n;
            let entries: Vec<_> = (0..n)
                .map(|i| {
                    let deps: Vec<&str> =
                        if i == target { vec![names[i].as_str()] } else { vec![] };
                    change_with_deps(&names[i], Status::Pending, &deps)
                })
                .collect();
            prop_assert!(!detect(&entries).is_empty());
        }
    }
}
