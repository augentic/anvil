//! Deterministic fingerprint probes over a staged source tree.

use std::fs;
use std::path::Path;

use error::Error;

/// Directory names skipped while walking a staged source value.
const SKIP: &[&str] = &[".git", ".emery", "node_modules", "target", "dist", "build"];

/// Manifest and extension probes for one source adapter.
///
/// A profile matches when any root-relative `paths` entry exists or
/// any walked file carries a listed extension or basename. Declared
/// starting probes, not a ranking.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Profile {
    /// Root-relative files or directories (`package.json`,
    /// `tests/data/replays`). Slash-free names also match any walked
    /// basename.
    pub paths: Vec<String>,
    /// File extensions without a leading dot (`ts`, `md`, `png`).
    pub extensions: Vec<String>,
}

impl Profile {
    /// Whether `root` satisfies this profile.
    ///
    /// # Errors
    ///
    /// Filesystem failures while walking `root`.
    pub fn matches(&self, root: &Path) -> Result<bool, Error> {
        if self.paths.iter().any(|path| root.join(path).exists()) {
            return Ok(true);
        }
        if !root.exists() {
            return Ok(false);
        }
        walk(root, self)
    }
}

fn walk(root: &Path, profile: &Profile) -> Result<bool, Error> {
    if file_hits(root, profile) {
        return Ok(true);
    }
    if !root.is_dir() {
        return Ok(false);
    }
    let entries = fs::read_dir(root).map_err(|source| Error::Filesystem {
        op: "readdir",
        path: root.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Filesystem {
            op: "readdir",
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(|source| Error::Filesystem {
            op: "stat",
            path: path.clone(),
            source,
        })?;
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            let name = entry.file_name();
            if SKIP.iter().any(|skip| name == *skip) {
                continue;
            }
            if walk(&path, profile)? {
                return Ok(true);
            }
        } else if file_hits(&path, profile) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn file_hits(path: &Path, profile: &Profile) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if profile.paths.iter().any(|probe| !probe.contains('/') && probe == name) {
        return true;
    }
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    let ext = ext.to_ascii_lowercase();
    profile.extensions.iter().any(|probe| probe.eq_ignore_ascii_case(&ext))
}
