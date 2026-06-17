//! Codex resolver (CH-12): roots + overlay discovery.
//!
//! Implements rules root resolution and codex root discovery
//! / §"Resolution inputs" / §"Overlay precedence". This chunk discovers
//! and parses every rule the export envelope eventually carries.
//!
//! Filtering by applicability and deprecation lives in the sibling
//! [`filter`] module (CH-13); stable export ordering (CH-14) and
//! findings (CH-15/16) remain out of scope here.
//!
//! # Closed precedence order
//!
//! Shared root (root 1) resolves from a **rules root**, picked by the
//! closed probe in codex root resolution:
//!
//! 1. `inputs.rules_root` when supplied — use for root 1 and the
//!    rules-root fallback overlay (overlay step 3 below).
//! 2. Else if `{project_dir}/adapters/shared/rules/universal/` exists,
//!    treat `project_dir` as the rules root (monorepo case). In this
//!    case the rules-root fallback overlay step is **skipped** —
//!    re-walking `project_dir` would just shadow the project-local
//!    rung with the same filesystem tree.
//! 3. Else if the distributed codex cache
//!    `<project-cache>/codex/adapters/shared/rules/universal/`
//!    exists (resolved out-of-tree from the OS cache), treat
//!    `<project-cache>/codex/` as the rules
//!    root. Populated by codex distribution (RM-07) at `specify init`
//!    or `specify rules sync`. Like the monorepo case this is a derived
//!    (non-explicit) root, so the rules-root fallback overlay step is
//!    **skipped**.
//! 4. Else → [`ResolveError::RulesRootRequired`].
//!
//! Source-adapter (root 3) and target-adapter (root 4) overlays follow
//! the closed location order in rules root resolution:
//!
//! 1. project-local `{project_dir}/adapters/{sources,targets}/<name>/rules/`;
//! 2. manifest cache `<project-cache>/manifests/{sources,targets}/<name>/rules/`
//!    (out-of-tree; provenance recorded under [`PathRoot::Cache`] with a
//!    cache-relative `manifests/...` path);
//! 3. rules-root fallback `{rules_root}/adapters/{sources,targets}/<name>/rules/`,
//!    **only** when `inputs.rules_root.is_some()` (step 1 of the probe);
//! 4. omit when no rung exists.
//!
//! The **first existing** rung wins; locations never merge. Roots 2
//! (shared language/artifact packs) and 5 (project-local overlays) are
//! reserved by the rules contract and not implemented here. The closed
//! [`Origin`] enum's `Unknown` variant is **not** root 5 — it is the
//! consumer indexer's fallback bucket for cache rule files whose path
//! does not match a recognized adapter shape (see the `infer_origin`
//! function in [`crate::lint::index`]).
//!
//! # Duplicate ids
//!
//! Per overlay precedence: rules never override each other
//! by sharing ids — duplicates always error, regardless of
//! `include_deprecated`. The check runs after every rung is loaded so
//! collisions across overlays surface as
//! [`ResolveError::DuplicateRuleId`].
//!
//! # Out of scope
//!
//! - Applicability + deprecation filtering — see the sibling
//!   [`filter`] module (CH-13). [`resolve`] returns the unfiltered pool;
//!   call [`filter`] on that result to get the narrowed view.
//! - Stable export ordering — CH-14 lives in the sibling `sort`
//!   module. CH-12 only enforces deterministic intra-directory lexical
//!   order so test goldens stay stable; the closed four-tuple sort and
//!   the [`super::ResolvedRules`] envelope are assembled by
//!   [`build_resolved_rules`].

mod filter;
mod sort;

use std::path::{Path, PathBuf};
use std::{fs, io};

pub use filter::filter;
pub use sort::{build_resolved_rules, sort_resolved};
use specify_error::Error;

use super::parse::{ParseError, parse_rule_file};
use super::{Origin, PathRoot, Rule};

