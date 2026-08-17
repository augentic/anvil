//! Project-root and change-home anchoring for one invocation.

use std::path::{Path, PathBuf};

use project::config::Roots;
use transport::command::selectors::{SeedRequest, SystemRequest};

/// Resolve in-place vs detached roots.
///
/// `adapter add --project-dir` always selects that product tree
/// (in-place, even before init); otherwise [`Roots::resolve`] over
/// `--change-dir` and the ancestor walk for `.emery/project.yaml`.
///
/// A resolution miss (no ancestor, no `--change-dir`) anchors in-place
/// at the invocation directory — pre-init in-place, exactly like the
/// seed path: `emery init` and `adapter add` stay legal, and every
/// change verb fails typed in-guest (`not-initialized`) instead of the
/// working directory being silently treated as a detached change home
/// (D2). The policy stays total so the guest renders the diagnostic.
#[must_use]
pub fn roots(invoked_dir: &Path, seed: Option<&SeedRequest>, change_dir: Option<&Path>) -> Roots {
    if let Some(dir) = seed.and_then(|request| request.project_dir.as_ref()) {
        let product = if dir.is_absolute() { dir.clone() } else { invoked_dir.join(dir) };
        return Roots::InPlace { product };
    }
    Roots::resolve(invoked_dir, change_dir).unwrap_or_else(|_unanchored| Roots::InPlace {
        product: invoked_dir.to_path_buf(),
    })
}

/// Definition-home mount for a `system *` invocation: `--dir` else CWD,
/// never a `project.yaml` walk.
#[must_use]
pub fn system_root(invoked_dir: &Path, system: &SystemRequest) -> PathBuf {
    system.root(invoked_dir)
}
