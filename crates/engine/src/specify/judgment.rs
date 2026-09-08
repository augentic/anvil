//! Model judgments
//!
//! The one way the engine asks the model a question: a judgment sends a
//! prompt, requires the answer to match a JSON schema, and hands the answer
//! to a caller-supplied check. An answer that parses but fails the check is
//! sent back with the findings attached, a bounded number of times, because
//! models usually fix a concrete finding on the next attempt.
//!
//! This is the shape the adapter SDK's `repaired` loop has; the engine
//! carries its own copy because no production crate may depend on the SDK.

use omnia_guest::model::{Format, Message, Request, Role, SchemaFormat};
use omnia_guest::{Error, Model, bad_gateway, bad_request};
use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Maximum repairs after the initial answer.
pub const MAX_REPAIRS: usize = 2;

/// Everything a check rejects in one answer, one line each.
pub type Findings = Vec<String>;

/// A schema-gated question: the system prose, the answer's schema name, and
/// the schema itself.
#[derive(Debug)]
pub struct Question {
    /// The system prompt.
    pub system: String,
    /// The schema name passed to the provider.
    pub name: &'static str,
    /// The JSON Schema the answer must conform to.
    pub schema: String,
}

impl Question {
    /// Runs the judgment over `user` with bounded `tail` repair.
    ///
    /// Only findings are retried; a model failure returns at once.
    ///
    /// # Errors
    ///
    /// Returns `BadGateway` for a model failure and `BadRequest` naming the
    /// final findings once the repairs are exhausted.
    pub async fn ask<M, T, F>(&self, model: &M, user: &str, mut tail: F) -> Result<T, Error>
    where
        M: Model,
        F: FnMut(&str) -> Result<T, Findings>,
    {
        let mut prompt = user.to_string();
        let mut attempt = 0;
        loop {
            let answer = self.complete(model, prompt).await?;
            let findings = match tail(&answer) {
                Ok(value) => return Ok(value),
                Err(findings) => findings.join("\n"),
            };
            if attempt == MAX_REPAIRS {
                let name = self.name;
                return Err(bad_request!(
                    "model `{name}` answer failed validation after {MAX_REPAIRS} repairs:\n{findings}"
                ));
            }
            attempt += 1;
            prompt = format!(
                "{user}\n\n## Previous answer (failed validation)\n\n{answer}\n\n\
                 ## Findings\n\n{findings}\n\n\
                 Produce a corrected, complete answer that resolves every finding."
            );
        }
    }

    async fn complete<M: Model>(&self, model: &M, user: String) -> Result<String, Error> {
        let format = SchemaFormat::builder().name(self.name).schema(self.schema.as_str()).build();
        let request = Request::builder()
            .system(self.system.as_str())
            .messages(vec![Message {
                role: Role::User,
                content: user,
            }])
            .format(Format::Schema(format))
            .build();
        let reply = Model::complete(model, request).await.map_err(|err| bad_gateway!(err))?;
        Ok(reply.answer)
    }
}

/// The draft-2020-12 schema of `T`, after `patch` has adjusted the generated
/// document (closed objects, string patterns) where the derive cannot.
///
/// # Panics
///
/// Panics only if `schemars` produces a non-object schema.
pub fn schema<T: JsonSchema>(title: &str, patch: impl FnOnce(&mut Value)) -> String {
    let generated = SchemaSettings::draft2020_12().into_generator().into_root_schema_for::<T>();
    let mut value = generated.to_value();
    let root = value.as_object_mut().expect("generated answer schema is an object");
    root.insert("title".to_string(), Value::String(title.to_string()));
    patch(&mut value);
    serde_json::to_string(&value).expect("generated answer schema serialises")
}

/// Deserialises `answer`, reporting a parse failure as one finding.
pub fn parse<T: DeserializeOwned>(answer: &str) -> Result<T, Findings> {
    serde_json::from_str(answer).map_err(|err| vec![format!("answer did not deserialize: {err}")])
}

/// Joins the synthesis prose at `paths` into one system prompt.
pub fn system(paths: &[&str]) -> String {
    paths.iter().map(|path| crate::prose::body(path)).collect::<Vec<_>>().join("\n\n---\n\n")
}