/// Closed input contract for [`resolve`] and [`filter`] per the rules contract
/// §"Resolution inputs".
///
/// CH-12's [`resolve`] consumes `project_dir`, `rules_root`,
/// `target_adapter`, and `source_adapters`. CH-13's [`filter`]
/// additionally consumes `artifact_paths`, `languages`,
/// `include_deprecated`, `include_unmatched`, and `include_core`.
/// Callers compose [`resolve`] then [`filter`] when they need the
/// narrowed pool; [`build_resolved_rules`] is the export entry point.
#[derive(Debug, Clone)]
pub struct ResolveInputs<'a> {
    /// Project root used for adapter resolution and optional
    /// project-local overlays.
    pub project_dir: &'a Path,
    /// Root containing first-party codex content for shared rules and
    /// rules-root fallback overlays. See §"Codex root resolution (v1)".
    pub rules_root: Option<&'a Path>,
    /// Target adapter name (optionally `<name>@v<major>`; resolver
    /// uses the raw name as the directory segment).
    pub target_adapter: &'a str,
    /// Source adapter names bound by the active plan entry.
    pub source_adapters: &'a [String],
    /// Project-relative artifact paths consumed by CH-13's
    /// `applicability.paths` glob check.
    pub artifact_paths: &'a [PathBuf],
    /// Language tokens consumed by CH-13's `applicability.languages`
    /// match.
    pub languages: &'a [String],
    /// Whether deprecated rules appear in the export. Toggled by CH-13.
    pub include_deprecated: bool,
    /// Whether rules with populated applicability dimensions the
    /// caller did not satisfy are included. Toggled by CH-13.
    pub include_unmatched: bool,
    /// Whether rules with [`super::Origin::Core`] appear in the
    /// export. Default off (consumer-export filtering): consumer-project
    /// review runs never evaluate `CORE-*` hints by accident.
    pub include_core: bool,
}

/// Pre-sort intermediate emitted by [`resolve`].
///
/// CH-14 owns the final sorted [`super::ResolvedRules`]; CH-12 returns
/// every discovered rule with `origin`, `path_root`, and `path`
/// populated. The `path` string is always relative to the matching
/// [`PathRoot`], with forward slashes as separators.
#[derive(Debug, Clone)]
pub struct ResolvedRuleEntry {
    /// Parsed rule from CH-11.
    pub rule: Rule,
    /// Resolver origin tier (shared / source / target).
    pub origin: Origin,
    /// Anchor for [`Self::path`] — rules-root vs project-dir.
    pub path_root: PathRoot,
    /// Path to the rule markdown file relative to `path_root`.
    pub path: String,
}

/// Failure modes for [`resolve`].
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    /// Shared probe failed: no `--rules-root`, no monorepo fallback
    /// under `project_dir`, and no distributed codex cache. Wire-id
    /// `rules-root-required` per codex root resolution and the §490
    /// golden.
    #[error(
        "rules-root-required: shared UNI-* rules require --rules-root pointing at a tree containing adapters/shared/rules/universal/, a monorepo adapters/shared/rules/universal/ tree, or a distributed codex cache (run `specify rules sync`)"
    )]
    RulesRootRequired,
    /// A rule id appeared in more than one discovered file. Per
    /// overlay precedence this is always invalid.
    #[error("duplicate rule id '{id}' across files: {paths}")]
    DuplicateRuleId {
        /// The colliding rule id.
        id: String,
        /// Comma-joined relative paths of every offending file.
        paths: String,
    },
    /// CH-11 parser rejected one of the discovered files.
    #[error("rule parse failed: {path}: {error}")]
    Parse {
        /// Absolute path of the failing file.
        path: PathBuf,
        /// Underlying parser error.
        error: ParseError,
    },
    /// `read_dir` / `read_to_string` failed while walking a rung.
    #[error("rule discovery failed: {path}: {source}")]
    Filesystem {
        /// Absolute path of the failing directory or file.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
}

/// Translate the resolver's typed [`ResolveError`] onto the closed
/// [`specify_error::Error`] enum.
///
/// `Exit::from(&Error)` then picks the right exit code per
/// `docs/standards/handler-shape.md`: the three codex-shape failures
/// map to `Validation` (exit 2) and the I/O failure maps to
/// `Filesystem` (exit 1).
#[must_use]
pub fn map_resolve_error(err: ResolveError) -> Error {
    match err {
        ResolveError::RulesRootRequired => Error::validation_failed(
            "rules-root-required",
            "shared UNI-* rules require --rules-root, a project-local \
             adapters/shared/rules/universal/ tree, or a distributed \
             codex cache in the out-of-tree project cache",
            "run `specify rules sync` to distribute the shared codex, or \
             pass --rules-root pointing at a tree containing \
             adapters/shared/rules/universal/",
        ),
        ResolveError::DuplicateRuleId { id, paths } => Error::validation_failed(
            "rules-duplicate-rule-id",
            format!("rule id '{id}' appears in multiple files"),
            paths,
        ),
        ResolveError::Parse { path, error } => Error::validation_failed(
            "rules-parse-error",
            format!("failed to parse rule {}", path.display()),
            error.to_string(),
        ),
        ResolveError::Filesystem { path, source } => Error::Filesystem {
            op: "readdir",
            path,
            source,
        },
    }
}

