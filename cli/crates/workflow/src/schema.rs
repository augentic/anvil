//! Domain-shaped JSON Schema validation hooks for source/target adapter split on-disk
//! artifacts.
//!
//! The raw JSON-Schema plumbing and embedded constants live in
//! [`specify_schema`] per [DECISIONS.md § Standards layer split into `specify-standards` and `specify-schema`](../../DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema); this module holds
//! the workflow-aware wrappers — they import [`crate::change::Plan`],
//! aggregate per-file findings into a single
//! [`specify_error::Error::Validation`] payload, and pin the wire
//! `rule_id` strings the CLI surfaces.
//!
//! Schemas are embedded by [`specify_schema::constants`] via
//! `include_str!`. The validators return [`Error::Validation`] on a
//! schema mismatch so the CLI exits with code 2
//! (`Exit::ValidationFailed` in the binary crate).

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use jsonschema::{Registry, Resource};
use serde_json::Value as JsonValue;
use specify_error::{Error, Result};
use specify_model::discovery::Lead;
pub(crate) use specify_schema::{
    BUILD_REPORT_JSON_SCHEMA, BUILD_REQUEST_JSON_SCHEMA, COMPONENTS_JSON_SCHEMA,
    DIAGNOSTIC_JSON_SCHEMA, EVIDENCE_JSON_SCHEMA, LEAD_JSON_SCHEMA, PARTS_JSON_SCHEMA,
    PLAN_JSON_SCHEMA, PROPOSAL_JSON_SCHEMA, PROVENANCE_JSON_SCHEMA, SLICE_MODEL_JSON_SCHEMA,
    SYNTHESIS_JSON_SCHEMA, TOPOLOGY_LOCK_JSON_SCHEMA, read_yaml_as_json, validate_serialisable,
    validate_value, validate_value_cached,
};
use specify_schema::{ValidationStatus, ValidationSummary, join_details};

use crate::change::Plan;

/// Validate `plan` against the embedded `schemas/plan/plan.schema.json`.
///
/// Returns `Ok(())` on a clean validation; otherwise a payload-free
/// [`Error::Validation`] keyed on the code `"plan-schema"`, with the
/// JSON-pointer + reason list the schema produced joined into the
/// detail. Used by `specify plan add` and `specify plan amend` so
/// first-use validation refuses to write a malformed plan.
///
/// # Errors
///
/// Returns [`Error::Validation`] when the in-memory plan fails the
/// schema; falls back to [`Error::Diag`] when the embedded schema is
/// unparseable or the plan is not JSON-serialisable (both should be
/// unreachable in production — they exist to surface a corrupted
/// binary).
pub fn validate_plan(plan: &Plan) -> Result<()> {
    validate_serialisable(
        plan,
        PLAN_JSON_SCHEMA,
        "plan-schema",
        "plan.yaml conforms to schemas/plan/plan.schema.json",
        "plan-schema-serialise",
        "plan",
    )
}

/// Validate raw `plan.yaml` content before typed deserialisation.
///
/// # Errors
///
/// Returns [`Error::Validation`] when YAML parsing or schema validation fails.
pub fn validate_plan_yaml(content: &str) -> Result<()> {
    validate_parsed_json(
        content,
        PLAN_JSON_SCHEMA,
        "plan-schema",
        "plan.yaml conforms to schemas/plan/plan.schema.json",
    )
}

/// Validate a lead-reconciliation envelope against the embedded
/// `schemas/discovery/proposal.schema.json`.
///
/// Backs `specify plan propose`: the dry-run request the
/// CLI emits and the agent grouping response read by `--from` share one
/// schema, discriminated by the closed `kind: request | response`
/// `oneOf`. A single call validates either kind — there is no separate
/// request/response entry point.
///
/// Both envelopes arrive as JSON (the request on stdout, the response
/// from stdin or a `--from <file>` path), so parsing through
/// [`serde_saphyr::from_str`] — which accepts JSON as a YAML subset —
/// mirrors [`validate_plan_yaml`] and lets hand-authored YAML responses
/// validate too. On a clean parse the value is checked against
/// [`PROPOSAL_JSON_SCHEMA`] and any failures are folded into one
/// payload-free [`Error::Validation`].
///
/// # Errors
///
/// Returns [`Error::Validation`] keyed on the code `"proposal-schema"`
/// (exit code 2) when parsing or schema validation fails.
pub fn validate_proposal_json(content: &str) -> Result<()> {
    validate_parsed_json(
        content,
        PROPOSAL_JSON_SCHEMA,
        "proposal-schema",
        "proposal envelope conforms to schemas/discovery/proposal.schema.json",
    )
}

