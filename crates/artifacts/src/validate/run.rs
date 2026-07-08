//! `validate_slice` — the top-level runner that walks the canonical
//! refine-time artifact set, locates each artifact, invokes the
//! registered rules, and collects a `Vec<Diagnostic>`.
//!
//! workflow §"Refinement" pins the canonical artifact set to
//! `proposal.md`, `spec.md`, `design.md`, `tasks.md`, plus the
//! `contracts/` overlay. Rules are registered in
//! [`crate::validate::registry::rules_for`] under per-brief namespaces
//! (`proposal`, `specs`, `design`, `tasks`, `contracts`); the runner
//! feeds artifacts into that registry directly.

use std::path::{Path, PathBuf};

use specify_diagnostics::{Artifact, Diagnostic, FindingLocation};
use specify_error::Error;

use crate::validate::registry::{cross_rules, rules_for};
use crate::validate::{BriefContext, Classification, CrossContext, RuleOutcome};

const DEFERRED_REASON: &str = "Semantic check — requires agent judgment";

/// Canonical refine-time artifact set, in registry-namespace order.
///
/// `(brief_id, artifact)` where `artifact` is either a literal path
/// relative to the slice dir or a glob (containing `*`). Mirrors the
/// validation registry's namespaces verbatim — rules are registered
/// under these ids in [`crate::validate::registry::rules_for`].
const CANONICAL_ARTIFACTS: &[(&str, &str)] = &[
    ("proposal", "proposal.md"),
    ("specs", "specs/**/*.md"),
    ("design", "design.md"),
    ("tasks", "tasks.md"),
    ("contracts", "contracts/**/*.yaml"),
];

/// Map a registry brief namespace to its diagnostic artifact category.
fn artifact_for(brief_id: &str) -> Artifact {
    match brief_id {
        "specs" => Artifact::Specs,
        "design" => Artifact::Design,
        "tasks" => Artifact::Tasks,
        "contracts" => Artifact::Contracts,
        _ => Artifact::Unknown,
    }
}

/// Slice-relative anchor location for an existing artifact, or `None`
/// when the relative path cannot be formed.
fn rel_location(slice_dir: &Path, artifact_path: &Path) -> Option<FindingLocation> {
    let path = relative_key(slice_dir, artifact_path);
    (!path.is_empty()).then_some(FindingLocation {
        path,
        line: None,
        column: None,
        end_line: None,
        end_column: None,
    })
}

/// Run all deterministic validations for a slice directory.
///
/// Iterates the canonical refine-time artifact set
/// (`CANONICAL_ARTIFACTS`). Plain entries are stat-checked once;
/// glob entries are expanded via the `glob` crate and only existing
/// matches are walked. Empty glob results are silently skipped — an
/// absent `specs/login/spec.md` is not, by itself, a failure.
///
/// Returns the [`Diagnostic`] findings only — structural `Fail`
/// outcomes as deterministic `violation`s and semantic rules as
/// non-blocking `review`s. Passing structural rules emit nothing, so an
/// empty vector means a clean slice. The caller assembles these into a
/// `DiagnosticReport`, renders it, and decides the exit policy.
///
/// # Errors
///
/// Returns an error if a glob pattern is malformed or a glob traversal
/// fails.
pub fn validate_slice(slice_dir: &Path) -> Result<Vec<Diagnostic>, Error> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let specs_dir = slice_dir.join("specs");

    for (brief_id, artifact) in CANONICAL_ARTIFACTS {
        let artifacts = expand_artifact(slice_dir, artifact)?;

        if artifacts.is_empty() {
            // Glob matched nothing. If the configured path is literal,
            // treat that as "artifact missing" so the skill sees the
            // failure. Globs that legitimately match nothing are
            // skipped silently — Specify slices don't have to populate
            // every overlay (e.g. `contracts/`).
            if !artifact.contains('*') {
                let missing_path = slice_dir.join(artifact);
                diagnostics.push(artifact_missing(brief_id, &missing_path, slice_dir));
            }
            continue;
        }

        for artifact_path in artifacts {
            run_brief_rules(brief_id, &artifact_path, slice_dir, &specs_dir, &mut diagnostics);
        }
    }

    run_cross_rules(slice_dir, &specs_dir, &mut diagnostics);

    Ok(diagnostics)
}

