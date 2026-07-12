//! Judgment-leg gates: the reconciliation proposal envelope and the
//! synthesis response.

use std::sync::LazyLock;

use error::Result;
use schema::{
    PROPOSAL_JSON_SCHEMA, SLICE_MODEL_JSON_SCHEMA, SYNTHESIS_JSON_SCHEMA, Validator,
    compile_ref_validator,
};

use super::support::{validate_parsed_json, validate_with_registry};

/// Validate a lead-reconciliation envelope against the embedded
/// `schemas/discovery/proposal.schema.json`.
///
/// Backs the guest `plan author` reconciliation: the assembled request
/// and the agent grouping response share one
/// schema, discriminated by the closed `kind: request | response`
/// `oneOf`. A single call validates either kind — there is no separate
/// request/response entry point.
///
/// Both envelopes ride the judgment call as JSON, so parsing through
/// [`serde_saphyr::from_str`] — which accepts JSON as a YAML subset —
/// mirrors [`super::validate_plan_yaml`] and lets hand-authored YAML
/// responses validate too. On a clean parse the value is checked
/// against [`PROPOSAL_JSON_SCHEMA`] and any failures are folded into
/// one payload-free [`error::Error::Validation`].
///
/// # Errors
///
/// Returns [`error::Error::Validation`] keyed on the code
/// `"proposal-schema"` (exit code 2) when parsing or schema validation
/// fails.
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
    "https://github.com/augentic/specify/schemas/slice/model.schema.json";

/// Validate an agent synthesis response against the embedded
/// `schemas/slice/synthesis.schema.json`.
///
/// Backs the guest refine orchestration's synthesis leg: synthesis is
/// always agent-dispatched, so the only schema-validated wire is the
/// returned `kind: response`. Its `model` property `$ref`s
/// `model.schema.json` by a relative URI, so the validator is built
/// through a registry that pins [`SLICE_MODEL_JSON_SCHEMA`] under
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
/// Returns [`error::Error::Validation`] keyed on the code
/// `"synthesis-schema"` (exit code 2) when parsing or schema
/// validation fails.
pub fn validate_synthesis_json(content: &str) -> Result<()> {
    validate_with_registry(
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
static SYNTHESIS_VALIDATOR: LazyLock<Validator> = LazyLock::new(|| {
    compile_ref_validator(SYNTHESIS_JSON_SCHEMA, MODEL_SCHEMA_URL, SLICE_MODEL_JSON_SCHEMA)
        .expect("embedded synthesis + model schemas compile (corrupt binary otherwise)")
});
