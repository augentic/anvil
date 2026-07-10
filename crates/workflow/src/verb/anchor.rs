//! [`Anchor`] — the provider-supplied project anchoring every
//! project-scoped verb loads its [`crate::verb::Ctx`] from.

use std::path::Path;

/// Project anchoring carried on the provider value.
///
/// The guest provider answers `"."` (the project-root mount preopen);
/// a native provider answers its configured root. Verbs never read the
/// process CWD themselves — the anchor is the single source of the
/// project location, so the same `Handler` impl serves the wasm guest,
/// the native dev shim, and tests against a tempdir.
///
/// [`omnia_guest::api::Provider`] is a supertrait so every
/// anchor-bearing provider satisfies the `Handler<P: Provider>` bound
/// without a second annotation.
pub trait Anchor: omnia_guest::api::Provider {
    /// Directory the project-root walk starts from.
    fn project_root(&self) -> &Path;

    /// Plan root override (the `--plan-dir` global): the directory
    /// holding the governing `plan.yaml` when it is not the project
    /// root. `None` anchors plan artifacts at the project root.
    fn plan_dir(&self) -> Option<&Path> {
        None
    }
}
