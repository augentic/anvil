//! Pure in-memory delta merge: applies a delta spec's
//! ADDED / MODIFIED / REMOVED / RENAMED sections to a baseline.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use specify_error::Error;
use specify_model::spec::{
    DeltaSpec, REQ_HEADING, Rename, Requirement, has_delta_headers, parse_baseline, parse_delta,
};

/// Result of a successful [`merge`] call.
///
/// `output` is the merged baseline text (pinned byte-for-byte by the
/// merge-engine goldens). `operations` records every change applied, in
/// the order `RENAMED → REMOVED → MODIFIED → ADDED` — the same order
/// used when mutating the underlying block list.
///
/// The `Serialize` derive omits `output` so the type can be `#[serde(flatten)]`-ed
/// into wire envelopes (e.g. `MergePreviewEntry`) that carry only the
/// operations list — the merged text travels separately to disk via
/// the commit writer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[must_use]
pub struct MergeResult {
    /// Merged baseline text.
    #[serde(skip)]
    pub output: String,
    /// Ordered list of changes applied during the merge.
    pub operations: Vec<MergeOperation>,
}

/// One structured entry in [`MergeResult::operations`].
///
/// `CreatedBaseline` is the "no delta headers, baseline was empty" branch:
/// the delta text is kept verbatim as the new baseline and we just record
/// how many `### Requirement:` blocks it contains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MergeOperation {
    /// A requirement was renamed (ID preserved, heading changed).
    Renamed {
        /// Requirement ID.
        id: String,
        /// Previous name.
        old_name: String,
        /// New name.
        new_name: String,
    },
    /// A requirement was removed from the baseline.
    Removed {
        /// Requirement ID.
        id: String,
        /// Requirement name.
        name: String,
    },
    /// A requirement body was replaced.
    Modified {
        /// Requirement ID.
        id: String,
        /// Requirement name.
        name: String,
    },
    /// A new requirement was appended.
    Added {
        /// Requirement ID.
        id: String,
        /// Requirement name.
        name: String,
    },
    /// Baseline created from scratch (no delta headers present).
    CreatedBaseline {
        /// Number of `### Requirement:` blocks found in the verbatim text.
        requirement_count: usize,
    },
}

/// Merge a delta spec into an optional baseline.
///
/// `baseline == None` (or `Some("")`, or `Some(whitespace-only)`) means
/// "new adapter": the baseline is being created from scratch. In that
/// case:
///   * if the delta has **no** delta-section headers (per
///     [`specify_model::spec::has_delta_headers`]), the delta text is returned
///     verbatim and `operations` holds a single
///     [`MergeOperation::CreatedBaseline`] entry whose `requirement_count`
///     counts the `### Requirement:` blocks found in the delta body;
///   * otherwise the `## ADDED Requirements` section is flattened into a
///     fresh baseline.
///
/// `baseline = Some(non-empty)` applies `RENAMED → REMOVED → MODIFIED →
/// ADDED` in that order. Any delta entry whose id cannot be resolved (or,
/// for ADDED, whose id collides with a surviving baseline id) becomes an
/// `Err(Error::Diag { code: "merge-spec-conflicts", .. })` with all
/// failure messages joined by `"\n"`.
///
/// # Errors
///
/// [`Error::Diag`] `merge-spec-conflicts` when one or more delta blocks
/// cannot be applied — a `RENAMED`/`MODIFIED`/`REMOVED` id missing from
/// the baseline, or an `ADDED` id that already exists. All such
/// conflicts are aggregated into the one error.
pub fn merge(baseline: Option<&str>, delta: &str) -> Result<MergeResult, Error> {
    let baseline_text = baseline.unwrap_or("");
    let delta_spec = parse_delta(delta);
    if baseline_text.trim().is_empty() {
        Ok(merge_into_empty_baseline(delta, &delta_spec))
    } else {
        merge_into_existing_baseline(baseline_text, &delta_spec)
    }
}

fn merge_into_empty_baseline(delta: &str, delta_spec: &DeltaSpec) -> MergeResult {
    // `has_delta_headers` uses a full-line match; see
    // `has_delta_headers_requires_full_line_match` in the spec crate.
    if !has_delta_headers(delta) {
        let requirement_count = count_requirement_headings(delta);
        return MergeResult {
            output: delta.to_string(),
            operations: vec![MergeOperation::CreatedBaseline { requirement_count }],
        };
    }

    let mut operations: Vec<MergeOperation> = Vec::new();
    let mut result_blocks: Vec<String> = Vec::new();
    for block in &delta_spec.added {
        result_blocks.push(block.body.clone());
        operations.push(MergeOperation::Added {
            id: block.id.clone(),
            name: block.name.clone(),
        });
    }
    let output = if result_blocks.is_empty() {
        String::new()
    } else {
        let mut joined = result_blocks.join("\n\n");
        joined.push('\n');
        joined
    };
    MergeResult { output, operations }
}

