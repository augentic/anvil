//! The RFC-90 D4 bounded repair brief: a deterministic projection of
//! a canonical phase report for one `repair` dispatch. The complete
//! report remains gate and audit authority.

use diagnostics::{Diagnostic, is_blocking};

/// Maximum findings carried by one repair brief — an engine constant
/// (RFC-90 D4), never supplied by an adapter or model.
pub const REPAIR_BRIEF_LIMIT: usize = 16;

/// Project the deterministic repair brief from a canonical report's
/// findings.
///
/// Retains only blocking findings ([`is_blocking`]) in their
/// canonical order, then takes the first [`REPAIR_BRIEF_LIMIT`].
/// Never mutates or renumbers — briefed findings keep the ids and
/// fingerprints of their canonical report, so repair prose can cite
/// them against the persisted phase record.
#[must_use]
pub fn repair_brief(canonical: &[Diagnostic]) -> Vec<Diagnostic> {
    canonical
        .iter()
        .filter(|finding| is_blocking(finding))
        .take(REPAIR_BRIEF_LIMIT)
        .cloned()
        .collect()
}
