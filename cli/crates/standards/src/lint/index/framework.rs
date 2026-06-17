//! Framework scan profile walker per the standards-layer contract §F1.
//!
//! Symmetric counterpart to [`super::files`]: walks the framework
//! repository (`augentic/specify`) with a wider include set, a
//! follow-the-link symlink policy, and cycle detection. Recognised
//! roots: `adapters/**`, `plugins/**`, `docs/**`, `.cursor/**`,
//! `rfcs/**`, `scripts/**`, `schemas/**`, plus the catch-all
//! `**/AGENTS.md` and `**/REVIEW.md` files and the repo-root
//! `README.md`.
//!
//! Discovery returns a [`FrameworkDiscovery`] payload carrying the
//! sorted file set and the recorded [`Symlink`] facts (with
//! `resolved_target` populated under §F1's follow mode). Cycle
//! detection uses `std::fs::canonicalize` to collapse equivalent
//! endpoints; the walker emits [`IndexError::Filesystem`] on revisit
//! and aborts the walk for that subtree.

use std::path::{MAIN_SEPARATOR, Path, PathBuf};

use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;

use super::files::DiscoveredFile;
use super::languages::infer_language;
use super::symlinks::FollowMode;
use super::{IndexError, symlinks};
use crate::lint::{FileKind, Symlink};

/// First 8 `KiB` window scanned for NUL bytes per §F1.
const BINARY_SNIFF_BYTES: usize = 8 * 1024;

/// Files + symlinks recorded during a single framework walk pass.
#[derive(Debug, Default)]
pub struct FrameworkDiscovery {
    /// Text + binary files discovered under the framework roots
    /// (with follow-mode traversal per §F1).
    pub files: Vec<DiscoveredFile>,
    /// Symlinks discovered under the framework roots. Under follow
    /// mode `Symlink::resolved_target` holds the canonical endpoint.
    pub symlinks: Vec<Symlink>,
}

/// Walk the framework tree rooted at `project_dir`.
///
/// `project_dir` is always the framework-repo root. `roots`, when
/// non-empty, narrows the walk to project-relative roots (e.g.
/// `--artifact-path adapters/sources/intent`); an empty list means
/// walk the full framework include set per §F1. `languages` filters
/// the discovered file set to a language token allow-list when
/// non-empty.
///
/// # Errors
///
/// - [`IndexError::ProjectDirMissing`] when `project_dir` is not a
///   directory.
/// - [`IndexError::OverrideCompile`] when the static always-ignore
///   glob list fails to compile (programmer error).
/// - [`IndexError::Filesystem`] when symlink-follow traversal
///   revisits a canonical endpoint (cycle).
pub fn discover(
    project_dir: &Path, roots: &[PathBuf], languages: &[String],
) -> Result<FrameworkDiscovery, IndexError> {
    if !project_dir.is_dir() {
        return Err(IndexError::ProjectDirMissing(project_dir.to_path_buf()));
    }

    let walk_roots: Vec<PathBuf> = if roots.is_empty() {
        vec![project_dir.to_path_buf()]
    } else {
        roots.iter().map(|p| project_dir.join(p)).collect()
    };

    let overrides = build_overrides(project_dir)?;
    let language_filter: Option<&[String]> =
        if languages.is_empty() { None } else { Some(languages) };

    let mut builder = WalkBuilder::new(&walk_roots[0]);
    for extra in walk_roots.iter().skip(1) {
        builder.add(extra);
    }
    builder
        .follow_links(true)
        .standard_filters(true)
        .hidden(false)
        .require_git(false)
        .overrides(overrides);

    let mut files: Vec<DiscoveredFile> = Vec::new();
    let mut symlinks_out: Vec<Symlink> = Vec::new();

    for entry in builder.build() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                // §F1: symlink cycles abort the framework walk with a
                // `Filesystem` error. The `ignore` walker reports them
                // as `ignore::Error::Loop`; every other walker error
                // (a vanished entry, transient I/O, etc.) collapses
                // to a silent per-entry skip per the file scan
                // contract.
                if is_loop_error(&err) {
                    return Err(IndexError::Filesystem(format!(
                        "symlink cycle detected during framework walk: {err}"
                    )));
                }
                // A broken symlink cannot be traversed under follow
                // mode, so the walker yields it as a `WithPath` error
                // rather than an `Ok` entry. Resurrect it as a symlink
                // fact (with `broken == true`) so resolution hints can
                // flag it; any non-symlink walker error stays a silent
                // skip.
                if let Some(path) = err_path(&err)
                    && std::fs::symlink_metadata(path)
                        .is_ok_and(|meta| meta.file_type().is_symlink())
                    && let Some(fact) = symlinks::record(path, project_dir, FollowMode::Follow)
                {
                    symlinks_out.push(fact);
                }
                continue;
            }
        };
        let path = entry.path();
        if path == project_dir {
            continue;
        }
        let Some(file_type) = entry.file_type() else {
            continue;
        };

        // With `follow_links(true)` the walker reports the *target*
        // file type for a symlink, so `file_type.is_symlink()` would
        // never fire and the `Symlink` facts would be lost. Re-check
        // via `symlink_metadata` which inspects the path itself, then
        // record the symlink fact.
        let is_symlink =
            std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink());
        if is_symlink {
            if let Some(fact) = symlinks::record(path, project_dir, FollowMode::Follow) {
                symlinks_out.push(fact);
            }
            // Fall through to also record the resolved file under
            // its symlink path so per-file extractors (frontmatter,
            // markdown sections, etc.) still see the body via the
            // canonical-endpoint walk that follows in the same loop.
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let Some(relative) = project_relative(project_dir, path) else {
            continue;
        };
        if !is_included(&relative) {
            continue;
        }

        let language = infer_language(&relative);
        if let (Some(filter), Some(lang)) = (language_filter, language.as_ref())
            && !filter.iter().any(|allowed| allowed == lang)
        {
            continue;
        }

        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let (kind, bytes_field) = classify(&bytes);

        files.push(DiscoveredFile {
            relative,
            kind,
            language,
            bytes: bytes_field,
        });
    }

    files.sort_by(|a, b| a.relative.cmp(&b.relative));
    symlinks_out.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(FrameworkDiscovery {
        files,
        symlinks: symlinks_out,
    })
}