fn merge_into_existing_baseline(
    baseline_text: &str, delta_spec: &DeltaSpec,
) -> Result<MergeResult, Error> {
    let parsed_baseline = parse_baseline(baseline_text);
    let mut blocks: Vec<Requirement> = parsed_baseline.requirements;
    let preamble = parsed_baseline.preamble;

    let mut blocks_by_id: HashMap<String, usize> = HashMap::new();
    for (i, block) in blocks.iter().enumerate() {
        if !block.id.is_empty() {
            blocks_by_id.insert(block.id.clone(), i);
        }
    }

    let mut errors: Vec<String> = Vec::new();
    let mut operations: Vec<MergeOperation> = Vec::new();

    apply_renamed(&delta_spec.renamed, &mut blocks, &blocks_by_id, &mut operations, &mut errors);
    let ids_to_remove =
        apply_removed(&delta_spec.removed, &blocks_by_id, &mut operations, &mut errors);
    apply_modified(&delta_spec.modified, &mut blocks, &blocks_by_id, &mut operations, &mut errors);
    apply_added(
        &delta_spec.added,
        &mut blocks,
        &blocks_by_id,
        &ids_to_remove,
        &mut operations,
        &mut errors,
    );

    if !errors.is_empty() {
        return Err(Error::Diag {
            code: "merge-spec-conflicts",
            detail: errors.join("\n"),
        });
    }

    Ok(MergeResult {
        output: assemble_merged_output(&preamble, &blocks, &ids_to_remove),
        operations,
    })
}

fn apply_renamed(
    renamed: &[Rename], blocks: &mut [Requirement], blocks_by_id: &HashMap<String, usize>,
    operations: &mut Vec<MergeOperation>, errors: &mut Vec<String>,
) {
    for entry in renamed {
        let Some(&idx) = blocks_by_id.get(&entry.id) else {
            errors.push(format!("RENAMED: ID {} not found in baseline", entry.id));
            continue;
        };
        let old_block = blocks[idx].clone();
        let new_heading = format!("{} {}", REQ_HEADING, entry.new_name);
        let new_body = replace_first(&old_block.body, &old_block.heading, &new_heading);
        operations.push(MergeOperation::Renamed {
            id: old_block.id.clone(),
            old_name: old_block.name.clone(),
            new_name: entry.new_name.clone(),
        });
        blocks[idx] = Requirement {
            heading: new_heading,
            name: entry.new_name.clone(),
            id: old_block.id,
            body: new_body,
            scenarios: old_block.scenarios,
        };
    }
}

fn apply_removed(
    removed: &[Requirement], blocks_by_id: &HashMap<String, usize>,
    operations: &mut Vec<MergeOperation>, errors: &mut Vec<String>,
) -> HashSet<String> {
    let mut ids_to_remove: HashSet<String> = HashSet::new();
    for block in removed {
        if blocks_by_id.contains_key(&block.id) {
            ids_to_remove.insert(block.id.clone());
            operations.push(MergeOperation::Removed {
                id: block.id.clone(),
                name: block.name.clone(),
            });
        } else {
            errors.push(format!("REMOVED: ID {} not found in baseline", block.id));
        }
    }
    ids_to_remove
}

fn apply_modified(
    modified: &[Requirement], blocks: &mut [Requirement], blocks_by_id: &HashMap<String, usize>,
    operations: &mut Vec<MergeOperation>, errors: &mut Vec<String>,
) {
    for mod_block in modified {
        let Some(&idx) = blocks_by_id.get(&mod_block.id) else {
            errors.push(format!("MODIFIED: ID {} not found in baseline", mod_block.id));
            continue;
        };
        operations.push(MergeOperation::Modified {
            id: mod_block.id.clone(),
            name: mod_block.name.clone(),
        });
        blocks[idx] = mod_block.clone();
    }
}

fn apply_added(
    added: &[Requirement], blocks: &mut Vec<Requirement>, blocks_by_id: &HashMap<String, usize>,
    ids_to_remove: &HashSet<String>, operations: &mut Vec<MergeOperation>,
    errors: &mut Vec<String>,
) {
    let mut existing_ids: HashSet<String> =
        blocks_by_id.keys().filter(|id| !ids_to_remove.contains(*id)).cloned().collect();
    for add_block in added {
        if !add_block.id.is_empty() && existing_ids.contains(&add_block.id) {
            errors.push(format!("ADDED: ID {} already exists in baseline", add_block.id));
            continue;
        }
        operations.push(MergeOperation::Added {
            id: add_block.id.clone(),
            name: add_block.name.clone(),
        });
        blocks.push(add_block.clone());
        if !add_block.id.is_empty() {
            existing_ids.insert(add_block.id.clone());
        }
    }
}

fn assemble_merged_output(
    preamble: &str, blocks: &[Requirement], ids_to_remove: &HashSet<String>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !preamble.trim().is_empty() {
        parts.push(rstrip(preamble).to_string());
    }
    for block in blocks {
        if ids_to_remove.contains(&block.id) && !block.id.is_empty() {
            continue;
        }
        parts.push(block.body.trim().to_string());
    }
    let mut output = parts.join("\n\n");
    output.push('\n');
    output
}

fn count_requirement_headings(text: &str) -> usize {
    text.lines().filter(|line| line.trim_start().starts_with(REQ_HEADING)).count()
}

/// Replace only the first occurrence of `needle`; an empty `needle`
/// returns `haystack` unchanged.
fn replace_first(haystack: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return haystack.to_string();
    }
    haystack.find(needle).map_or_else(
        || haystack.to_string(),
        |idx| {
            let mut out = String::with_capacity(haystack.len() + replacement.len());
            out.push_str(&haystack[..idx]);
            out.push_str(replacement);
            out.push_str(&haystack[idx + needle.len()..]);
            out
        },
    )
}

fn rstrip(s: &str) -> &str {
    s.trim_end_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c'])
}

#[cfg(test)]
mod tests;
