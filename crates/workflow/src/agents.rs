//! Init-time `AGENTS.md` context-fence generation.
//!
//! Pure, dependency-light logic for the fenced `AGENTS.md` context block:
//! shallow root-marker [`detect`]ion, deterministic Markdown [`render`]ing,
//! byte-preserving [`fences`] parsing and write planning, input
//! [`fingerprint`]ing, and the [`lock`] sidecar. The init-time context
//! generation in `crate::init` assembles a [`render::Input`] from the
//! freshly scaffolded project and drives these modules.
pub mod detect;
pub mod fences;
pub mod fingerprint;
pub mod lock;
pub mod render;