/// `$id` the synthesis schema's relative `model` `$ref` resolves to.
const MODEL_SCHEMA_URL: &str =
    "https://github.com/augentic/specify-cli/schemas/slice/model.schema.json";

/// Validate an agent synthesis response against the embedded
/// `schemas/slice/synthesis.schema.json`.
///
/// Backs `specify slice synthesize`: synthesis is
/// always agent-dispatched, so the only schema-validated wire is the
/// returned `kind: response`. Its `model` property `$ref`s
/// `model.schema.json` by a relative URI, so the validator is built
/// through a [`Registry`] that pins [`SLICE_MODEL_JSON_SCHEMA`] under
/// its `$id` (`MODEL_SCHEMA_URL`) — the same registry pattern the
/// diagnostic-report renderer uses to resolve its relative finding
/// `$ref`.
///
/// The response arrives as JSON (a YAML subset), so parsing through
/// [`serde_saphyr::from_str`] mirrors [`validate_proposal_json`] and
/// lets hand-authored YAML responses validate too.
///
/// # Errors
///
/// Returns [`Error::Validation`] keyed on the code `"synthesis-schema"`
/// (exit code 2) when parsing or schema validation fails.
pub fn validate_synthesis_json(content: &str) -> Result<()> {
    validate_with_ref_validator(
        content,
        &SYNTHESIS_VALIDATOR,
        "synthesis-schema",
        "synthesis response conforms to schemas/slice/synthesis.schema.json",
    )
}

/// Synthesis validator with the model schema pinned so the relative
/// `model` `$ref` resolves, compiled once on first use.
///
/// A compile failure here means an embedded schema is corrupt (a broken
/// binary), so the `expect` is genuinely unreachable in production and
/// mirrors the `LazyLock<Regex>` pattern used elsewhere for static
/// schema/regex compilation.
static SYNTHESIS_VALIDATOR: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    compile_ref_validator(SYNTHESIS_JSON_SCHEMA, MODEL_SCHEMA_URL, SLICE_MODEL_JSON_SCHEMA)
        .expect("embedded synthesis + model schemas compile (corrupt binary otherwise)")
});

/// Validate a target build request against the embedded
/// `schemas/target/build-request.schema.json`.
///
/// Backs `specify slice build`: the request the CLI assembles
/// ([`crate::slice::build_request`]) and writes to
/// `.specify/slices/<slice>/build/request.yaml` is gated against this
/// shape before handoff. The request carries no `$ref`, so the simple
/// [`validate_value`] path (as in [`validate_plan_yaml`]) suffices.
/// Parsing through [`serde_saphyr::from_str`] accepts both the YAML the
/// CLI persists and a JSON instance.
///
/// # Errors
///
/// Returns [`Error::Validation`] keyed on `target-build-request-schema`
/// (exit code 2) when parsing or schema validation fails.
pub fn validate_build_request_json(content: &str) -> Result<()> {
    validate_parsed_json(
        content,
        BUILD_REQUEST_JSON_SCHEMA,
        "target-build-request-schema",
        "build request conforms to schemas/target/build-request.schema.json",
    )
}

/// `$id` the build-report schema's relative `findings[]` `$ref` resolves
/// to.
const DIAGNOSTIC_SCHEMA_URL: &str =
    "https://github.com/augentic/specify-cli/schemas/diagnostics/diagnostic.schema.json";

/// Validate a target build report against the embedded
/// `schemas/target/build-report.schema.json`.
///
/// Backs `specify slice build`: the report a target writes to
/// `.specify/slices/<slice>/build/report.yaml` is gated against this
/// shape before the `built` transition. Its `findings[]` `$ref`s
/// `diagnostic.schema.json` by a relative URI, so the validator is built
/// through a [`Registry`] that pins [`DIAGNOSTIC_JSON_SCHEMA`] under its
/// `$id` (`DIAGNOSTIC_SCHEMA_URL`) — the same registry pattern
/// `compile_synthesis_validator` uses for the relative `model` `$ref`.
///
/// # Errors
///
/// Returns [`Error::Validation`] keyed on `target-build-report-schema`
/// (exit code 2) when parsing or schema validation fails.
pub fn validate_build_report_json(content: &str) -> Result<()> {
    validate_with_ref_validator(
        content,
        &BUILD_REPORT_VALIDATOR,
        "target-build-report-schema",
        "build report conforms to schemas/target/build-report.schema.json",
    )
}

