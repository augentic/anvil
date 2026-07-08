//! Consolidated integration binary for `error`.
//!
//! Pure-logic edge matrices that are CLI-unreachable: the kebab-case
//! validators, the `Error` discriminant/`Display` contract, the
//! `WIRE_CODES` table invariants, and the RFC 3339 serde helpers. Each
//! former area is pulled in here as a `#[path]` submodule so the crate
//! links exactly once. See
//! [docs/standards/testing.md](../../../docs/standards/testing.md).

#![allow(clippy::too_many_lines, reason = "edge-matrix test fns are intentionally long")]

#[path = "codes.rs"]
mod codes;
#[path = "error.rs"]
mod error;
#[path = "kebab.rs"]
mod kebab;
#[path = "serde_time.rs"]
mod serde_time;