/// Expand `artifact` into a concrete list of absolute paths under
/// `slice_dir`. Plain paths are returned as a singleton (regardless of
/// existence — the runner checks that separately). Glob patterns
/// (containing `*`) are expanded via the `glob` crate and only existing
/// matches are returned.
fn expand_artifact(slice_dir: &Path, artifact: &str) -> Result<Vec<PathBuf>, Error> {
    let joined = slice_dir.join(artifact);
    if !artifact.contains('*') {
        return Ok(vec![joined]);
    }
    let pattern = joined.to_str().ok_or_else(|| Error::Diag {
        code: "validate-glob-non-utf8",
        detail: format!("non-UTF8 glob pattern `{}`", joined.display()),
    })?;
    let mut out: Vec<PathBuf> = Vec::new();
    let entries = glob::glob(pattern).map_err(|err| Error::Diag {
        code: "validate-glob-invalid",
        detail: format!("invalid glob `{pattern}`: {err}"),
    })?;
    for entry in entries {
        match entry {
            Ok(path) if path.is_file() => out.push(path),
            Ok(_) => {}
            Err(err) => {
                return Err(Error::Diag {
                    code: "validate-glob-traversal-failed",
                    detail: format!("glob traversal failure: {err}"),
                });
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Build the slice-relative key carried on each diagnostic's
/// `location.path` for multi-artifact briefs. We strip `slice_dir` to
/// make the key stable across different tempdir prefixes; unix-style
/// forward slashes are used so golden fixtures compare identically
/// across platforms.
fn relative_key(slice_dir: &Path, artifact_path: &Path) -> String {
    let rel = artifact_path.strip_prefix(slice_dir).unwrap_or(artifact_path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn artifact_missing(brief_id: &str, artifact_path: &Path, slice_dir: &Path) -> Diagnostic {
    let rel = relative_key(slice_dir, artifact_path);
    Diagnostic::violation(
        format!("{brief_id}.artifact-exists"),
        format!("Generated artifact {rel} exists"),
        format!("artifact `{rel}` not found under slice dir"),
        artifact_for(brief_id),
        None,
    )
}

fn run_brief_rules(
    brief_id: &str, artifact_path: &Path, slice_dir: &Path, specs_dir: &Path,
    out: &mut Vec<Diagnostic>,
) {
    let Ok(content) = std::fs::read_to_string(artifact_path) else {
        out.push(artifact_missing(brief_id, artifact_path, slice_dir));
        return;
    };

    // Parse brief-specific structured context.
    let parsed_spec = (brief_id == "specs").then(|| crate::spec::parse_baseline(&content));
    let tasks = (brief_id == "tasks").then(|| crate::task::parse_tasks(&content));

    let ctx = BriefContext {
        id: brief_id,
        content: &content,
        parsed_spec: parsed_spec.as_ref(),
        tasks: tasks.as_ref(),
        slice_dir,
        specs_dir,
    };

    let artifact = artifact_for(brief_id);
    let location = rel_location(slice_dir, artifact_path);
    for rule in rules_for(brief_id) {
        match rule.check {
            // Semantic rule (`check: None`) — a non-blocking review
            // request the agent must judge.
            None => out.push(Diagnostic::review(
                rule.id,
                rule.description,
                DEFERRED_REASON,
                artifact,
                location.clone(),
            )),
            Some(check) => {
                if let RuleOutcome::Fail { detail } = check(&ctx) {
                    out.push(Diagnostic::violation(
                        rule.id,
                        rule.description,
                        detail,
                        artifact,
                        location.clone(),
                    ));
                }
            }
        }
    }
}

fn run_cross_rules(slice_dir: &Path, specs_dir: &Path, out: &mut Vec<Diagnostic>) {
    let ctx = CrossContext { slice_dir, specs_dir };
    for rule in cross_rules() {
        match rule.classification {
            Classification::Semantic => out.push(Diagnostic::review(
                rule.id,
                rule.description,
                DEFERRED_REASON,
                Artifact::Specs,
                None,
            )),
            Classification::Structural => {
                if let RuleOutcome::Fail { detail } = (rule.check)(&ctx) {
                    out.push(Diagnostic::violation(
                        rule.id,
                        rule.description,
                        detail,
                        Artifact::Specs,
                        None,
                    ));
                }
            }
        }
    }
}
