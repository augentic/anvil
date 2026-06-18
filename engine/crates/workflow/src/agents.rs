//! Init-time `AGENTS.md` context-fence generation.
//!
//! Pure, dependency-light logic for the fenced `AGENTS.md` context block:
//! shallow root-marker [`detect`]ion, deterministic Markdown [`render`]ing,
//! byte-preserving [`fences`] parsing and write planning, input
//! [`fingerprint`]ing, and the [`lock`] sidecar. The binary's `agents`
//! command assembles a [`render::Input`] from its `Ctx` and drives these
//! modules; everything here is `Ctx`-free so it can carry its own unit tests
//! (per `docs/standards/testing.md`).
//!
//! ## Lint posture
//!
//! This code originated in the binary crate
//! (`src/runtime/commands/agents`), where exported-item lints never applied —
//! a binary exports no public API, so `missing_docs` on `pub` fields,
//! `must_use_candidate`, `missing_panics_doc`, and similar `pedantic` /
//! `nursery` checks never fired. It moved verbatim to host its unit
//! tests; the module-scoped allow preserves that pre-move posture rather than
//! churning ~30 field-doc comments and `#[must_use]` / `# Panics` attributes
//! onto relocated internals.
#![allow(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    clippy::pedantic,
    clippy::nursery,
    reason = "relocated binary-internal context-fence code retains its pre-move lint posture; a binary exports nothing, so exported-item lints never applied. See module docs."
)]

pub mod detect;
pub mod fences;
pub mod fingerprint;
pub mod lock;
pub mod render;
