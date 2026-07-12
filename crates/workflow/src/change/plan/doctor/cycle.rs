//! Cycle detection for the `cycle-in-depends-on` diagnostic.

use petgraph::algo::tarjan_scc;
use schema::diagnostics::{Diagnostic, Severity};

use super::CYCLE;
use crate::change::plan::core::Entry;
use crate::change::plan::core::validate::{dependency_graph, structured_finding};

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
    let graph = dependency_graph(changes);

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
        out.push(structured_finding(
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
