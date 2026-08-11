//! The RFC-90 D2 phase-finding canonicalizer: stamp, recompute
//! fingerprints, deduplicate, sort by the closed key, renumber.
//! Byte-stable across input permutations.

use diagnostics::{Diagnostic, Severity, is_blocking, renumber};

/// Engine identity stamped onto every canonicalized finding.
#[derive(Debug, Clone, Copy)]
pub struct Stamp<'a> {
    /// Target-adapter name the phase dispatched to.
    pub target_adapter: &'a str,
    /// Slice the build serves.
    pub slice: &'a str,
    /// Change identity when known.
    pub change: Option<&'a str>,
}

/// Canonicalize `findings` per RFC-90 D2.
///
/// # Algorithm
///
/// 1. Stamp `target_adapter`, `slice`, and `change` onto every
///    finding.
/// 2. Recompute the fingerprint over the stamped finding and always
///    overwrite the supplied value — verification **is**
///    recomputation, so a mismatched adapter-supplied fingerprint is
///    simply replaced, never rejected.
/// 3. Group by fingerprint and retain one representative per group:
///    blocking findings first (a blocking representative is never
///    displaced by a non-blocking twin), then strongest severity
///    ([`Severity::Critical`] strongest); a remaining tie breaks on
///    the lexicographically least JSON of the complete stamped
///    finding with `id` omitted (`serde_json` orders object keys
///    deterministically for a given build, which is all a stable
///    tie-break needs).
/// 4. Sort representatives by `(severity rank, location presence,
///    path, line, column, fingerprint)` — located findings precede
///    unlocated ones, and a missing line/column follows every
///    concrete value.
/// 5. Renumber report-local ids via [`renumber`].
#[must_use]
pub fn canonicalize(findings: Vec<Diagnostic>, stamp: &Stamp<'_>) -> Vec<Diagnostic> {
    let mut groups: std::collections::BTreeMap<String, Diagnostic> =
        std::collections::BTreeMap::new();
    for mut finding in findings {
        finding.target_adapter = Some(stamp.target_adapter.to_string());
        finding.slice = Some(stamp.slice.to_string());
        finding.change = stamp.change.map(str::to_string);
        finding.fingerprint = diagnostics::fingerprint(&finding);

        match groups.entry(finding.fingerprint.clone()) {
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(finding);
            }
            std::collections::btree_map::Entry::Occupied(mut slot) => {
                if prefer(&finding, slot.get()) {
                    slot.insert(finding);
                }
            }
        }
    }

    let mut representatives: Vec<Diagnostic> = groups.into_values().collect();
    representatives.sort_by_key(sort_key);
    renumber(&mut representatives);
    representatives
}

/// Whether `candidate` replaces `incumbent` as its fingerprint
/// group's representative: blocking over non-blocking first (a
/// non-blocking twin must never flip a blocking report to passing),
/// then strictly stronger severity, then the lexicographically
/// lesser id-less JSON.
fn prefer(candidate: &Diagnostic, incumbent: &Diagnostic) -> bool {
    match is_blocking(candidate).cmp(&is_blocking(incumbent)) {
        std::cmp::Ordering::Greater => return true,
        std::cmp::Ordering::Less => return false,
        std::cmp::Ordering::Equal => {}
    }
    match candidate.severity.cmp(&incumbent.severity) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => tie_break_json(candidate) < tie_break_json(incumbent),
    }
}

/// The same-severity tie-break key: the finding serialized to JSON
/// with `id` removed.
///
/// Renumbering happens after selection, so `id` must not perturb
/// which representative survives.
fn tie_break_json(finding: &Diagnostic) -> String {
    let mut value = serde_json::to_value(finding)
        .unwrap_or_else(|_| unreachable!("a Diagnostic is infallibly serialisable"));
    if let Some(object) = value.as_object_mut() {
        object.remove("id");
    }
    serde_json::to_string(&value)
        .unwrap_or_else(|_| unreachable!("a JSON value is infallibly serialisable"))
}

/// The closed D2 sort key.
///
/// `Severity`'s derived [`Ord`] is the documented rank (`Critical <
/// Important < Suggestion < Optional`); located findings sort before
/// unlocated ones; a missing line or column sorts after every
/// concrete value.
fn sort_key(finding: &Diagnostic) -> (Severity, u8, String, (u8, u32), (u8, u32), String) {
    let (presence, path, line, column) = finding.location.as_ref().map_or_else(
        || (1, String::new(), (1, 0), (1, 0)),
        |location| {
            (0, location.path.clone(), option_last(location.line), option_last(location.column))
        },
    );
    (finding.severity, presence, path, line, column, finding.fingerprint.clone())
}

/// Order an optional coordinate with `None` after every `Some`.
const fn option_last(value: Option<u32>) -> (u8, u32) {
    match value {
        Some(concrete) => (0, concrete),
        None => (1, 0),
    }
}
