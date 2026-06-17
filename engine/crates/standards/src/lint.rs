//! Review surface for the deterministic lint layer.
//!
//! Cross-references: the standards-layer split lives in
//! [DECISIONS.md § Standards layer split into `specify-standards` and `specify-schema`](../../DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema);
//! the `WorkspaceModel` envelope is pinned by
//! `schemas/lint/workspace-model.schema.json` (`WORKSPACE_MODEL_JSON_SCHEMA`).
//!
//! Sibling umbrella to [`crate::rules`]: this module owns the
//! `specify lint` deterministic review pipeline — `WorkspaceModel` DTOs,
//! the consumer / framework indexer, the hint interpreter, and the
//! diagnostic formatters that `specify lint` and (later)
//! `specify lint framework --format json` share.
//!
//! The submodule shape mirrors the the standards-layer dependency invariant sketch.
//! v1 ships the [`model`] DTO layer; [`index`], [`eval`], and
//! [`diagnostics`] are placeholders filled in by later standards-layer
//! implementation slices.
//!
//! Only the [`model`] surface is re-exported at the umbrella root.
//! [`index`], [`eval`], and [`diagnostics`] stay reachable only by
//! their fully-qualified path so the `rules` (authoring) and
//! `review` (enforcement) surfaces cannot collide.

pub mod adapter_briefs;
pub mod contract;
pub mod diagnostics;
pub mod eval;
mod framework_tools;
pub mod ignore;
pub mod index;
pub mod model;
pub mod runner;

pub use model::*;
