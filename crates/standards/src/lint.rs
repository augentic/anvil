//! Review surface for the deterministic lint layer.
//!
//! Sibling umbrella to [`crate::rules`]: this module owns the
//! `specify lint` deterministic review pipeline — the `WorkspaceModel`
//! DTOs ([`model`], pinned by
//! `schemas/lint/workspace-model.schema.json`), the consumer /
//! framework indexer ([`index`]), the hint interpreter ([`eval`]), and
//! the diagnostic formatters ([`diagnostics`]).
//!
//! Only the [`model`] surface is re-exported at the umbrella root.
//! [`index`], [`eval`], and [`diagnostics`] stay reachable only by
//! their fully-qualified path so the `rules` (authoring) and
//! `review` (enforcement) surfaces cannot collide.

pub mod diagnostics;
pub mod eval;
mod framework_tools;
pub mod ignore;
pub mod index;
mod layout;
pub mod model;
pub mod runner;

pub use model::*;
