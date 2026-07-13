//! Plan-shape gates: `plan.yaml` in memory and on the wire.

use error::Result;
use schema::{PLAN_JSON_SCHEMA, validate_serialisable};

use super::support::validate_parsed_json;
use crate::plan::Plan;

/// Validate `plan` against the embedded `schemas/plan/plan.schema.json`.
///
/// Returns `Ok(())` on a clean validation; otherwise a payload-free
/// [`error::Error::Validation`] keyed on the code `"plan-schema"`, with
/// the JSON-pointer + reason list the schema produced joined into the
/// detail. Used by `specify plan add` and `specify plan amend` so
/// first-use validation refuses to write a malformed plan.
///
/// # Errors
///
/// Schema violations return [`error::Error::Validation`]. An invalid
/// embedded schema or unserialisable plan reports binary corruption
/// through [`error::Error::Diag`].
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

/// Validate raw `plan.yaml` content before typed deserialisation,
/// returning [`error::Error::Validation`] on malformed input.
pub fn validate_plan_yaml(content: &str) -> Result<()> {
    validate_parsed_json(
        content,
        PLAN_JSON_SCHEMA,
        "plan-schema",
        "plan.yaml conforms to schemas/plan/plan.schema.json",
    )
}
