//! Source-axis gates: Evidence documents and survey leads.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use artifacts::discovery::Lead;
use error::{Error, Result};
use schema::{
    EVIDENCE_JSON_SCHEMA, LEAD_JSON_SCHEMA, ValidationStatus, ValidationSummary, join_details,
    read_yaml_as_json, validate_value_cached,
};
use serde_json::Value as JsonValue;

use super::support::{relabel, validate_labelled_yaml};

/// Sorted paths to `.yaml`/`.yml` files under `<slice_dir>/evidence/`.
///
/// The walk is non-recursive: only direct children of `evidence/` whose
/// extension is `yaml` or `yml` are considered. Returns an empty
/// vector when `evidence/` is missing or not a directory.
///
/// # Errors
///
/// - [`Error::Filesystem`] if `evidence/` exists but cannot be read.
pub fn evidence_yaml_paths(slice_dir: &Path) -> Result<Vec<PathBuf>> {
    let evidence_dir = slice_dir.join("evidence");
    if !evidence_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in crate::fs::dir_entries(&evidence_dir)? {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(OsStr::to_str).unwrap_or("");
        if ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// One parsed `evidence/<source>.yaml` from a single read.
///
/// Read and schema-validated once by [`validate_evidence_dir`] and
/// handed back so downstream slice gates (catalog-drift and
/// model-drift) can reuse the parsed document instead of re-reading and
/// re-parsing the file from disk.
#[derive(Debug)]
pub struct EvidenceDoc {
    /// Path the document was read from, retained for per-file error
    /// attribution and source-key derivation (the file stem) in the
    /// consuming gates.
    pub path: PathBuf,
    /// The parsed YAML-as-JSON document — byte-identical to the value
    /// the consuming gates would have produced by re-reading the file.
    pub value: JsonValue,
}

/// Validate every `*.yaml` file under `<slice_dir>/evidence/` against
/// the embedded `schemas/evidence.schema.json`.
///
/// `slice_dir` is the directory typically at
/// `.specify/slices/<name>/`. The evidence subdirectory is optional —
/// returning an empty `Vec` when it is absent matches the workflow
/// §Extraction reliability rule that an empty `claims: []` (or no
/// Evidence at all before extract runs) is valid.
///
/// All findings are aggregated and returned in a single
/// [`Error::Validation`] so the caller sees every malformed file in
/// one pass. On a clean validation the parsed documents are returned
/// (in sorted-path order) so the slice-validate drift gates can reuse
/// them without a second read or parse.
///
/// # Errors
///
/// - [`Error::Filesystem`] if `evidence/` exists but cannot be read.
/// - [`Error::Validation`] if any Evidence file fails YAML parse or
///   schema validation.
pub fn validate_evidence_dir(slice_dir: &Path) -> Result<Vec<EvidenceDoc>> {
    let paths = evidence_yaml_paths(slice_dir)?;

    let mut docs: Vec<EvidenceDoc> = Vec::with_capacity(paths.len());
    let mut summaries: Vec<ValidationSummary> = Vec::new();
    for path in paths {
        match read_yaml_as_json(&path) {
            Ok(instance) => {
                for summary in validate_value_cached(
                    &instance,
                    EVIDENCE_JSON_SCHEMA,
                    "evidence-schema",
                    "evidence file conforms to schemas/evidence.schema.json",
                ) {
                    if summary.status == ValidationStatus::Fail {
                        summaries.push(relabel(summary, path.display()));
                    }
                }
                docs.push(EvidenceDoc {
                    path,
                    value: instance,
                });
            }
            Err(err) => {
                summaries.push(ValidationSummary {
                    status: ValidationStatus::Fail,
                    rule_id: "evidence-schema".into(),
                    rule: "evidence file conforms to schemas/evidence.schema.json".into(),
                    detail: Some(format!("{}: {err}", path.display())),
                });
            }
        }
    }

    if summaries.is_empty() {
        Ok(docs)
    } else {
        Err(Error::Validation {
            code: "evidence-schema".into(),
            detail: join_details(&summaries),
        })
    }
}

/// Validate a single Evidence document (already read into `content`)
/// against the embedded `schemas/evidence.schema.json`.
///
/// This is the `extract` validate-before-visible gate: the runner
/// reads the agent- or tool-produced Evidence,
/// runs it through this check, and only persists it to
/// `.specify/slices/<slice>/evidence/<source>.yaml` on success — a
/// schema failure writes no Evidence file. `source_path` labels error
/// messages with the originating file so an operator can find the
/// offending document.
///
/// Validating the already-read `content` (rather than re-reading the
/// path) pins validation to the exact bytes the caller persists.
///
/// # Errors
///
/// Returns [`Error::Validation`] (`evidence-schema`, exit code 2) when
/// YAML parsing or schema validation fails.
pub fn validate_evidence(content: &str, source_path: &Path) -> Result<()> {
    validate_labelled_yaml(
        content,
        source_path,
        EVIDENCE_JSON_SCHEMA,
        "evidence-schema",
        "evidence file conforms to schemas/evidence.schema.json",
    )
}

/// Validate every lead in `leads` against the embedded
/// `schemas/discovery/lead.schema.json`.
///
/// This is the `survey` validate-before-visible gate: the
/// `survey` runner parses the agent- or tool-produced lead set, runs it
/// through this check, and only calls
/// [`crate::change`]-side [`artifacts::discovery::Discovery::merge_survey`]
/// on success — a schema failure leaves `discovery.md` untouched.
///
/// Findings across every lead are aggregated into a single
/// [`Error::Validation`] (exit code 2) keyed on `discovery-lead-schema`,
/// each labelled with the offending lead's `lead`.
///
/// # Errors
///
/// - [`Error::Diag`] (`discovery-lead-serialise`) when a lead is not
///   JSON-serialisable (unreachable for the closed `Lead` derive).
/// - [`Error::Validation`] (`discovery-lead-schema`) when any lead
///   fails the schema.
pub fn validate_leads(leads: &[Lead]) -> Result<()> {
    let rule = "lead conforms to schemas/discovery/lead.schema.json";
    let mut summaries: Vec<ValidationSummary> = Vec::new();
    for lead in leads {
        let instance = serde_json::to_value(lead).map_err(|err| Error::Diag {
            code: "discovery-lead-serialise",
            detail: format!(
                "failed to serialise lead `{}` for schema validation: {err}",
                lead.lead
            ),
        })?;
        for summary in
            validate_value_cached(&instance, LEAD_JSON_SCHEMA, "discovery-lead-schema", rule)
        {
            if summary.status == ValidationStatus::Fail {
                summaries.push(relabel(summary, format_args!("lead `{}`", lead.lead)));
            }
        }
    }

    if summaries.is_empty() {
        Ok(())
    } else {
        Err(Error::Validation {
            code: "discovery-lead-schema".into(),
            detail: join_details(&summaries),
        })
    }
}
