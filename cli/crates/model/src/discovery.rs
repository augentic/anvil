//! Discovery surface — the `## Lead inventory` blocks in `discovery.md`.
//!
//! Each block is a raw, unmerged per-source lead a source adapter emits
//! at `survey` time, identified by its `(source, lead)` pair and
//! validated against `schemas/discovery/lead.schema.json`.
//!
//! The whole-document model lives in [`document`]; it parses
//! `discovery.md` and exposes [`Discovery::resolve_lead`] for the
//! `--sources <key>=<lead>` rewrite path.

pub mod document;
pub mod lead;

pub use document::{Discovery, ResolveError as DiscoveryResolveError};
pub use lead::Lead;
