//! Framework-root adapter layout resolution (nested vs flattened).
//!
//! Two framework-root shapes ship today. Specify's own framework root
//! nests the adapter axes and codex pack under `adapters/`
//! (`adapters/sources/`, `adapters/targets/`, `adapters/codex/`). The
//! extracted `specify-adapters` framework root promotes them to the
//! repo root (`sources/`, `targets/`, `codex/`). These helpers give
//! the directory-walking checkers and fact extractors one definition of
//! "where the axes live" so every site agrees.
//!
//! Detection is structural: a root carrying an `adapters/` directory is
//! the nested shape; any other root is treated as flattened. This keeps
//! the nested shape (specify, the engine test fixtures, consumer
//! projects) byte-for-byte unchanged while letting a flattened root
//! resolve to its promoted axes.

use std::path::{Path, PathBuf};

/// The directory under which the adapter axes and codex pack live for
/// `project_dir`: `project_dir/adapters` for the nested shape, else
/// `project_dir` itself for the flattened shape.
fn adapters_root(project_dir: &Path) -> PathBuf {
    let nested = project_dir.join("adapters");
    if nested.is_dir() { nested } else { project_dir.to_path_buf() }
}

/// The axis directory (`sources` / `targets`) for `project_dir`,
/// resolving the nested-vs-flattened root shape.
#[must_use]
pub(super) fn framework_axis_dir(project_dir: &Path, axis: &str) -> PathBuf {
    adapters_root(project_dir).join(axis)
}

/// The codex-pack directory for `project_dir`, resolving the
/// nested-vs-flattened root shape.
#[must_use]
pub(super) fn framework_codex_dir(project_dir: &Path) -> PathBuf {
    adapters_root(project_dir).join("codex")
}

/// The top-level directories that hold adapter trees, for whole-tree
/// markdown walks: the single nested `adapters/` directory, or each
/// present flattened axis / codex directory. An empty result means the
/// root carries no adapter tree in either shape.
#[must_use]
pub(super) fn framework_adapter_roots(project_dir: &Path) -> Vec<PathBuf> {
    let nested = project_dir.join("adapters");
    if nested.is_dir() {
        return vec![nested];
    }
    ["sources", "targets", "codex"]
        .iter()
        .map(|axis| project_dir.join(axis))
        .filter(|dir| dir.is_dir())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mkdir(root: &Path, rel: &str) {
        std::fs::create_dir_all(root.join(rel)).expect("mkdir");
    }

    #[test]
    fn nested_root_resolves_under_adapters() {
        let dir = tempfile::tempdir().expect("tempdir");
        mkdir(dir.path(), "adapters/sources");
        mkdir(dir.path(), "adapters/targets");
        mkdir(dir.path(), "adapters/codex");
        assert_eq!(framework_axis_dir(dir.path(), "targets"), dir.path().join("adapters/targets"));
        assert_eq!(framework_codex_dir(dir.path()), dir.path().join("adapters/codex"));
        assert_eq!(framework_adapter_roots(dir.path()), vec![dir.path().join("adapters")]);
    }

    #[test]
    fn flattened_root_resolves_at_repo_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        mkdir(dir.path(), "sources");
        mkdir(dir.path(), "targets");
        mkdir(dir.path(), "codex");
        assert_eq!(framework_axis_dir(dir.path(), "targets"), dir.path().join("targets"));
        assert_eq!(framework_codex_dir(dir.path()), dir.path().join("codex"));
        assert_eq!(
            framework_adapter_roots(dir.path()),
            vec![dir.path().join("sources"), dir.path().join("targets"), dir.path().join("codex")]
        );
    }
}