const SHARED_REL: &str = "adapters/shared/rules/universal";
const CORE_REL: &str = "adapters/shared/rules/core";
/// Cache-relative provenance prefix recorded for cache-resolved adapter
/// rules.
///
/// The physical manifest cache lives out-of-tree (see
/// [`specify_schema::cache::project_cache_dir`]), so rule provenance is
/// recorded under [`PathRoot::Cache`] with this stable, cache-relative
/// prefix rather than the physical absolute path, keeping findings and
/// goldens portable.
const MANIFEST_CACHE_LOGICAL: &str = "manifests";

/// Out-of-tree manifest cache root, `<project-cache>/manifests/`. Kept
/// in lockstep with `specify_workflow::adapter::cache_axis_dir`.
fn manifest_cache_root(project_dir: &Path) -> PathBuf {
    specify_schema::cache::project_cache_dir(project_dir).join("manifests")
}

/// Out-of-tree project codex cache root, `<project-cache>/codex/`.
/// Probe step 3 treats it as a derived rules root when it carries the
/// shared `universal/` pack. Kept in lockstep with
/// `specify_workflow::init::codex_cache_root`.
fn codex_cache_root(project_dir: &Path) -> PathBuf {
    specify_schema::cache::project_cache_dir(project_dir).join("codex")
}

/// Discover every rule visible to `inputs` and parse it.
///
/// See the module docs for the closed precedence order, rules-root
/// probe, and duplicate-id rules. Filtering and sorting are deferred
/// to CH-13 / CH-14.
///
/// # Errors
///
/// Returns the matching [`ResolveError`] variant for each failure
/// mode; see the variant docs.
pub fn resolve(inputs: &ResolveInputs<'_>) -> Result<Vec<ResolvedRuleEntry>, ResolveError> {
    let rules_root = probe_rules_root(inputs)?;
    let explicit_rules_root = inputs.rules_root.is_some();

    let mut entries: Vec<ResolvedRuleEntry> = Vec::new();

    let shared_dir = rules_root.join(SHARED_REL);
    if shared_dir.is_dir() {
        for path in list_rule_files(&shared_dir)? {
            let rel = relative_path(&rules_root, &path);
            let rule = parse(&path)?;
            entries.push(ResolvedRuleEntry {
                rule,
                origin: Origin::Shared,
                path_root: PathRoot::RulesRoot,
                path: rel,
            });
        }
    }

    let core_dir = rules_root.join(CORE_REL);
    if core_dir.is_dir() {
        for path in list_rule_files(&core_dir)? {
            let rel = relative_path(&rules_root, &path);
            let rule = parse(&path)?;
            entries.push(ResolvedRuleEntry {
                rule,
                origin: Origin::Core,
                path_root: PathRoot::RulesRoot,
                path: rel,
            });
        }
    }

    for source_name in inputs.source_adapters {
        load_overlay(
            inputs.project_dir,
            &rules_root,
            explicit_rules_root,
            "sources",
            source_name,
            Origin::Source,
            &mut entries,
        )?;
    }

    load_overlay(
        inputs.project_dir,
        &rules_root,
        explicit_rules_root,
        "targets",
        inputs.target_adapter,
        Origin::Target,
        &mut entries,
    )?;

    detect_duplicates(&entries)?;

    Ok(entries)
}

/// Pick the rules root from the §"Codex root resolution (v1)" probe.
fn probe_rules_root(inputs: &ResolveInputs<'_>) -> Result<PathBuf, ResolveError> {
    if let Some(explicit) = inputs.rules_root {
        return Ok(explicit.to_path_buf());
    }
    let project_shared = inputs.project_dir.join(SHARED_REL);
    if project_shared.is_dir() {
        return Ok(inputs.project_dir.to_path_buf());
    }
    let codex_cache = codex_cache_root(inputs.project_dir);
    if codex_cache.join(SHARED_REL).is_dir() {
        return Ok(codex_cache);
    }
    Err(ResolveError::RulesRootRequired)
}

