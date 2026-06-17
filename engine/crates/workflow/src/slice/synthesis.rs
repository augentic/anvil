//! Slice synthesis projection kernel.
//!
//! The agent owns cross-modal reconciliation — which requirements
//! exist and how claims merge or split. Everything the CLI can make
//! deterministic around that judgment is a pure projection over the
//! agent's returned structure: authority resolution, status
//! derivation, and per-claim winner markers. The kernel consumes data the
//! caller has already read from disk — Evidence authority classes, the
//! per-slice override, and the agent's agreement verdict — and never
//! performs I/O of its own.

pub mod authority;
pub mod project;
pub mod render;
pub mod wire;
