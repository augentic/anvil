//! Read side of the slice-merge engine: staged-tree discovery for the
//! 3-way merge plan, opaque-replace pre-image enumeration, and
//! baseline-mtime drift checks.

use std::fs;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use specify_error::Error;

use super::parse::system_time_to_utc;
use super::{BaselineConflict, MergePreviewEntry, OpaqueAction, OpaquePreviewEntry};
use crate::merge::artifact_class::{ArtifactClass, MergeStrategy};
use crate::merge::engine::merge;
use crate::merge::validate::validate_baseline;

/// File name for the optional composition delta that lives at the top
/// of a slice directory (alongside `proposal.md` etc.). Promoted into
/// the first [`MergeStrategy::ThreeWayMerge`] class's baseline.
pub(super) const COMPOSITION_FILENAME: &str = "composition.yaml";

/// One delta spec discovered under a class's `staged_dir/<spec>/spec.md`,
/// paired with the path of the baseline file it merges into.
struct DeltaSpecRef {
    spec_name: String,
    delta_path: PathBuf,
    baseline_path: PathBuf,
}

/// First [`MergeStrategy::ThreeWayMerge`] class in declaration order.
///
/// The composition delta is promoted against this class only; later
/// `ThreeWayMerge` classes (if any) skip composition handling. Multiple
/// `ThreeWayMerge` classes in one slice are unusual today but not
/// forbidden by the engine.
pub(super) fn first_three_way(classes: &[ArtifactClass]) -> Option<&ArtifactClass> {
    classes.iter().find(|c| matches!(c.strategy, MergeStrategy::ThreeWayMerge))
}

/// Compute the in-memory merge plan for every delta spec discovered
/// under each [`MergeStrategy::ThreeWayMerge`] class's `staged_dir`,
/// plus the optional `composition.yaml` delta at the top of the slice
/// directory.
///
/// Per-spec merge or coherence-validation conflicts are aggregated into
/// a single `Error::Diag { code: "merge-spec-conflicts" }` so callers
/// can surface every conflict at once instead of bailing on the first.
pub(super) fn plan_three_way(
    slice_dir: &Path, classes: &[ArtifactClass],
) -> Result<Vec<MergePreviewEntry>, Error> {
    let mut merged: Vec<MergePreviewEntry> = Vec::new();
    let mut aborts: Vec<String> = Vec::new();
    let mut composition_handled = false;

    for class in classes.iter().filter(|c| matches!(c.strategy, MergeStrategy::ThreeWayMerge)) {
        let delta_specs = list_delta_specs(class)?;
        for spec in delta_specs {
            match merge_delta_spec(class, &spec)? {
                Ok(entry) => merged.push(entry),
                Err(details) => aborts.extend(details),
            }
        }

        if !composition_handled {
            composition_handled = true;
            if let Some(entry) = merge_composition_delta(slice_dir, class)? {
                match entry {
                    Ok(preview) => merged.push(preview),
                    Err(detail) => aborts.push(detail),
                }
            }
        }
    }

    if !aborts.is_empty() {
        return Err(Error::Diag {
            code: "merge-spec-conflicts",
            detail: aborts.join("\n"),
        });
    }

    merged.sort_by(|a, b| {
        (a.class_name.as_str(), a.name.as_str()).cmp(&(b.class_name.as_str(), b.name.as_str()))
    });
    Ok(merged)
}

fn list_delta_specs(class: &ArtifactClass) -> Result<Vec<DeltaSpecRef>, Error> {
    let mut delta_specs: Vec<DeltaSpecRef> = Vec::new();
    if !class.staged_dir.is_dir() {
        return Ok(delta_specs);
    }
    for entry in fs::read_dir(&class.staged_dir).map_err(|err| Error::Filesystem {
        op: "readdir",
        path: class.staged_dir.clone(),
        source: err,
    })? {
        let entry = entry.map_err(|err| Error::Filesystem {
            op: "dir-entry",
            path: class.staged_dir.clone(),
            source: err,
        })?;
        let file_type = entry.file_type().map_err(|err| Error::Filesystem {
            op: "file-type",
            path: entry.path(),
            source: err,
        })?;
        if !file_type.is_dir() {
            continue;
        }
        let delta_path = entry.path().join("spec.md");
        if !delta_path.is_file() {
            continue;
        }
        let spec_name = entry
            .file_name()
            .to_str()
            .ok_or_else(|| Error::Diag {
                code: "merge-non-utf8-name",
                detail: "non-UTF8 spec directory name".into(),
            })?
            .to_string();
        let baseline_path = class.baseline_dir.join(&spec_name).join("spec.md");
        delta_specs.push(DeltaSpecRef {
            spec_name,
            delta_path,
            baseline_path,
        });
    }
    delta_specs.sort_by(|a, b| a.delta_path.cmp(&b.delta_path));
    Ok(delta_specs)
}