/// Walk overlay rungs for one adapter and append discovered rules.
///
/// `axis_segment` is the literal directory name (`"sources"` or
/// `"targets"`). The first existing rung wins per the rules contract §"Resolution
/// roots".
fn load_overlay(
    project_dir: &Path, rules_root: &Path, explicit_rules_root: bool, axis_segment: &str,
    adapter_name: &str, origin: Origin, out: &mut Vec<ResolvedRuleEntry>,
) -> Result<(), ResolveError> {
    let project_local =
        project_dir.join("adapters").join(axis_segment).join(adapter_name).join("rules");
    let manifest_cache =
        manifest_cache_root(project_dir).join(axis_segment).join(adapter_name).join("rules");

    if project_local.is_dir() {
        return collect_overlay(&project_local, project_dir, PathRoot::ProjectDir, origin, out);
    }
    if manifest_cache.is_dir() {
        // Physical files live out-of-tree; record provenance under the
        // stable cache-relative `manifests/...` path (PathRoot::Cache).
        let logical = format!("{MANIFEST_CACHE_LOGICAL}/{axis_segment}/{adapter_name}/rules");
        return collect_overlay_logical(&manifest_cache, &logical, origin, out);
    }
    if explicit_rules_root {
        let fallback =
            rules_root.join("adapters").join(axis_segment).join(adapter_name).join("rules");
        if fallback.is_dir() {
            return collect_overlay(&fallback, rules_root, PathRoot::RulesRoot, origin, out);
        }
    }
    Ok(())
}

/// Parse every `.md` file under `dir` and append to `out`.
fn collect_overlay(
    dir: &Path, path_root_dir: &Path, path_root: PathRoot, origin: Origin,
    out: &mut Vec<ResolvedRuleEntry>,
) -> Result<(), ResolveError> {
    for path in list_rule_files(dir)? {
        let rel = relative_path(path_root_dir, &path);
        let rule = parse(&path)?;
        out.push(ResolvedRuleEntry {
            rule,
            origin,
            path_root,
            path: rel,
        });
    }
    Ok(())
}

/// Parse every `.md` file under an out-of-tree `dir`, recording each
/// rule's provenance under `logical_root` (a stable cache-relative
/// prefix anchored at [`PathRoot::Cache`]) rather than the physical
/// out-of-tree path. `list_rule_files` is non-recursive, so the logical
/// path is `logical_root/<filename>`.
fn collect_overlay_logical(
    dir: &Path, logical_root: &str, origin: Origin, out: &mut Vec<ResolvedRuleEntry>,
) -> Result<(), ResolveError> {
    for path in list_rule_files(dir)? {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let rule = parse(&path)?;
        out.push(ResolvedRuleEntry {
            rule,
            origin,
            path_root: PathRoot::Cache,
            path: format!("{logical_root}/{name}"),
        });
    }
    Ok(())
}

/// Non-recursive `.md` listing for a codex directory, excluding
/// `README.md` (case-insensitive) and symlinks.
///
/// Results are sorted lexically by absolute path so per-directory
/// iteration order is deterministic for goldens.
fn list_rule_files(dir: &Path) -> Result<Vec<PathBuf>, ResolveError> {
    let entries = fs::read_dir(dir).map_err(|source| ResolveError::Filesystem {
        path: dir.to_path_buf(),
        source,
    })?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| ResolveError::Filesystem {
            path: dir.to_path_buf(),
            source,
        })?;
        let file_type = entry.file_type().map_err(|source| ResolveError::Filesystem {
            path: entry.path(),
            source,
        })?;
        if file_type.is_symlink() {
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if !name_str.to_ascii_lowercase().ends_with(".md") {
            continue;
        }
        if name_str.eq_ignore_ascii_case("README.md") {
            continue;
        }
        paths.push(entry.path());
    }
    paths.sort();
    Ok(paths)
}

fn parse(path: &Path) -> Result<Rule, ResolveError> {
    parse_rule_file(path).map_err(|error| ResolveError::Parse {
        path: path.to_path_buf(),
        error,
    })
}

/// Compute `path` relative to `root` and normalise separators to `/`.
///
/// Falls back to the absolute path string if `strip_prefix` fails (in
/// practice this only happens when callers pass a path outside the
/// claimed root, which `list_rule_files` never does).
fn relative_path(root: &Path, path: &Path) -> String {
    let stripped = path.strip_prefix(root).unwrap_or(path);
    let display = stripped.to_string_lossy().into_owned();
    if cfg!(windows) { display.replace('\\', "/") } else { display }
}

/// Per overlay precedence: duplicates always error.
fn detect_duplicates(entries: &[ResolvedRuleEntry]) -> Result<(), ResolveError> {
    use std::collections::BTreeMap;

    let mut by_id: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for entry in entries {
        by_id.entry(entry.rule.id.as_str()).or_default().push(entry.path.as_str());
    }
    for (id, paths) in by_id {
        if paths.len() > 1 {
            return Err(ResolveError::DuplicateRuleId {
                id: id.to_string(),
                paths: paths.join(", "),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
