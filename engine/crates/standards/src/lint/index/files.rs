//! `.gitignore`-aware filesystem walk per `WorkspaceModel` file scan.
//!
//! Owns the consumer-profile traversal: applies the always-ignore
//! globs, post-filters against the default include globs, detects
//! binary files (NUL byte in the first 8 `KiB`), decodes text bytes
//! lossily as UTF-8, infers a language token from the extension, and
//! captures symlink facts as it walks (without traversing through
//! them). The result is a sorted `(files, symlinks)` pair that the
//! umbrella in [`crate::lint::index`] hands to the per-file
//! extractors.

use std::path::{MAIN_SEPARATOR, Path, PathBuf};

use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;

use super::languages::infer_language;
use super::symlinks::FollowMode;
use super::{IndexError, symlinks};
use crate::lint::{FileKind, Symlink};

/// Intermediate per-file carrier passed to the per-file extractors.
///
/// `bytes` is `Some` for text files (UTF-8 decoded lossily into
/// [`Self::text`] on demand by callers) and `None` for binaries.
/// Stored once per file so the umbrella can fan extractors out in
/// parallel without re-reading the disk per extractor.
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// Project-relative path with forward slashes per `WorkspaceModel` stability.
    pub relative: String,
    /// Closed file-kind discriminant per `WorkspaceModel` file scan.
    pub kind: FileKind,
    /// Language token inferred from the file extension. `None` for
    /// binaries or unknown extensions.
    pub language: Option<String>,
    /// Raw bytes for text files; `None` for binary files.
    pub bytes: Option<Vec<u8>>,
}

impl DiscoveredFile {
    /// Decode the carried bytes as UTF-8 with U+FFFD replacement per
    /// `WorkspaceModel` file scan. Returns an empty string for binary files.
    pub fn text(&self) -> String {
        self.bytes.as_ref().map_or_else(String::new, |b| String::from_utf8_lossy(b).into_owned())
    }
}

/// Files + symlinks recorded during a single walk pass.
#[derive(Debug, Default)]
pub struct DiscoveryOutput {
    /// Text + binary files discovered under `roots`.
    pub files: Vec<DiscoveredFile>,
    /// Symlinks discovered under `roots`. Not traversed.
    pub symlinks: Vec<Symlink>,
}

/// First 8 `KiB` window scanned for NUL bytes per `WorkspaceModel` file scan.
const BINARY_SNIFF_BYTES: usize = 8 * 1024;

/// Walk the consumer project tree rooted at `project_dir`.
///
/// When `roots` is empty the walk starts at `project_dir`; otherwise
/// every entry is treated as a project-relative path under
/// `project_dir` per lint scope resolution. Walk-level failures (entry iteration
/// errors) are swallowed silently in v1 — reserved-hint diagnostics reserves the
/// `index.warning` finding for S7's hint runner; the walk only emits
/// an `Err` when the project root itself is missing.
///
/// The output collections are sorted by project-relative path so the
/// downstream parallel dispatch is deterministic before the per-family
/// sort runs.
///
/// # Errors
///
/// Returns [`IndexError::ProjectDirMissing`] when `project_dir` is
/// not a directory; per-entry failures collapse to a silent skip.
pub fn discover(
    project_dir: &Path, roots: &[PathBuf], languages: &[String],
) -> Result<DiscoveryOutput, IndexError> {
    if !project_dir.is_dir() {
        return Err(IndexError::ProjectDirMissing(project_dir.to_path_buf()));
    }

    let walk_roots: Vec<PathBuf> = if roots.is_empty() {
        vec![project_dir.to_path_buf()]
    } else {
        roots.iter().map(|p| project_dir.join(p)).collect()
    };

    let overrides = build_always_ignore(project_dir)?;
    let language_filter: Option<&[String]> =
        if languages.is_empty() { None } else { Some(languages) };

    let mut builder = WalkBuilder::new(&walk_roots[0]);
    for extra in walk_roots.iter().skip(1) {
        builder.add(extra);
    }
    builder
        .follow_links(false)
        .standard_filters(true)
        // `.specify/` is hidden by default; the project profile
        // walks it explicitly per `WorkspaceModel` file scan.
        .hidden(false)
        // `.gitignore` must be honoured even when the consumer project
        // is not itself a git repo (e.g. inside a tempdir).
        .require_git(false)
        .overrides(overrides);

    let mut files: Vec<DiscoveredFile> = Vec::new();
    let mut symlinks_out: Vec<Symlink> = Vec::new();

    for entry in builder.build() {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if path == project_dir {
            continue;
        }
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            if let Some(fact) = symlinks::record(path, project_dir, FollowMode::Record) {
                symlinks_out.push(fact);
            }
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

    Ok(DiscoveryOutput {
        files,
        symlinks: symlinks_out,
    })
}

fn build_always_ignore(project_dir: &Path) -> Result<ignore::overrides::Override, IndexError> {
    let mut builder = OverrideBuilder::new(project_dir);
    for pattern in ALWAYS_IGNORE_GLOBS {
        builder.add(pattern).map_err(|err| IndexError::OverrideCompile(err.to_string()))?;
    }
    builder.build().map_err(|err| IndexError::OverrideCompile(err.to_string()))
}

/// Project-relative path with forward slashes, or `None` when the
/// entry sits outside the project root.
fn project_relative(project_dir: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(project_dir).ok()?;
    let s = rel.to_str()?;
    if MAIN_SEPARATOR == '/' { Some(s.to_owned()) } else { Some(s.replace(MAIN_SEPARATOR, "/")) }
}

/// `WorkspaceModel` file scan binary-file detection: NUL byte anywhere in the first
/// `BINARY_SNIFF_BYTES` window.
fn classify(bytes: &[u8]) -> (FileKind, Option<Vec<u8>>) {
    let window = &bytes[..bytes.len().min(BINARY_SNIFF_BYTES)];
    if window.contains(&0) {
        (FileKind::Binary, None)
    } else {
        (FileKind::Text, Some(bytes.to_vec()))
    }
}

/// `WorkspaceModel` file scan default include globs. A file passes the include filter
/// when its extension is in the language table OR it lives under
/// `.specify/`. Manual matching avoids pulling in a glob engine for a
/// single brace-expansion.
fn is_included(relative: &str) -> bool {
    if relative.starts_with(".specify/") {
        return true;
    }
    let Some((_, ext)) = relative.rsplit_once('.') else {
        return false;
    };
    INCLUDE_EXTENSIONS.contains(&ext)
}

const INCLUDE_EXTENSIONS: &[&str] = &[
    "md", "yaml", "yml", "json", "toml", "rs", "swift", "kt", "kts", "gradle", "ts", "tsx", "js",
    "jsx", "py", "sql",
];

const ALWAYS_IGNORE_GLOBS: &[&str] = &[
    "!target/**",
    "!**/node_modules/**",
    "!.git/**",
    "!dist/**",
    "!build/**",
    "!out/**",
    "!**/.DS_Store",
];

#[cfg(test)]
mod tests;
