//! Decision Record catalogue.
//!
//! The append-only baseline catalogue at `.specify/decisions/` is the
//! durable home for design *decisions* — the immutable "why" plus the
//! rejected alternatives. `specify slice merge` promotes each
//! slice-authored record under `.specify/slices/<slice>/decisions/` into
//! `.specify/decisions/DEC-NNNN-<slug>.md` by whole-file add (the same
//! opaque-add strategy contracts use), assigning the durable
//! project-global `DEC-NNNN` id; the only permitted mutation to an
//! existing record is flipping its `status` to `superseded` when a newer
//! record names it under `supersedes:`.
//!
//! This module owns the shared baseline reader ([`read_baseline`]) — used
//! by the merge promotion kernel ([`promote`]), the refine-gate
//! supersede-orphan check, and the identity projection — plus the
//! pure id-assignment / supersede-flip kernel that `merge` drives.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use specify_error::Error;
use specify_model::atomic::bytes_write;
use specify_model::decision::{self, DecisionRecord, DecisionStatus};

use crate::config::Layout;

#[cfg(test)]
mod tests;

/// Zero-padding width for a freshly assigned `DEC-NNNN` id. Existing
/// ids with more digits still sort and parse correctly; the pad only
/// keeps small catalogues tidy.
const ID_PAD: usize = 4;

/// One Decision Record resident in the baseline catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineDecision {
    /// Numeric component of the `DEC-NNNN` id.
    pub number: u32,
    /// The parsed front-matter (carries the stamped `id` / `slice` /
    /// `date` and current `status`).
    pub record: DecisionRecord,
    /// The record's H1 heading text, projected into routing identity
    /// as the decision title.
    pub title: Option<String>,
    /// The Markdown body below the front-matter, preserved verbatim so
    /// a supersede flip leaves the historical rationale untouched.
    pub body: String,
    /// On-disk path of the baseline file.
    pub path: PathBuf,
}

impl BaselineDecision {
    /// The `DEC-NNNN` id, or an empty string when the record lacks one
    /// (never the case for a well-formed baseline file).
    #[must_use]
    pub fn id(&self) -> &str {
        self.record.id.as_deref().unwrap_or_default()
    }
}

/// Parse the numeric component of a `DEC-NNNN` id. Returns `None` when
/// the string is not a `DEC-` prefixed run of ASCII digits.
#[must_use]
fn dec_number(id: &str) -> Option<u32> {
    id.strip_prefix("DEC-").filter(|tail| !tail.is_empty()).and_then(|tail| tail.parse().ok())
}

/// `true` when `target` is a `DEC-NNNN` id reference (vs a slug).
#[must_use]
pub fn is_dec_ref(target: &str) -> bool {
    target.strip_prefix("DEC-").is_some_and(is_ascii_digits)
}

fn is_ascii_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// List every `*.md` file directly under `dir`, sorted by path for
/// deterministic iteration. The caller is responsible for confirming the
/// directory exists (an absent `decisions/` is an opt-out, not an error).
///
/// # Errors
///
/// Surfaces I/O errors reading the directory or one of its entries.
pub(crate) fn list_md_files(dir: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut out: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|source| Error::Filesystem {
        op: "readdir",
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| Error::Filesystem {
            op: "readdir-entry",
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// Read the baseline catalogue at `.specify/decisions/`, returning every
/// record sorted by `DEC-NNNN` ascending. A missing directory yields an
/// empty vector.
///
/// # Errors
///
/// Surfaces I/O errors reading the directory or a record, and
/// `Error::Diag { code: "decision-baseline-malformed" }` when a baseline
/// file cannot be parsed (it is machine-written, so this signals
/// corruption).
pub fn read_baseline(decisions_dir: &Path) -> Result<Vec<BaselineDecision>, Error> {
    if !decisions_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out: Vec<BaselineDecision> = Vec::new();
    for path in list_md_files(decisions_dir)? {
        let text = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.clone(),
            source,
        })?;
        let (record, body) = parse_file(&text).ok_or_else(|| Error::Diag {
            code: "decision-baseline-malformed",
            detail: format!("baseline decision `{}` could not be parsed", path.display()),
        })?;
        let number = record.id.as_deref().and_then(dec_number).ok_or_else(|| Error::Diag {
            code: "decision-baseline-malformed",
            detail: format!("baseline decision `{}` has no `DEC-NNNN` id", path.display()),
        })?;
        let title = decision::parse_decision(&text).title;
        out.push(BaselineDecision {
            number,
            record,
            title,
            body: body.to_string(),
            path,
        });
    }
    out.sort_by_key(|d| d.number);
    Ok(out)
}

/// A slice-authored record awaiting promotion.
struct SliceRecord {
    slug: String,
    record: DecisionRecord,
    body: String,
}

/// Read the slice-authored records under `<slice>/decisions/`, sorted by
/// slug (the deterministic promotion order).
fn read_slice_records(src: &Path) -> Result<Vec<SliceRecord>, Error> {
    if !src.is_dir() {
        return Ok(Vec::new());
    }
    let mut out: Vec<SliceRecord> = Vec::new();
    for path in list_md_files(src)? {
        let text = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.clone(),
            source,
        })?;
        let (record, body) = parse_file(&text).ok_or_else(|| Error::Diag {
            code: "decision-record-malformed",
            detail: format!(
                "slice decision `{}` could not be parsed; run `specify slice validate` first",
                path.display()
            ),
        })?;
        out.push(SliceRecord {
            slug: record.slug.clone(),
            record,
            body: body.to_string(),
        });
    }
    out.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(out)
}