fn merge_delta_spec(
    class: &ArtifactClass, spec: &DeltaSpecRef,
) -> Result<Result<MergePreviewEntry, Vec<String>>, Error> {
    let delta_text = fs::read_to_string(&spec.delta_path).map_err(|err| Error::Filesystem {
        op: "read",
        path: spec.delta_path.clone(),
        source: err,
    })?;
    let baseline_text = read_optional_file(&spec.baseline_path)?;

    let result = match merge(baseline_text.as_deref(), &delta_text) {
        Ok(r) => r,
        Err(Error::Diag {
            code: "merge-spec-conflicts",
            detail,
        }) => return Ok(Err(vec![format!("{}: {detail}", spec.spec_name)])),
        Err(other) => return Err(other),
    };

    let issues: Vec<String> = validate_baseline(&result.output)
        .into_iter()
        .map(|diagnostic| format!("{}: {}", spec.spec_name, diagnostic.impact))
        .collect();
    if !issues.is_empty() {
        return Ok(Err(issues));
    }

    Ok(Ok(MergePreviewEntry {
        class_name: class.name.clone(),
        name: spec.spec_name.clone(),
        baseline_path: spec.baseline_path.clone(),
        result,
    }))
}

fn merge_composition_delta(
    slice_dir: &Path, class: &ArtifactClass,
) -> Result<Option<Result<MergePreviewEntry, String>>, Error> {
    let composition_delta_path = slice_dir.join(COMPOSITION_FILENAME);
    if !composition_delta_path.is_file() {
        return Ok(None);
    }
    let delta_text =
        fs::read_to_string(&composition_delta_path).map_err(|err| Error::Filesystem {
            op: "read",
            path: composition_delta_path.clone(),
            source: err,
        })?;
    let baseline_path = class.baseline_dir.join(COMPOSITION_FILENAME);
    let baseline_text = read_optional_file(&baseline_path)?;

    match crate::merge::composition::merge(baseline_text.as_deref(), &delta_text) {
        Ok(comp_result) => Ok(Some(Ok(MergePreviewEntry {
            class_name: class.name.clone(),
            name: "composition".to_string(),
            baseline_path,
            result: comp_result,
        }))),
        Err(Error::Diag {
            code:
                "composition-delta-malformed"
                | "composition-delta-empty"
                | "composition-delta-not-mapping"
                | "composition-baseline-malformed"
                | "composition-baseline-no-screens"
                | "composition-screen-conflict"
                | "composition-serialize-failed",
            detail,
        }) => Ok(Some(Err(format!("composition: {detail}")))),
        Err(other) => Err(other),
    }
}

/// A3 precondition: refuse a whole-document (`screens:`) slice
/// composition that would silently replace a non-empty baseline.
///
/// Reads the slice's top-level `composition.yaml` and the baseline at
/// `class.baseline_dir/composition.yaml`. The gate fires — returning
/// [`Error::Diag { code: "composition-baseline-overwrite-blocked" }`] —
/// only when all three hold: the slice composition uses whole-document
/// replacement format, the baseline is non-empty, and `allow_replace`
/// is `false`. An absent slice file, an absent/empty baseline, a
/// `delta:`-format slice composition, or `allow_replace == true` all
/// pass.
///
/// This is a single-responsibility authorisation precondition — it
/// carries no merge mechanism. The pure shape predicates it composes
/// live in `crate::merge::composition`; this function adds only the
/// file reads and the authorisation policy (composition overwrite-gate placement).
///
/// # Errors
///
/// - [`Error::Diag { code: "composition-baseline-overwrite-blocked" }`]
///   when an unauthorised whole-document replacement is detected.
/// - [`Error::Diag { code: "composition-delta-malformed" }`] when the
///   slice composition does not parse (propagated from
///   [`crate::merge::composition::is_whole_document_replacement`]).
/// - [`Error::Filesystem`] (`op = "read"`) when the slice composition or
///   baseline file cannot be read.
pub(super) fn composition_overwrite_gate(
    slice_dir: &Path, class: &ArtifactClass, allow_replace: bool,
) -> Result<(), Error> {
    if allow_replace {
        return Ok(());
    }
    let Some(slice_text) = read_optional_file(&slice_dir.join(COMPOSITION_FILENAME))? else {
        return Ok(());
    };
    if !crate::merge::composition::is_whole_document_replacement(&slice_text)? {
        return Ok(());
    }
    let Some(baseline_text) = read_optional_file(&class.baseline_dir.join(COMPOSITION_FILENAME))?
    else {
        return Ok(());
    };
    if !crate::merge::composition::baseline_is_non_empty(&baseline_text) {
        return Ok(());
    }
    Err(Error::Diag {
        code: "composition-baseline-overwrite-blocked",
        detail: "Slice composition uses whole-document replacement format but a non-empty \
                 baseline exists. Use `delta:` format, or pass `--allow-composition-replace` to \
                 authorise full replacement."
            .to_string(),
    })
}