/// Extract the on-disk path the walker attached to an error, walking
/// the wrapped `ignore::Error` chain. Returns `None` for error kinds
/// that carry no path (the broken-symlink recovery needs the path to
/// re-stat the entry).
fn err_path(err: &ignore::Error) -> Option<&Path> {
    match err {
        ignore::Error::WithPath { path, .. } => Some(path.as_path()),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            err_path(err)
        }
        _ => None,
    }
}

/// Walk the wrapped `ignore::Error` chain (`WithPath` / `WithDepth`
/// / `WithLineNumber` / `Partial`) and return `true` when any node
/// is `ignore::Error::Loop` (the symlink-cycle discriminant).
fn is_loop_error(err: &ignore::Error) -> bool {
    match err {
        ignore::Error::Loop { .. } => true,
        ignore::Error::WithPath { err, .. }
        | ignore::Error::WithDepth { err, .. }
        | ignore::Error::WithLineNumber { err, .. } => is_loop_error(err),
        ignore::Error::Partial(errs) => errs.iter().any(is_loop_error),
        _ => false,
    }
}

fn build_overrides(project_dir: &Path) -> Result<ignore::overrides::Override, IndexError> {
    let mut builder = OverrideBuilder::new(project_dir);
    for pattern in ALWAYS_IGNORE_GLOBS {
        builder.add(pattern).map_err(|err| IndexError::OverrideCompile(err.to_string()))?;
    }
    builder.build().map_err(|err| IndexError::OverrideCompile(err.to_string()))
}

fn project_relative(project_dir: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(project_dir).ok()?;
    let s = rel.to_str()?;
    if MAIN_SEPARATOR == '/' { Some(s.to_owned()) } else { Some(s.replace(MAIN_SEPARATOR, "/")) }
}

fn classify(bytes: &[u8]) -> (FileKind, Option<Vec<u8>>) {
    let window = &bytes[..bytes.len().min(BINARY_SNIFF_BYTES)];
    if window.contains(&0) {
        (FileKind::Binary, None)
    } else {
        (FileKind::Text, Some(bytes.to_vec()))
    }
}

/// Framework include set per §F1.
fn is_included(relative: &str) -> bool {
    if INCLUDE_PREFIXES.iter().any(|prefix| relative.starts_with(prefix)) {
        return true;
    }
    // Repo-root README.md only (deeper READMEs ride their prefix).
    if relative == "README.md" {
        return true;
    }
    let Some(file_name) = relative.rsplit_once('/').map_or(Some(relative), |(_, name)| Some(name))
    else {
        return false;
    };
    matches!(file_name, "AGENTS.md" | "REVIEW.md" | "Specify.toml")
}

const INCLUDE_PREFIXES: &[&str] = &[
    "adapters/",
    "plugins/",
    "docs/",
    ".cursor/",
    "rfcs/",
    "scripts/",
    "schemas/",
    ".cursor-plugin/",
];

const ALWAYS_IGNORE_GLOBS: &[&str] =
    &["!target/**", "!**/node_modules/**", "!.git/**", "!dist/**", "!.specify/**", "!**/.DS_Store"];

#[cfg(test)]
mod tests;