/// Split + deserialize one Decision Record file into its front-matter
/// record and verbatim body. `None` when the front-matter is missing or
/// does not deserialize.
fn parse_file(text: &str) -> Option<(DecisionRecord, &str)> {
    let (front, body) = decision::split_frontmatter(text)?;
    let record = serde_saphyr::from_str::<DecisionRecord>(front).ok()?;
    Some((record, body))
}

/// A pending supersede flip resolved during promotion.
enum FlipTarget {
    /// An index into the pre-merge baseline vector.
    Baseline(usize),
    /// An index into this merge's freshly promoted records.
    New(usize),
}

/// A freshly stamped record staged for writing during promotion.
struct NewWrite {
    id: String,
    slug: String,
    record: DecisionRecord,
    body: String,
}

/// Promote every slice-authored Decision Record into the baseline
/// catalogue, returning the assigned `DEC-NNNN` ids in slug order.
///
/// Records land at `.specify/decisions/DEC-NNNN-<slug>.md` by whole-file
/// add; each `supersedes:` target (a baseline `DEC-NNNN` or a slug
/// merged earlier in this slice) is re-resolved against the live
/// baseline ∪ ids assigned earlier in this same merge and its target
/// flipped to `status: superseded` + `superseded-by`. A no-op (empty
/// vector) when the slice authored no records.
///
/// # Errors
///
/// - `Error::Validation { code: "decision-supersede-orphan" }` when a
///   `supersedes:` target resolves to neither the baseline nor an
///   earlier sibling — the blocking re-check the refine gate cannot make
///   (the baseline may move between refine and merge).
/// - `Error::Diag { code: "decision-record-malformed" }` when a staged
///   record cannot be parsed (validate should have caught it first).
/// - I/O errors writing the baseline files.
pub fn promote(
    slice_dir: &Path, project_dir: &Path, slice_name: &str, now: Timestamp,
) -> Result<Vec<String>, Error> {
    let slice_records = read_slice_records(&slice_dir.join("decisions"))?;
    if slice_records.is_empty() {
        return Ok(Vec::new());
    }

    let decisions_dir = Layout::new(project_dir).decisions_dir();
    let mut baseline = read_baseline(&decisions_dir)?;

    let baseline_idx_by_id: HashMap<String, usize> =
        baseline.iter().enumerate().map(|(i, b)| (b.id().to_string(), i)).collect();
    let baseline_idx_by_slug: HashMap<String, usize> =
        baseline.iter().enumerate().map(|(i, b)| (b.record.slug.clone(), i)).collect();

    // Idempotency guard: a `(slice, slug)` already in the baseline was
    // promoted by an earlier (possibly partially failed) run of this same
    // merge. Skip it so a retry never assigns a duplicate `DEC-NNNN`.
    let already_promoted: std::collections::HashSet<(String, String)> = baseline
        .iter()
        .filter_map(|b| b.record.slice.clone().map(|slice| (slice, b.record.slug.clone())))
        .collect();

    let mut next = baseline.iter().map(|b| b.number).max().unwrap_or(0) + 1;
    let date = now.strftime("%Y-%m-%d").to_string();

    let mut writes: Vec<NewWrite> = Vec::new();
    let mut new_idx_by_slug: HashMap<String, usize> = HashMap::new();
    let mut assigned: Vec<String> = Vec::new();
    let mut flips: Vec<(FlipTarget, String)> = Vec::new();

    for sr in slice_records {
        if already_promoted.contains(&(slice_name.to_string(), sr.slug.clone())) {
            continue;
        }
        let id = format!("DEC-{next:0ID_PAD$}");
        next += 1;
        let mut record = sr.record;
        record.id = Some(id.clone());
        record.slice = Some(slice_name.to_string());
        record.date = Some(date.clone());

        for target in &record.supersedes {
            let flip = resolve_target(
                target,
                &baseline_idx_by_id,
                &baseline_idx_by_slug,
                &new_idx_by_slug,
            )
            .ok_or_else(|| {
                Error::validation_failed(
                    "decision-supersede-orphan",
                    "every `supersedes:` target resolves to a baseline DEC or earlier sibling",
                    format!(
                        "decision `{}` (slug `{}`) supersedes `{target}`, which resolves to \
                         neither the baseline catalogue nor an earlier record in this slice",
                        id, record.slug
                    ),
                )
            })?;
            flips.push((flip, id.clone()));
        }

        let new_index = writes.len();
        new_idx_by_slug.insert(sr.slug.clone(), new_index);
        assigned.push(id.clone());
        writes.push(NewWrite {
            id,
            slug: sr.slug,
            record,
            body: sr.body,
        });
    }

    // Apply flips in memory before any write so an orphan aborts cleanly.
    let mut flipped_baseline: Vec<usize> = Vec::new();
    for (target, by) in flips {
        match target {
            FlipTarget::Baseline(i) => {
                baseline[i].record.status = DecisionStatus::Superseded;
                baseline[i].record.superseded_by = Some(by);
                flipped_baseline.push(i);
            }
            FlipTarget::New(i) => {
                writes[i].record.status = DecisionStatus::Superseded;
                writes[i].record.superseded_by = Some(by);
            }
        }
    }

    for w in &writes {
        let path = decisions_dir.join(format!("{}-{}.md", w.id, w.slug));
        write_record(&path, &w.record, &w.body)?;
    }
    flipped_baseline.sort_unstable();
    flipped_baseline.dedup();
    for i in flipped_baseline {
        let b = &baseline[i];
        write_record(&b.path, &b.record, &b.body)?;
    }

    Ok(assigned)
}

