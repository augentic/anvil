//! [`Anchor`] — the provider-supplied project anchoring every
//! project-scoped verb loads its [`crate::handler::Ctx`] from.

use std::path::Path;

use super::ExecutionPaths;

/// Project anchoring carried on the provider value.
///
/// The guest provider answers `"."` (the project-root mount preopen);
/// a native provider answers its configured root. Verbs never read the
/// process CWD themselves — the anchor is the single source of the
/// project location, so the same operation serves the wasm guest,
/// native providers, and tests against a tempdir. The carried
/// [`ExecutionPaths`] also fixes cache placement: an isolated value
/// pins an explicit cache parent instead of mutating process
/// environment.
///
/// [`omnia_guest::api::Provider`] is a supertrait so every
/// anchor-bearing provider satisfies the `Operation<P: Provider>` bound
/// without a second annotation.
pub trait Anchor: omnia_guest::api::Provider {
    /// Project root plus cache placement.
    fn paths(&self) -> &ExecutionPaths;

    /// Directory the project-root walk starts from.
    fn project_root(&self) -> &Path {
        self.paths().project_root()
    }
}
