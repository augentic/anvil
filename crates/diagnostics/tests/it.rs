//! Consolidated integration binary for `specify-diagnostics`.
//!
//! Pure-logic coverage for the neutral diagnostic substrate: the `v1`
//! fingerprint algorithm (determinism plus which fields enter the
//! hash), and the triage predicates (`blocking`, severity tally,
//! `count_status`, `renumber`). These are deterministic, CLI-unreachable
//! invariants — the canonical unit carve-out. Each area is pulled in as
//! a `#[path]` submodule so the crate links exactly once; shared
//! fixtures live in `test_support`. See
//! [docs/standards/testing.md](../../../docs/standards/testing.md).

#[path = "test_support.rs"]
mod test_support;

#[path = "fingerprint.rs"]
mod fingerprint;
#[path = "report.rs"]
mod report;
