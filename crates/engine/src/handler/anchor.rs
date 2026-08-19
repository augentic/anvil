//! [`Anchor`] — the provider-supplied project anchoring every
//! project-scoped verb loads its [`crate::handler::Ctx`] from.

use std::path::Path;

use super::ExecutionPaths;

/// Project anchoring carried on the provider value.
///
/// Verbs never read the process CWD — the anchor is the single source
/// of the project location, so the same operation serves the wasm
/// guest, native providers, and tests. The carried [`ExecutionPaths`]
/// also fixes cache placement without mutating process environment.
/// [`omnia_guest::api::Provider`] is a supertrait so anchor-bearing
/// providers satisfy the `Operation<P: Provider>` bound directly.
pub trait Anchor: omnia_guest::api::Provider {
    /// Project root plus cache placement.
    fn paths(&self) -> &ExecutionPaths;

    /// Directory the project-root walk starts from.
    fn project_root(&self) -> &Path {
        self.paths().project_root()
    }
}
