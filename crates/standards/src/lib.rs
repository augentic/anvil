//! Specify standards layer — rule parser and resolver.
//!
//! Per the standards-layer dependency invariant, this crate is a
//! standards-layer sibling of `workflow`: it carries the rule
//! DTOs, the rule frontmatter parser, and the resolver pipeline behind
//! `specify rules export`. The structured diagnostic currency
//! ([`diagnostics::Diagnostic`], renderers, fingerprint) lives in
//! the neutral [`diagnostics`] leaf — import it directly rather
//! than through this crate.
//!
//! The [`rules`] umbrella wraps the parser and resolver so paths like
//! `standards::rules::parse` stay stable for downstream consumers.
//! Rule and resolver DTOs re-export at the crate root via `pub use rules::*`.
//! Import diagnostic types from [`diagnostics`] directly.

pub mod rules;

pub use rules::*;
