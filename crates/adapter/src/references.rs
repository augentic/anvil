//! Reference tools
//!
//! The `list_docs` and `read_doc` tools a judgment offers the model, so it
//! can consult the adapter's embedded reference documents on demand instead
//! of receiving the whole corpus in the prompt.
//!
//! Tool calls are answered in-process from the embedded [`Doc`] table. There
//! is no server behind them, so an adapter needs no network access and no
//! external endpoint to expose its references.

use emery_prose::registry::{self, Doc};
use omnia_guest::model::{Function, Tool, ToolCall};
use serde_json::{Value, json};

/// The reference tools declared for a docs-carrying judgment.
#[must_use]
pub fn tools() -> Vec<Tool> {
    vec![
        Tool::Function(
            Function::builder()
                .name("list_docs")
                .description("List every reference document path this adapter embeds.")
                .parameters(json!({ "type": "object", "properties": {} }).to_string())
                .build(),
        ),
        Tool::Function(
            Function::builder()
                .name("read_doc")
                .description("Read one embedded reference document in full by its path.")
                .parameters(
                    json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Adapter-relative document path, \
                                                e.g. `prompts/build.md`."
                            }
                        },
                        "required": ["path"]
                    })
                    .to_string(),
                )
                .build(),
        ),
    ]
}

/// Answers one reference tool call over the embedded `docs`.
///
/// # Errors
///
/// Returns a repairable message for an unknown tool, malformed
/// arguments, or an unembedded path.
pub fn answer(docs: &[Doc], call: &ToolCall) -> Result<String, String> {
    match call.name.as_str() {
        "list_docs" => {
            let paths: Vec<&str> = docs.iter().map(|doc| doc.path).collect();
            Ok(json!({ "paths": paths }).to_string())
        }
        "read_doc" => {
            let arguments: Value = serde_json::from_str(&call.arguments)
                .map_err(|err| format!("read_doc: invalid arguments: {err}"))?;
            let path = arguments
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| "read_doc requires a string `path` argument".to_string())?;
            registry::resolve(docs, path).map_or_else(
                || Err(format!("no document `{path}`")),
                |body| Ok(json!({ "path": path, "body": body }).to_string()),
            )
        }
        other => Err(format!("unknown tool `{other}`")),
    }
}
