//! Evidence answers
//!
//! The schema an `extract` judgment asks the model to answer against, and the
//! check that turns the model's answer into a valid [`Evidence`] document.
//!
//! The schema constrains the answer's shape but cannot express every rule a
//! claim must satisfy, so the answer is validated again in code. Running that
//! check inside the adapter, where a failed answer can be sent back for
//! repair, means the engine rarely sees evidence it has to reject.

use emery_source::claims::DOTTED_KEBAB_PATTERN;
use schemars::generate::SchemaSettings;
use serde_json::{Value, json};

use crate::types::{Error, Evidence};

/// Schema for `extract` answers.
///
/// # Panics
///
/// Panics only if `schemars` produces a non-object or drops the
/// compile-owned `Evidence` properties patched below.
#[must_use]
pub fn evidence_schema() -> String {
    let schema = SchemaSettings::draft2020_12().into_generator().into_root_schema_for::<Evidence>();
    let mut value = schema.to_value();
    let root = value.as_object_mut().expect("generated answer schema is an object");
    root.insert("title".to_string(), json!("Emery extract answer"));
    root.insert(
        "description".to_string(),
        json!(
            "Schema-gated source extract answer, generated from the Evidence DTO that \
             deserialises the model response."
        ),
    );

    let id = value
        .pointer_mut("/$defs/Claim/properties/id")
        .and_then(Value::as_object_mut)
        .expect("evidence schema carries Claim.id");
    id.insert("pattern".to_string(), json!(DOTTED_KEBAB_PATTERN));

    let claim = value
        .pointer_mut("/$defs/Claim")
        .and_then(Value::as_object_mut)
        .expect("evidence schema carries Claim");
    claim.insert(
        "if".to_string(),
        json!({
            "properties": {
                "kind": {"enum": ["requirement", "criterion", "example"]}
            }
        }),
    );
    claim.insert(
        "then".to_string(),
        json!({
            "properties": {
                "id": {"pattern": DOTTED_KEBAB_PATTERN, "type": "string"}
            },
            "required": ["id"]
        }),
    );

    serde_json::to_string(&value).expect("generated answer schema serialises")
}

/// Parses and validates an evidence answer for [`crate::repaired`].
///
/// # Errors
///
/// Returns [`Error::Internal`] on parse or validation failure.
pub fn evidence_tail(answer: &str) -> Result<Evidence, Error> {
    let evidence: Evidence = serde_json::from_str(answer)
        .map_err(|err| Error::Internal(format!("evidence answer did not deserialize: {err}")))?;
    evidence.validate()?;
    Ok(evidence)
}