/// Build-report validator with the diagnostic schema pinned so the
/// relative `findings[]` `$ref` resolves, compiled once on first use.
///
/// See [`SYNTHESIS_VALIDATOR`] for the `expect`-on-corrupt-binary
/// rationale.
static BUILD_REPORT_VALIDATOR: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    compile_ref_validator(BUILD_REPORT_JSON_SCHEMA, DIAGNOSTIC_SCHEMA_URL, DIAGNOSTIC_JSON_SCHEMA)
        .expect("embedded build-report + diagnostic schemas compile (corrupt binary otherwise)")
});

/// Validate a [`crate::registry::TopologyLock`] against the embedded
/// `schemas/topology-lock.schema.json`.
///
/// Returns `Ok(())` on a clean validation; otherwise a payload-free
/// [`Error::Validation`] keyed on `"topology-lock-schema"`. Used by the
/// `topology.lock` reader/writer so a corrupt cache fails closed.
///
/// # Errors
///
/// Returns [`Error::Validation`] when the lock fails the schema; falls
/// back to [`Error::Diag`] when the embedded schema is unparseable or
/// the lock is not JSON-serialisable (both unreachable in production).
pub fn validate_topology_lock(lock: &crate::registry::TopologyLock) -> Result<()> {
    validate_serialisable(
        lock,
        TOPOLOGY_LOCK_JSON_SCHEMA,
        "topology-lock-schema",
        ".specify/topology.lock conforms to schemas/topology-lock.schema.json",
        "topology-lock-schema-serialise",
        "topology.lock",
    )
}

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

    let entries = fs::read_dir(&evidence_dir).map_err(|source| Error::Filesystem {
        op: "readdir",
        path: evidence_dir.clone(),
        source,
    })?;

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Filesystem {
            op: "readdir-entry",
            path: evidence_dir.clone(),
            source,
        })?;
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

/// Validate raw `components.yaml` content against the embedded
/// `schemas/design-system/components.schema.json`.
///
/// `source_path` labels error messages with the originating file.
///
/// # Errors
///
/// Returns [`Error::Validation`] when YAML parsing or schema validation fails.
pub fn validate_components_yaml(content: &str, source_path: &Path) -> Result<()> {
    validate_labelled_yaml(
        content,
        source_path,
        COMPONENTS_JSON_SCHEMA,
        "catalog-schema",
        "components.yaml conforms to schemas/design-system/components.schema.json",
    )
}

/// Validate raw `parts.yaml` content against the embedded
/// `schemas/design-system/parts.schema.json`.
///
/// `source_path` labels error messages with the originating file.
/// Backs [`crate::design_system::Parts::load`] — the schema is the only
/// gate on the operator-authored parts input ("beyond
/// schema conformance there are no coherence gates").
///
/// # Errors
///
/// Returns [`Error::Validation`] when YAML parsing or schema validation fails.
pub fn validate_parts_yaml(content: &str, source_path: &Path) -> Result<()> {
    validate_labelled_yaml(
        content,
        source_path,
        PARTS_JSON_SCHEMA,
        "parts-schema",
        "parts.yaml conforms to schemas/design-system/parts.schema.json",
    )
}

/// Validate a single Evidence document (already read into `content`)
/// against the embedded `schemas/evidence.schema.json`.
///
/// This is the `extract` validate-before-visible gate (
/// DECISIONS.md §"Source operations"): the runner reads the agent-
/// or tool-produced Evidence,
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
/// This is the `survey` validate-before-visible gate (
/// DECISIONS.md §"Source operations"): the
/// `survey` runner parses the agent- or tool-produced lead set, runs it
/// through this check, and only calls
/// [`crate::change`]-side [`specify_model::discovery::Discovery::merge_survey`]
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

fn relabel(mut summary: ValidationSummary, label: impl std::fmt::Display) -> ValidationSummary {
    let detail = summary.detail.take().unwrap_or_default();
    summary.detail =
        Some(if detail.is_empty() { label.to_string() } else { format!("{label}: {detail}") });
    summary
}

