//! Runtime ownership-overlap authoring. Writes an inert proposal and
//! never applies it.

use std::collections::BTreeSet;

use error::Error;

use super::{Frontiers, Ownership, Proposal, Repair, VERSION};
use crate::config::Layout;
use crate::journal::{EventKind, claim};
use crate::name::SliceName;
use crate::plan::decomposition::{Decomposition, compile};
use crate::plan::execution::collect_events;
use crate::plan::model::Plan;
use crate::snapshot::SnapshotId;

/// Detect in-flight overlapping leaves and persist one ownership
/// proposal when none already covers the pair.
///
/// Returns the new or existing digest. `None` when no overlap.
///
/// # Errors
///
/// Frontier snapshot, decomposition load, or persist failures.
pub fn author(layout: Layout<'_>, plan: &Plan) -> Result<Option<SnapshotId>, Error> {
    let Some(tree) = Decomposition::load_opt(&layout.decomposition_path())? else {
        return Ok(None);
    };
    let events = collect_events(layout)?;
    let in_flight = in_flight(&events);
    if in_flight.len() < 2 {
        return Ok(None);
    }
    let Some((left, right, nearest)) = first_pair(&tree, &in_flight)? else {
        return Ok(None);
    };
    if let Some(digest) = existing(layout, &left, &right)? {
        return Ok(Some(digest));
    }
    let expected = Frontiers::live(layout, plan)?;
    let proposal = Proposal::Ownership(Ownership {
        version: VERSION,
        nearest,
        repair: Repair::DependsOn {
            predecessor: left,
            successor: right,
        },
        expected,
    });
    Ok(Some(proposal.save(layout)?))
}

fn in_flight(events: &[crate::journal::Event]) -> BTreeSet<SliceName> {
    let mut claimed: BTreeSet<SliceName> =
        claim::project(events).iter().map(|(slice, _)| slice.clone()).collect();
    let mut opened = BTreeSet::new();
    let mut done = BTreeSet::new();
    for event in events {
        match &event.kind {
            EventKind::TargetWaveOpened { slice_name, .. } => {
                opened.insert(slice_name.clone());
            }
            EventKind::TargetMergeWaveCommitted { members, .. } => {
                done.extend(members.iter().cloned());
            }
            _ => {}
        }
    }
    claimed.extend(opened.difference(&done).cloned());
    claimed
}

fn first_pair(
    tree: &Decomposition, in_flight: &BTreeSet<SliceName>,
) -> Result<Option<(SliceName, SliceName, String)>, Error> {
    let edges = compile(tree)?;
    let mut names: Vec<SliceName> = in_flight.iter().cloned().collect();
    names.sort();
    for (index, left) in names.iter().enumerate() {
        let Ok(left_id) = tree.leaf_id(left.as_str()) else {
            continue;
        };
        let left_own: BTreeSet<&str> =
            tree.node(left_id)?.ownership.iter().map(String::as_str).collect();
        for right in names.iter().skip(index + 1) {
            let Ok(right_id) = tree.leaf_id(right.as_str()) else {
                continue;
            };
            let right_own: BTreeSet<&str> =
                tree.node(right_id)?.ownership.iter().map(String::as_str).collect();
            if !globs_overlap(&left_own, &right_own) {
                continue;
            }
            if ordered(&edges, left, right) {
                continue;
            }
            let nearest = nearest(tree, left_id, right_id)?;
            return Ok(Some((left.clone(), right.clone(), nearest)));
        }
    }
    Ok(None)
}

fn ordered(
    edges: &std::collections::BTreeMap<SliceName, Vec<SliceName>>, left: &SliceName,
    right: &SliceName,
) -> bool {
    reaches(edges, left, right) || reaches(edges, right, left)
}

fn reaches(
    edges: &std::collections::BTreeMap<SliceName, Vec<SliceName>>, from: &SliceName, to: &SliceName,
) -> bool {
    let mut seen = BTreeSet::new();
    let mut stack = vec![from];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        if id == to && id != from {
            return true;
        }
        if let Some(next) = edges.get(id) {
            stack.extend(next.iter());
        }
    }
    false
}

fn nearest(tree: &Decomposition, left: &str, right: &str) -> Result<String, Error> {
    let left_anc = tree.ancestry(left)?;
    let right_anc = tree.ancestry(right)?;
    let mut shared = tree.root.clone();
    for (a, b) in left_anc.iter().zip(right_anc.iter()) {
        if a == b {
            shared.clone_from(a);
        } else {
            break;
        }
    }
    Ok(shared)
}

fn globs_overlap(left: &BTreeSet<&str>, right: &BTreeSet<&str>) -> bool {
    left.iter().any(|a| right.iter().any(|b| glob_overlap(a, b)))
}

fn glob_overlap(left: &str, right: &str) -> bool {
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

fn existing(
    layout: Layout<'_>, left: &SliceName, right: &SliceName,
) -> Result<Option<SnapshotId>, Error> {
    let events = collect_events(layout)?;
    for (digest, proposal) in Proposal::load_all(layout)? {
        if super::is_applied(&events, &digest) {
            continue;
        }
        if let Proposal::Ownership(body) = proposal
            && let Repair::DependsOn {
                predecessor,
                successor,
            } = &body.repair
        {
            let pair = (predecessor == left && successor == right)
                || (predecessor == right && successor == left);
            if pair {
                return Ok(Some(digest));
            }
        }
    }
    Ok(None)
}
