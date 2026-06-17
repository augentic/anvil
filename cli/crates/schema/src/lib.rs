//! Embedded JSON Schemas and the JSON-Schema validation plumbing
//! shared between `specify-workflow` (workflow artifacts) and the future
//! `specify-standards` standards-layer crate, per [DECISIONS.md § Standards layer split into `specify-standards` and `specify-schema`](../../DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema).
//!
//! Schemas are bundled at compile time via `include_str!` so the binary
//! carries them with no runtime filesystem lookup. The helpers in
//! [`validate`] convert `jsonschema` validator output into the
//! operational [`validate::ValidationSummary`] shape that callers fold
//! into a payload-free [`specify_error::Error::Validation`] (exit code
//! 2) or [`specify_error::Error::Diag`] (exit code 1) as their policy
//! dictates.

pub mod cache;
pub mod constants;
pub mod digest;
pub mod validate;

pub use constants::{
    ADAPTER_JSON_SCHEMA, BUILD_REPORT_JSON_SCHEMA, BUILD_REQUEST_JSON_SCHEMA,
    COMPONENTS_JSON_SCHEMA, CONTRACT_DUMP_JSON_SCHEMA, DECISION_JSON_SCHEMA,
    DIAGNOSTIC_JSON_SCHEMA, DIAGNOSTIC_REPORT_JSON_SCHEMA, EMBEDDED_SCHEMAS, EVIDENCE_JSON_SCHEMA,
    EXTENSION_JSON_SCHEMA, EXTENSION_SIDECAR_JSON_SCHEMA, FRAMEWORK_JSON_SCHEMA, LEAD_JSON_SCHEMA,
    MARKETPLACE_JSON_SCHEMA, PARTS_JSON_SCHEMA, PLAN_JSON_SCHEMA, PROPOSAL_JSON_SCHEMA,
    PROVENANCE_JSON_SCHEMA, RESOLVED_RULES_JSON_SCHEMA, RULE_JSON_SCHEMA, SCENARIO_JSON_SCHEMA,
    SKILL_JSON_SCHEMA, SLICE_MODEL_JSON_SCHEMA, SOURCE_JSON_SCHEMA, SYNTHESIS_JSON_SCHEMA,
    TARGET_JSON_SCHEMA, TOPOLOGY_LOCK_JSON_SCHEMA, WORKSPACE_MODEL_JSON_SCHEMA,
};
pub use validate::{
    ValidationStatus, ValidationSummary, cached_validator, compile_schema, join_details,
    read_yaml_as_json, validate_serialisable, validate_value, validate_value_cached,
};
