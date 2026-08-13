//! The exec-bit seam: the one mode distinction the snapshot contract
//! round-trips (the Git precedent, `100755` vs `100644`).

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::path::Path;

use error::Error;

/// Bulk exec-bit access beneath one tree root — one read per snapshot
/// walk, one apply per materialization.
///
/// Split from the tree walk because `wasi:filesystem` carries no
/// permission bits: the in-guest kernel reads and applies exec sets
/// through a host capability, while native deployments touch the
/// filesystem directly.
pub trait ExecBits: Debug + Send + Sync {
    /// The `/`-separated relative paths of executable regular files
    /// beneath `root`. Never follows symlinks. May skip paths the
    /// snapshot walk excludes (kernel excludes, `.gitignore` matches)
    /// as an optimization — the walk never queries paths it excludes,
    /// so the set is consulted as a lookup superset only.
    ///
    /// # Errors
    ///
    /// Filesystem failures.
    fn read(&self, root: &Path) -> Result<BTreeSet<String>, Error>;

    /// Set the executable bit (0o755) on `exec` and clear it (0o644)
    /// on `plain`, each `/`-separated relative to `root`.
    ///
    /// # Errors
    ///
    /// Filesystem failures.
    fn apply(&self, root: &Path, exec: &[String], plain: &[String]) -> Result<(), Error>;
}

/// Direct-filesystem exec bits for native deployments. A no-op on
/// platforms without unix mode bits, matching the manifest's
/// `exec: false` default there.
#[derive(Clone, Copy, Debug)]
pub struct FsExecBits;

impl ExecBits for FsExecBits {
    fn read(&self, root: &Path) -> Result<BTreeSet<String>, Error> {
        let mut set = BTreeSet::new();
        if cfg!(unix) {
            collect(root, "", &mut set, &super::Ignores::default())?;
        }
        Ok(set)
    }

    fn apply(&self, root: &Path, exec: &[String], plain: &[String]) -> Result<(), Error> {
        for path in exec {
            set_exec(&root.join(path), true)?;
        }
        for path in plain {
            set_exec(&root.join(path), false)?;
        }
        Ok(())
    }
}

/// Stat-only recursive walk folding executable file paths into `set`,
/// under the same admission as the content walk (kernel excludes plus
/// `.gitignore`) so ignored build trees are never descended.
fn collect(
    dir: &Path, prefix: &str, set: &mut BTreeSet<String>, ignores: &super::Ignores,
) -> Result<(), Error> {
    let ignores = ignores.descend(dir);
    for entry in crate::fs::dir_entries(dir)? {
        let name = entry.file_name();
        // Non-UTF-8 names are the content walk's typed refusal; the
        // exec walk just never claims them.
        let Some(name) = name.to_str() else {
            continue;
        };
        if super::store::IGNORED.contains(&name)
            || (prefix.is_empty() && super::store::IGNORED_ROOT.contains(&name))
        {
            continue;
        }
        let rel = if prefix.is_empty() { name.to_string() } else { format!("{prefix}/{name}") };
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            continue;
        }
        if ignores.excluded(&path, meta.is_dir()) {
            continue;
        }
        if meta.is_dir() {
            collect(&path, &rel, set, &ignores)?;
        } else if is_exec(&meta) {
            set.insert(rel);
        }
    }
    Ok(())
}

#[cfg(unix)]
fn is_exec(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    meta.permissions().mode() & 0o100 != 0
}

#[cfg(not(unix))]
fn is_exec(_meta: &std::fs::Metadata) -> bool {
    false
}

#[cfg(unix)]
fn set_exec(path: &Path, exec: bool) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt as _;
    // Never chmod through a symlink: `set_permissions` follows links,
    // and a link here would reach a target outside the tree. Manifests
    // only name regular files in their mode sets.
    if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(Error::Diag {
            code: "workspace-path-unsupported",
            detail: format!("`{}` is a symlink; exec bits apply to regular files", path.display()),
        });
    }
    let mode = if exec { 0o755 } else { 0o644 };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_exec(_path: &Path, _exec: bool) -> Result<(), Error> {
    Ok(())
}