fn read_optional_file(path: &Path) -> Result<Option<String>, Error> {
    if path.is_file() {
        Ok(Some(fs::read_to_string(path).map_err(|err| Error::Filesystem {
            op: "read",
            path: path.to_path_buf(),
            source: err,
        })?))
    } else {
        Ok(None)
    }
}

/// Walk every [`MergeStrategy::OpaqueReplace`] class's `staged_dir`
/// and report each file that would be promoted, paired with whether
/// its baseline counterpart already exists ([`OpaqueAction::Replaced`])
/// or is brand new ([`OpaqueAction::Added`]).
pub(super) fn preview_opaque(classes: &[ArtifactClass]) -> Result<Vec<OpaquePreviewEntry>, Error> {
    let mut entries: Vec<OpaquePreviewEntry> = Vec::new();
    for class in classes.iter().filter(|c| matches!(c.strategy, MergeStrategy::OpaqueReplace)) {
        if !class.staged_dir.is_dir() {
            continue;
        }
        collect_opaque_entries(
            &class.staged_dir,
            &class.staged_dir,
            &class.baseline_dir,
            &class.name,
            &mut entries,
        )?;
    }
    entries.sort_by(|a, b| {
        (a.class_name.as_str(), a.relative_path.as_str())
            .cmp(&(b.class_name.as_str(), b.relative_path.as_str()))
    });
    Ok(entries)
}

fn collect_opaque_entries(
    base: &Path, current: &Path, baseline_dir: &Path, class_name: &str,
    entries: &mut Vec<OpaquePreviewEntry>,
) -> Result<(), Error> {
    for entry in fs::read_dir(current).map_err(|err| Error::Filesystem {
        op: "readdir",
        path: current.to_path_buf(),
        source: err,
    })? {
        let entry = entry.map_err(|err| Error::Filesystem {
            op: "dir-entry",
            path: current.to_path_buf(),
            source: err,
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_opaque_entries(base, &path, baseline_dir, class_name, entries)?;
        } else {
            let relative = path.strip_prefix(base).map_err(|_err| Error::Filesystem {
                op: "path-prefix",
                path: path.clone(),
                source: std::io::Error::other(format!(
                    "path {} is not under base {}",
                    path.display(),
                    base.display()
                )),
            })?;
            let baseline_path = baseline_dir.join(relative);
            let action =
                if baseline_path.is_file() { OpaqueAction::Replaced } else { OpaqueAction::Added };
            entries.push(OpaquePreviewEntry {
                class_name: class_name.to_string(),
                relative_path: relative.to_string_lossy().to_string(),
                action,
            });
        }
    }
    Ok(())
}

/// Recursively walk `current` (rooted at `base`) and check whether each
/// file's counterpart under `baseline_dir` has been modified after
/// `defined_at`. Files that exist only in the staged tree (not yet in
/// baseline) are skipped — they represent new artefacts, not drifted
/// ones.
pub(super) fn check_opaque_drift(
    base: &Path, current: &Path, baseline_dir: &Path, class_name: &str, defined_raw: &str,
    defined_at: Timestamp, conflicts: &mut Vec<BaselineConflict>,
) -> Result<(), Error> {
    if !current.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(current).map_err(|err| Error::Filesystem {
        op: "readdir",
        path: current.to_path_buf(),
        source: err,
    })? {
        let entry = entry.map_err(|err| Error::Filesystem {
            op: "dir-entry",
            path: current.to_path_buf(),
            source: err,
        })?;
        let path = entry.path();
        if path.is_dir() {
            check_opaque_drift(
                base,
                &path,
                baseline_dir,
                class_name,
                defined_raw,
                defined_at,
                conflicts,
            )?;
        } else {
            let relative = path.strip_prefix(base).map_err(|_err| Error::Filesystem {
                op: "path-prefix",
                path: path.clone(),
                source: std::io::Error::other(format!(
                    "path {} is not under base {}",
                    path.display(),
                    base.display()
                )),
            })?;
            let baseline_path = baseline_dir.join(relative);
            let meta = match fs::metadata(&baseline_path) {
                Ok(m) => m,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => return Err(Error::Io(err)),
            };
            let mtime = system_time_to_utc(meta.modified()?)?;
            if mtime > defined_at {
                conflicts.push(BaselineConflict {
                    adapter: format!("{class_name}/{}", relative.to_string_lossy()),
                    defined_at: defined_raw.to_string(),
                    baseline_modified_at: mtime,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
