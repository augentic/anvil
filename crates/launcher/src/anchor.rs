//! Project-root anchoring for one invocation.

use std::path::Path;

use project::config::Roots;

/// Resolve the invocation's roots: [`Roots::resolve`] over the
/// ancestor walk for `.emery/project.yaml`. A resolution miss anchors
/// in-place at the invocation directory — pre-init, so `emery init`
/// stays legal and later verbs fail typed in-guest
/// (`not-initialized`).
#[must_use]
pub fn roots(invoked_dir: &Path) -> Roots {
    Roots::resolve(invoked_dir, None).unwrap_or_else(|_unanchored| Roots::InPlace {
        product: invoked_dir.to_path_buf(),
    })
}
