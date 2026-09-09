//! Evidence answers
//!
//! The one model call an adapter makes: [`evidence`] asks the extract
//! question against the [`Evidence`] schema and turns the answer into a
//! valid document, repairing it in-adapter when the claim gate finds fault.
//! [`content_note`] is the prompt fragment that tells the model what it was
//! bound to.
//!
//! The schema constrains the answer's shape but cannot express every rule a
//! claim must satisfy, so the answer is validated again in code. Running that
//! check inside the adapter, where a failed answer can be sent back for
//! repair, means the engine rarely sees evidence it has to reject.

use emery_source::claims::DOTTED_KEBAB_PATTERN;
use omnia_guest::Model;
use schemars::generate::SchemaSettings;
use serde_json::{Value, json};

use crate::types::{Context, Error, Evidence, SourceContent, SourceInput};

/// Asks the extract question and returns the accepted [`Evidence`].
///
/// # Errors
///
/// Returns the mapped model error, or [`Error::Internal`] with the last
/// gate findings once the repair budget is spent.
pub async fn evidence<P: Model>(
    model: &P, ctx: &Context<'_>, system: String, user: String,
) -> Result<Evidence, Error> {
    let schema = evidence_schema();
    crate::repaired(model, ctx, system, user, "evidence", &schema, tail).await
}

/// Describes the bound source to the model; `tree` names what a workspace
/// holds (for example `the documentation tree`).
#[must_use]
pub fn content_note(input: &SourceInput, tree: &str) -> String {
    match &input.content {
        SourceContent::Workspace(root) => format!(
            "`$SOURCE_DIR` is the read-only view at `{root}` — {tree} the prompt walks. \
             Nothing outside it is reachable; extract mines only this source."
        ),
        SourceContent::Value(value) => format!(
            "The bound material is this inline value; no `$SOURCE_DIR` is lent:\n\n{value}\n\n\
             Nothing else is reachable; extract mines only this source."
        ),
    }
}

/// Schema for `extract` answers.
///
/// # Panics
///
/// Panics only if `schemars` produces a non-object or drops the
/// compile-owned `Claim` definition patched below.
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

// Parse, then run the claim gate; both failures are repairable.
fn tail(answer: &str) -> Result<Evidence, Error> {
    let evidence: Evidence = serde_json::from_str(answer)
        .map_err(|err| Error::Internal(format!("evidence answer did not deserialize: {err}")))?;
    evidence.validate()?;
    Ok(evidence)
}
