//! Target-axis gates: the build request and build report envelopes.

use std::sync::LazyLock;

use error::Result;
use schema::{
    BUILD_REPORT_JSON_SCHEMA, BUILD_REQUEST_JSON_SCHEMA, DIAGNOSTIC_JSON_SCHEMA, Validator,
    compile_ref_validator,
};

use super::support::{validate_parsed_json, validate_with_registry};

/// Validate a target build request against the embedded
/// `schemas/target/build-request.schema.json`.
///
/// Backs `specify slice build`: the request the CLI assembles
/// ([`crate::slice::build_request`]) and writes to
/// `.specify/slices/<slice>/build/request.yaml` is gated against this
/// shape before handoff. The request carries no `$ref`, so the simple
/// cached-validator path (as in [`super::validate_plan_yaml`])
/// suffices. Parsing through [`serde_saphyr::from_str`] accepts both
/// the YAML the CLI persists and a JSON instance. Failures use
/// `target-build-request-schema` on the validation exit path.
pub fn validate_request(content: &str) -> Result<()> {
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
    "https://github.com/augentic/specify/schemas/diagnostics/diagnostic.schema.json";

/// Validate a target build report against the embedded
/// `schemas/target/build-report.schema.json`.
///
/// Backs `specify slice build`: the report a target writes to
/// `.specify/slices/<slice>/build/report.yaml` is gated against this
/// shape before the `built` transition. Its `findings[]` `$ref`s
/// `diagnostic.schema.json` by a relative URI, so the validator is built
/// through a registry that pins [`DIAGNOSTIC_JSON_SCHEMA`] under its
/// `$id` (`DIAGNOSTIC_SCHEMA_URL`) — the same registry pattern the
/// synthesis validator uses for the relative `model` `$ref`. Failures
/// use `target-build-report-schema` on the validation exit path.
pub fn validate_report(content: &str) -> Result<()> {
    validate_with_registry(
        content,
        &BUILD_REPORT_VALIDATOR,
        "target-build-report-schema",
        "build report conforms to schemas/target/build-report.schema.json",
    )
}

/// Build-report validator with the diagnostic schema pinned so the
/// relative `findings[]` `$ref` resolves, compiled once on first use.
///
/// See the synthesis validator in [`super::judgment`] for the
/// `expect`-on-corrupt-binary rationale.
static BUILD_REPORT_VALIDATOR: LazyLock<Validator> = LazyLock::new(|| {
    compile_ref_validator(BUILD_REPORT_JSON_SCHEMA, DIAGNOSTIC_SCHEMA_URL, DIAGNOSTIC_JSON_SCHEMA)
        .expect("embedded build-report + diagnostic schemas compile (corrupt binary otherwise)")
});
