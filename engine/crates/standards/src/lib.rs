//! Specify standards layer — rule parser, resolver, and lint engine.
//!
//! Per the standards-layer dependency invariant, this crate is a standards-layer
//! sibling of `specify-workflow`: it carries every rules/lint DTO, the CH-11
//! frontmatter parser, the CH-12/13/14 resolver pipeline, and the CH-16
//! hint interpreter. The structured diagnostic currency
//! ([`specify_diagnostics::Diagnostic`], renderers, fingerprint) lives in
//! the neutral [`specify_diagnostics`] leaf — import it directly rather
//! than through this crate.
//!
//! **Crate name vs modules:** `specify-standards` is the whole standards layer
//! (policy resolution plus the `specify lint` scanner). The [`rules`]
//! module owns parsing and `specify rules export`; [`lint`] owns indexing
//! and hint evaluation — not a separate `specify-rules` crate.
//!
//! The internal module shape preserves the rules-vs-lint split:
//! the [`rules`] umbrella wraps the parser and resolver so paths like
//! `specify_standards::rules::parse` stay stable for downstream consumers.
//! Rule and resolver DTOs re-export at the crate root via `pub use rules::*`.
//! Import diagnostic types from [`specify_diagnostics`] directly.

pub mod lint;
pub mod rules;

pub use rules::*;
