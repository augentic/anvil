//! Adapter manifest resolution.
//!
//! workflow §"Adapter implementation shape" / §"Resolver and cache".
//! Source and target adapters share the `adapter.yaml` wire shape but
//! split into [`SourceAdapter`] / [`TargetAdapter`] in memory, each
//! carrying its closed operation set ([`SourceOperation`] /
//! [`TargetOperation`]) derived from the closed WIT contract
//! (`wit/specify.wit`). See [DECISIONS.md §"Operations typed at parse
//! boundary"] for the rationale.
//!
//! Resolution is path-agnostic: each axis-specific loader probes
//! `<project-cache>/manifests/{sources,targets}/<name>/`
//! first (the agent-populated out-of-tree manifest cache) and then
//! `<project_dir>/adapters/{sources,targets}/<name>/` (in-repo). The
//! manifest cache mirrors the in-repo adapter tree so source and
//! target adapters with colliding names disambiguate by axis. See
//! [DECISIONS.md §"Cache layout"].
//!
//! Brief bodies are read in-guest by each adapter's own workflow
//! guest; the CLI never parses brief markdown.
//!
//! [DECISIONS.md §"Operations typed at parse boundary"]: ../../../DECISIONS.md#operations-typed-at-parse-boundary
//! [DECISIONS.md §"Cache layout"]: ../../../DECISIONS.md#cache-layout

mod core;
pub(crate) mod operation;
mod resolve;
mod validate_manifest;

pub use core::{
    ADAPTER_FILENAME, ADAPTER_GUEST_FILENAME, ADAPTERS_DIR, AdapterLocation, AdapterRef, Axis,
    BuildInputDeclaration, PlatformsCapability, PlatformsViolation, ResolvedTargetAdapter,
    SourceAdapter, TargetAdapter, adapter_axis_dir, cache_axis_dir, cache_dir,
};

pub use operation::{SourceOperation, TargetOperation};
pub use validate_manifest::check_axis_unique_for_name;
