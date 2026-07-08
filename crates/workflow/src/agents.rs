//! Init-time `AGENTS.md` context-fence generation.
//!
//! Pure, dependency-light logic for the fenced `AGENTS.md` context block:
//! shallow root-marker [`detect`]ion, deterministic Markdown [`render`]ing,
//! byte-preserving [`fences`] parsing and write planning, input
//! [`fingerprint`]ing, and the [`lock`] sidecar. The binary's `agents`
//! command assembles a [`render::Input`] from its `Ctx` and drives these
//! modules; everything here is `Ctx`-free so it can carry its own unit tests
//! (per `docs/standards/testing.md`).
#![allow(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    clippy::pedantic,
    clippy::nursery,
    reason = "binary-internal context-fence code consumed only by the `agents` command; documenting ~30 internal fields and `#[must_use]` / `# Panics` attributes adds noise, not API surface"
)]

pub mod detect;
pub mod fences;
pub mod fingerprint;
pub mod lock;
pub mod render;
