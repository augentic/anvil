//! Consolidated integration binary for `specify-model`.
//!
//! One binary per crate: each `tests/<area>.rs` is pulled in here as a
//! `#[path]` submodule so the crate-under-test links exactly once. The
//! parser edge-matrices (`spec`, `task`, `discovery`, `decision`,
//! `evidence`) and the atomic writer exercise `specify-model`'s public
//! API. See [docs/standards/testing.md](../../../docs/standards/testing.md).

#[path = "atomic.rs"]
mod atomic;
#[path = "decision.rs"]
mod decision;
#[path = "discovery_document.rs"]
mod discovery_document;
#[path = "discovery_lead.rs"]
mod discovery_lead;
#[path = "evidence_authority.rs"]
mod evidence_authority;
#[path = "evidence_example.rs"]
mod evidence_example;
#[path = "spec.rs"]
mod spec;
#[path = "spec_provenance.rs"]
mod spec_provenance;
#[path = "task.rs"]
mod task;
#[path = "validate.rs"]
mod validate;
