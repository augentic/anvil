//! YAML delta merge for composition.yaml — screen-level operations
//! (added/modified/removed) applied to a baseline `screens` map.

use serde_json::Value;
use specify_error::Error;

use crate::merge::engine::{MergeOperation, MergeResult};

/// Merge a composition delta into an optional baseline.
///
/// `baseline` is the existing `composition.yaml` with a `screens` map (or None for new).
/// `delta_text` is the per-change `composition.yaml` — may have `screens` (new baseline)
/// or `delta` (screen-level operations).
///
/// # Errors
///
/// Returns an [`Error::Diag`] whose `code` names the failure: a
/// malformed/empty/non-mapping delta (`composition-delta-malformed`,
/// `composition-delta-empty`, `composition-delta-not-mapping`), an
/// unparseable or screens-less baseline (`composition-baseline-malformed`,
/// `composition-baseline-no-screens`), aggregated screen-level operation
/// conflicts (`composition-screen-conflict`), or a re-serialisation
/// failure (`composition-serialize-failed`).
pub fn merge(baseline: Option<&str>, delta_text: &str) -> Result<MergeResult, Error> {
    let delta_doc: Value = serde_saphyr::from_str(delta_text).map_err(|e| Error::Diag {
        code: "composition-delta-malformed",
        detail: format!("failed to parse composition delta: {e}"),
    })?;

    let has_screens = delta_doc.get("screens").is_some();
    let has_delta = delta_doc.get("delta").is_some();

    if has_screens && !has_delta {
        let screen_count =
            delta_doc.get("screens").and_then(|s| s.as_object()).map_or(0, serde_json::Map::len);
        return Ok(MergeResult {
            output: delta_text.to_string(),
            operations: vec![MergeOperation::CreatedBaseline {
                requirement_count: screen_count,
            }],
        });
    }

    if !has_delta {
        return Err(Error::Diag {
            code: "composition-delta-empty",
            detail: "composition delta has neither `screens` nor `delta`".to_string(),
        });
    }

    let delta = delta_doc.get("delta").and_then(|d| d.as_object()).ok_or_else(|| Error::Diag {
        code: "composition-delta-not-mapping",
        detail: "`delta` is not a mapping".to_string(),
    })?;

    let baseline_text = baseline.unwrap_or("");
    let mut baseline_doc: Value = if baseline_text.trim().is_empty() {
        serde_saphyr::from_str("version: 1\nscreens: {}").map_err(|e| Error::Diag {
            code: "composition-baseline-fallback-malformed",
            detail: format!("hardcoded empty baseline failed to parse: {e}"),
        })?
    } else {
        serde_saphyr::from_str(baseline_text).map_err(|e| Error::Diag {
            code: "composition-baseline-malformed",
            detail: format!("failed to parse composition baseline: {e}"),
        })?
    };

    let screens = baseline_doc
        .as_object_mut()
        .and_then(|m| m.get_mut("screens"))
        .and_then(|s| s.as_object_mut())
        .ok_or_else(|| Error::Diag {
            code: "composition-baseline-no-screens",
            detail: "baseline has no `screens` mapping".to_string(),
        })?;

    let mut operations: Vec<MergeOperation> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    if let Some(removed) = delta.get("removed").and_then(|r| r.as_object()) {
        for (slug, _) in removed {
            screens.remove(slug.as_str());
            operations.push(MergeOperation::Removed {
                id: slug.clone(),
                name: slug.clone(),
            });
        }
    }

    if let Some(added) = delta.get("added").and_then(|a| a.as_object()) {
        for (slug, screen_entry) in added {
            if screens.contains_key(slug.as_str()) {
                errors.push(format!(
                    "screen `{slug}` already exists in baseline; use `modified` to update it"
                ));
                continue;
            }
            screens.insert(slug.clone(), screen_entry.clone());
            operations.push(MergeOperation::Added {
                id: slug.clone(),
                name: slug.clone(),
            });
        }
    }

    if let Some(modified) = delta.get("modified").and_then(|m| m.as_object()) {
        for (slug, screen_entry) in modified {
            if !screens.contains_key(slug.as_str()) {
                errors.push(format!(
                    "screen `{slug}` not found in baseline; use `added` for new screens"
                ));
                continue;
            }
            screens.insert(slug.clone(), screen_entry.clone());
            operations.push(MergeOperation::Modified {
                id: slug.clone(),
                name: slug.clone(),
            });
        }
    }

    if !errors.is_empty() {
        return Err(Error::Diag {
            code: "composition-screen-conflict",
            detail: errors.join("\n"),
        });
    }

    let output = serde_saphyr::to_string(&baseline_doc).map_err(|e| Error::Diag {
        code: "composition-serialize-failed",
        detail: format!("failed to serialize merged composition: {e}"),
    })?;

    Ok(MergeResult { output, operations })
}

/// Pure shape predicate: does `text` use the whole-document
/// (`screens:`) replacement format rather than the incremental
/// (`delta:`) format?
///
/// Returns `true` iff a `screens` key is present and a `delta` key is
/// absent — mirroring the `has_screens && !has_delta` branch [`merge`]
/// takes when it treats a delta document as a wholesale baseline
/// replacement. No I/O and no policy: the authorisation decision (may a
/// whole-document replacement overwrite a non-empty baseline?) lives in
/// the merge-side `composition_overwrite_gate` precondition, not here.
///
/// # Errors
///
/// Returns [`Error::Diag { code: "composition-delta-malformed" }`] when
/// `text` does not parse as YAML — the same code [`merge`] raises for an
/// unparseable delta.
pub fn is_whole_document_replacement(text: &str) -> Result<bool, Error> {
    let doc: Value = serde_saphyr::from_str(text).map_err(|e| Error::Diag {
        code: "composition-delta-malformed",
        detail: format!("failed to parse composition delta: {e}"),
    })?;
    Ok(doc.get("screens").is_some() && doc.get("delta").is_none())
}

/// Pure shape predicate: is `text` a non-empty baseline composition?
///
/// Returns `true` iff `text` parses and its `screens` key is a mapping
/// with at least one entry. A malformed document, an absent/empty
/// `screens`, or a non-mapping `screens` all yield `false` — an empty or
/// unparseable baseline carries nothing the overwrite gate needs to
/// protect.
#[must_use]
pub fn baseline_is_non_empty(text: &str) -> bool {
    serde_saphyr::from_str::<Value>(text)
        .ok()
        .as_ref()
        .and_then(|doc| doc.get("screens"))
        .and_then(Value::as_object)
        .is_some_and(|screens| !screens.is_empty())
}

#[cfg(test)]
mod tests;