/// Parse `content` (JSON or its YAML superset) and validate it against a
/// `$ref`-free embedded `schema`, folding every schema failure into one
/// payload-free [`Error::Validation`] keyed on `code`.
///
/// This is the shared kernel behind the simple `validate_*_json`
/// entry points whose schema carries no relative `$ref`.
///
/// # Errors
///
/// Returns [`Error::Validation`] (keyed on `code`) when parsing or
/// schema validation fails.
fn validate_parsed_json(
    content: &str, schema: &'static str, code: &'static str, rule: &str,
) -> Result<()> {
    let instance: JsonValue = serde_saphyr::from_str(content)
        .map_err(|err| Error::validation_failed(code, rule, format!("parse failed: {err}")))?;
    err_from_failures(code, &validation_failures(&instance, schema, code, rule))
}

/// Parse YAML `content` and validate it against a `$ref`-free embedded
/// `schema`, labelling the parse error and every schema failure with
/// `source_path` so an operator can find the offending file, then
/// folding the failures into one [`Error::Validation`] keyed on `code`.
///
/// This is the shared kernel behind the file-anchored `validate_*_yaml`
/// / `validate_evidence` entry points.
///
/// # Errors
///
/// Returns [`Error::Validation`] (keyed on `code`) when parsing or
/// schema validation fails.
fn validate_labelled_yaml(
    content: &str, source_path: &Path, schema: &'static str, code: &'static str, rule: &str,
) -> Result<()> {
    let instance: JsonValue = serde_saphyr::from_str(content).map_err(|err| {
        Error::validation_failed(
            code,
            rule,
            format!("{}: YAML parse failed: {err}", source_path.display()),
        )
    })?;
    let failures: Vec<ValidationSummary> = validation_failures(&instance, schema, code, rule)
        .into_iter()
        .map(|summary| relabel(summary, source_path.display()))
        .collect();
    err_from_failures(code, &failures)
}

/// Parse `content` and validate it against a pre-compiled, registry-backed
/// `validator` (one whose schema carries a relative `$ref`), folding every
/// schema failure into one [`Error::Validation`] keyed on `code`.
///
/// # Errors
///
/// Returns [`Error::Validation`] (keyed on `code`) when parsing or
/// schema validation fails.
fn validate_with_ref_validator(
    content: &str, validator: &jsonschema::Validator, code: &'static str, rule: &str,
) -> Result<()> {
    let instance: JsonValue = serde_saphyr::from_str(content)
        .map_err(|err| Error::validation_failed(code, rule, format!("parse failed: {err}")))?;
    let failures: Vec<String> =
        validator.iter_errors(&instance).map(|err| err.to_string()).collect();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::Validation {
            code: code.into(),
            detail: failures.join("; "),
        })
    }
}

/// Compile an embedded `schema` whose relative `$ref` is satisfied by
/// pinning `ref_schema` under `ref_url` in a [`Registry`].
///
/// Returns the joined failure string on any parse/registry/compile
/// error; callers wrap it in a `LazyLock` and `expect` (a failure means
/// a corrupt binary).
fn compile_ref_validator(
    schema: &str, ref_url: &str, ref_schema: &str,
) -> std::result::Result<jsonschema::Validator, String> {
    let schema_value: JsonValue =
        serde_json::from_str(schema).map_err(|err| format!("schema parse failed: {err}"))?;
    let ref_value: JsonValue = serde_json::from_str(ref_schema)
        .map_err(|err| format!("ref schema parse failed: {err}"))?;
    let registry = Registry::new()
        .add(ref_url, Resource::from_contents(ref_value))
        .and_then(jsonschema::RegistryBuilder::prepare)
        .map_err(|err| format!("registry build failed: {err}"))?;
    jsonschema::options()
        .with_registry(&registry)
        .build(&schema_value)
        .map_err(|err| format!("schema compile failed: {err}"))
}

fn validation_failures(
    instance: &JsonValue, schema_source: &'static str, rule_id: &str, rule: &str,
) -> Vec<ValidationSummary> {
    validate_value_cached(instance, schema_source, rule_id, rule)
        .into_iter()
        .filter(|summary| summary.status == ValidationStatus::Fail)
        .collect()
}

fn err_from_failures(code: &'static str, results: &[ValidationSummary]) -> Result<()> {
    if results.is_empty() {
        Ok(())
    } else {
        Err(Error::Validation {
            code: code.into(),
            detail: join_details(results),
        })
    }
}

#[cfg(test)]
mod tests;
