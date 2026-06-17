//! Slice-validation kernel for `specify slice validate`.
//!
//! Holds the pure, `Ctx`-free gate logic the handler orchestrates: the
//! pre-adapter gates ([`pre_adapter_gates`]) — provenance scan, spec
//! file-location, per-slice authority-override orphans, component-catalog
//! drift, typed-model drift, and Decision
//! Record gates — plus the non-blocking synopsis advisory and the
//! synthesis journal emission. Every entry point takes a [`Layout`] or
//! plain paths rather than the CLI `Ctx`, so the gates are unit-testable
//! without standing up a binary. Adapter validation (`validate_slice`,
//! from `specify-model`'s `validate` registry) and report rendering stay
//! in the handler, keeping this kernel free of the registry and of
//! output concerns.
//!
//! This module is the thin orchestrator: it owns the public entry
//! points ([`pre_adapter_gates`], [`append_synthesis_journal`], the
//! [`PreAdapter`] outcome) and the two filesystem helpers shared across
//! gates (`path_hint`, `collect_spec_files`). The gate machinery
//! lives in the cohesive submodules — `pre_adapter` (spec scan, gate
//! bundle, authority overrides, synopsis advisory), `model_drift`,
//! `decisions`, `catalog`, and `spec_location`.

use std::path::{Path, PathBuf};

use jiff::Timestamp;
use specify_diagnostics::Diagnostic;
use specify_error::{Error, Result};
use specify_model::spec::provenance::RequirementTag;

use crate::config::Layout;
use crate::journal::{Event, EventKind, append_batch};
use crate::schema::validate_evidence_dir;

mod catalog;
mod decisions;
mod model_drift;
mod pre_adapter;
mod spec_location;
#[cfg(test)]
mod tests;

/// Outcome of the pre-adapter gate sweep ([`pre_adapter_gates`]).
///
/// `slice validate` runs structural gates before invoking the target
/// adapter's rules; a firing gate short-circuits adapter validation so
/// the operator sees the structural cause first.
#[derive(Debug)]
pub enum PreAdapter {
    /// A gate fired. `code` is the error discriminant the handler raises
    /// after rendering `findings`; `findings` are the blocking diagnostics
    /// for that gate.
    Gate {
        /// Stable `Error::Validation` discriminant for the failing gate.
        code: &'static str,
        /// Blocking diagnostics to render before failing.
        findings: Vec<Diagnostic>,
    },
    /// Every gate passed. The handler proceeds to adapter validation,
    /// folds `advisories` into the adapter findings, and — on overall
    /// success — journals `synthesis_tags`.
    Proceed {
        /// `(requirement-id, tag)` pairs to journal on overall success.
        synthesis_tags: Vec<(String, RequirementTag)>,
        /// Non-blocking advisories (synopsis content-floor) to render
        /// alongside the adapter findings.
        advisories: Vec<Diagnostic>,
    },
}

/// Run the pre-adapter gate sweep for slice `name`.
///
/// First-use schema validation of per-source `Evidence` files runs first
/// (per workflow §Source adapter contract); a structural Evidence problem
/// short-circuits with [`Error`] before any gate so the operator sees it
/// before downstream artefact noise. Then the provenance scan and the
/// pre-adapter gates fire in order, each able to return
/// [`PreAdapter::Gate`]. When all gates pass, returns
/// [`PreAdapter::Proceed`] carrying the synthesis tags and the synopsis
/// advisory surface.
///
/// # Errors
///
/// Returns [`Error`] when Evidence schema validation fails, or when a
/// plan, spec, model, discovery, decision, or Evidence file cannot be
/// read or parsed.
pub fn pre_adapter_gates(layout: Layout<'_>, name: &str) -> Result<PreAdapter> {
    let slice_dir = layout.slices_dir().join(name);
    let evidence_docs = validate_evidence_dir(&slice_dir)?;

    let source_keys = pre_adapter::resolve_slice_source_keys(layout, name)?;
    let (_spec_req_ids, synthesis_tags, provenance_findings) =
        pre_adapter::scan_slice_specs(&slice_dir, &source_keys)?;
    if !provenance_findings.is_empty() {
        return Ok(PreAdapter::Gate {
            code: "slice-provenance-invalid",
            findings: provenance_findings,
        });
    }

    let gate_findings =
        pre_adapter::collect_pre_adapter_gates(layout, &slice_dir, name, &evidence_docs)?;
    if !gate_findings.is_empty() {
        return Ok(PreAdapter::Gate {
            code: "slice-pre-adapter-gate",
            findings: gate_findings,
        });
    }

    let advisories = pre_adapter::synopsis_thin(layout)?;
    Ok(PreAdapter::Proceed {
        synthesis_tags,
        advisories,
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
        let entries = std::fs::read_dir(&dir).map_err(|source| Error::Filesystem {
            op: "readdir",
            path: dir.clone(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| Error::Filesystem {
                op: "readdir-entry",
                path: dir.clone(),
                source,
            })?;
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