/// Resolve a `supersedes:` target to a flip. A `DEC-NNNN` target
/// resolves against the baseline; a slug resolves first against an
/// earlier sibling promoted in this merge, then against the baseline.
fn resolve_target(
    target: &str, baseline_by_id: &HashMap<String, usize>,
    baseline_by_slug: &HashMap<String, usize>, new_by_slug: &HashMap<String, usize>,
) -> Option<FlipTarget> {
    if is_dec_ref(target) {
        return baseline_by_id.get(target).copied().map(FlipTarget::Baseline);
    }
    if let Some(&i) = new_by_slug.get(target) {
        return Some(FlipTarget::New(i));
    }
    baseline_by_slug.get(target).copied().map(FlipTarget::Baseline)
}

/// Serialise a record's front-matter and write `---\n<yaml>---\n<body>`
/// to `path` via [`bytes_write`] (temp-file + rename, creating the parent
/// chain) so a crash mid-merge never leaves a half-written record.
/// Atomicity is per file, not across the promotion set: the caller flips
/// the slice metadata only after every write returns, so an interrupted
/// promotion is safely re-runnable.
fn write_record(path: &Path, record: &DecisionRecord, body: &str) -> Result<(), Error> {
    let yaml = serde_saphyr::to_string(record)?;
    let yaml = if yaml.ends_with('\n') { yaml } else { format!("{yaml}\n") };
    let content = format!("---\n{yaml}---\n{body}");
    bytes_write(path, content.as_bytes()).map_err(|err| match err {
        Error::Io(source) => Error::Filesystem {
            op: "write",
            path: path.to_path_buf(),
            source,
        },
        other => other,
    })
}
