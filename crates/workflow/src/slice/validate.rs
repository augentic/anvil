//! Slice-validation kernel shared by `specify slice validate` and the
//! guest refine orchestrator.
//!
//! [`run`] collects the ordered pre-adapter gates, then folds adapter
//! findings with non-blocking synopsis advisories. Rendering, journaling,
//! and error envelopes remain at the caller boundaries.

use std::path::{Path, PathBuf};

use artifacts::spec::provenance::RequirementTag;
use error::Result;
use jiff::Timestamp;
use schema::diagnostics::Diagnostic;

use crate::config::Layout;
use crate::journal::{Event, EventKind, append_batch};
use crate::schema_gate::validate_evidence_dir;

mod catalog;
mod decisions;
mod model_drift;
mod pre_adapter;
mod spec_location;

/// Outcome of the full validation sweep ([`run`]).
///
/// The structural gates run before the target adapter's rules; a firing
/// gate short-circuits adapter validation so the operator sees the
/// structural cause first.
#[derive(Debug)]
pub enum Validation {
    /// A pre-adapter gate fired; adapter validation did not run. `code`
    /// is the error discriminant the caller raises after surfacing
    /// `findings` — the blocking diagnostics for that gate.
    Gate {
        /// Stable `Error::Validation` discriminant for the failing gate.
        code: &'static str,
        /// Blocking diagnostics to surface before failing.
        findings: Vec<Diagnostic>,
    },
    /// Every gate passed and the adapter rules ran. `findings` carries
    /// the adapter diagnostics folded with the non-blocking synopsis
    /// advisories; the caller fails with `slice-validation-failed` when
    /// any finding blocks, and — on overall success — journals
    /// `synthesis_tags` via [`append_synthesis_journal`].
    Adapter {
        /// Adapter diagnostics plus non-blocking advisories.
        findings: Vec<Diagnostic>,
        /// `(requirement-id, tag)` pairs to journal on overall success.
        synthesis_tags: Vec<(String, RequirementTag)>,
    },
}

/// Run the full validation sweep for slice `name`: the pre-adapter
/// gates, then the adapter rules with the advisory fold.
///
/// First-use schema validation of per-source `Evidence` files runs first
/// (per workflow §Source adapter contract); a structural Evidence problem
/// short-circuits with [`error::Error`] before any gate so the operator sees it
/// before downstream artefact noise. Then the provenance scan and the
/// pre-adapter gates fire in order, each able to return
/// [`Validation::Gate`]. When all gates pass, the adapter rules run and
/// the sweep returns [`Validation::Adapter`].
///
/// # Errors
///
/// Returns [`error::Error`] when Evidence schema validation fails, or when a
/// plan, spec, model, discovery, decision, or Evidence file cannot be
/// read or parsed.
pub fn run(layout: Layout<'_>, name: &str) -> Result<Validation> {
    let slice_dir = layout.slice_dir(name);
    let evidence_docs = validate_evidence_dir(&slice_dir)?;

    let source_keys = pre_adapter::source_keys(layout, name)?;
    let (_spec_req_ids, synthesis_tags, provenance_findings) =
        pre_adapter::scan_specs(&slice_dir, &source_keys)?;
    if !provenance_findings.is_empty() {
        return Ok(Validation::Gate {
            code: "slice-provenance-invalid",
            findings: provenance_findings,
        });
    }

    let gate_findings = pre_adapter::gates(layout, &slice_dir, name, &evidence_docs)?;
    if !gate_findings.is_empty() {
        return Ok(Validation::Gate {
            code: "slice-pre-adapter-gate",
            findings: gate_findings,
        });
    }

    // Adapter validation findings — `validate_slice` returns one
    // `violation` diagnostic per structural Fail and one `review`
    // diagnostic per deferred semantic rule. The non-blocking
    // `discovery-lead-synopsis-thin` advisories ride this surface too;
    // only a blocking diagnostic gates the caller's exit.
    let mut findings = artifacts::validate::validate_slice(&slice_dir)?;
    findings.append(&mut pre_adapter::synopsis_thin(layout)?);
    Ok(Validation::Adapter {
        findings,
        synthesis_tags,
    })
}

/// Append one `slice.synthesis.*` journal line per `(requirement-id,
/// tag)` pair gathered during the spec scan.
///
/// Each event is stamped with the dispatcher-injected `now` (workflow
/// §Time injection). Skipped when the slice has no tagged requirements.
///
/// # Errors
///
/// Propagates the journal write error from [`append_batch`].
pub fn append_synthesis_journal(
    layout: Layout<'_>, now: Timestamp, slice_name: &str, tags: Vec<(String, RequirementTag)>,
) -> Result<()> {
    if tags.is_empty() {
        return Ok(());
    }
    let events: Vec<Event> = tags
        .into_iter()
        .map(|(requirement_id, tag)| {
            let kind = match tag {
                RequirementTag::Unknown => EventKind::SliceSynthesisUnknown {
                    slice_name: slice_name.into(),
                    requirement_id,
                },
                RequirementTag::Conflict => EventKind::SliceSynthesisConflict {
                    slice_name: slice_name.into(),
                    requirement_id,
                },
                RequirementTag::Divergence => EventKind::SliceSynthesisDivergence {
                    slice_name: slice_name.into(),
                    requirement_id,
                },
            };
            Event::new(now, kind)
        })
        .collect();
    append_batch(layout, &events)
}

/// Path the operator sees in each finding's detail. Anchored at the
/// slice directory so the printed string is `specs/<group>/spec.md`
/// rather than an absolute tempdir path that varies per test run.
fn path_hint(path: &Path, slice_dir: &Path) -> String {
    let rel = path.strip_prefix(slice_dir).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Recursive walk of `<slice>/specs/` collecting every `*.md` file.
/// Hand-rolled (rather than reaching for `glob`) so the call site
/// stays auditable on the operator path. Sorted for stable error
/// ordering.
fn collect_spec_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in crate::fs::dir_entries(&dir)? {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}
