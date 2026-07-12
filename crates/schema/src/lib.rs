//! Embedded JSON Schemas, the JSON-Schema validation plumbing, and the
//! neutral [`diagnostics`] substrate shared by the workspace crates
//! (`artifacts`, `workflow`, `cli`).
//!
//! Schemas are bundled at compile time via `include_str!` so the binary
//! carries them with no runtime filesystem lookup. The helpers in
//! [`validate`] convert `jsonschema` validator output into the
//! operational [`validate::ValidationSummary`] shape that callers fold
//! into a payload-free [`error::Error::Validation`] (exit code
//! 2) or [`error::Error::Diag`] (exit code 1) as their policy
//! dictates.

pub mod answers;
pub mod cache;
pub mod constants;
pub mod diagnostics;
pub mod digest;
pub mod validate;

pub use constants::{
    BUILD_REPORT_JSON_SCHEMA, BUILD_REQUEST_JSON_SCHEMA, COMPONENTS_JSON_SCHEMA,
    DECISION_JSON_SCHEMA, DIAGNOSTIC_JSON_SCHEMA, DIAGNOSTIC_REPORT_JSON_SCHEMA, EMBEDDED_SCHEMAS,
    EVIDENCE_JSON_SCHEMA, LEAD_JSON_SCHEMA, MARKETPLACE_JSON_SCHEMA, PARTS_JSON_SCHEMA,
    PLAN_JSON_SCHEMA, PROPOSAL_JSON_SCHEMA, PROVENANCE_JSON_SCHEMA, SCENARIO_JSON_SCHEMA,
    SKILL_JSON_SCHEMA, SLICE_MODEL_JSON_SCHEMA, SYNTHESIS_JSON_SCHEMA, TOPOLOGY_LOCK_JSON_SCHEMA,
};
pub use validate::{
    ValidationStatus, ValidationSummary, Validator, cached_validator, compile_ref_validator,
    compile_schema, join_details, read_yaml_as_json, validate_serialisable, validate_value,
    validate_value_cached, validation_errors,
};
