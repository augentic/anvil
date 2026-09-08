//! Model judgments
//!
//! The one way the engine asks the model a question: a judgment sends a
//! prompt, requires the answer to match the JSON schema of the type it is
//! asked for, and hands the parsed answer to a caller-supplied check. An
//! answer that parses but fails the check is sent back with the findings
//! attached, a bounded number of times, because models usually fix a
//! concrete finding on the next attempt.
//!
//! This is the shape the adapter SDK's `repaired` loop has; the engine
//! carries its own copy because no production crate may depend on the SDK.

use std::marker::PhantomData;

use omnia_guest::model::{Format, Message, Request, Role, SchemaFormat};
use omnia_guest::{Error, Model, bad_gateway, bad_request};
use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde::de::DeserializeOwned;

/// Maximum repairs after the initial answer.
pub const MAX_REPAIRS: usize = 2;

/// Everything a check rejects in one answer, one line each.
pub type Findings = Vec<String>;

/// A question whose answer is one `T`: the system prose, the schema name
/// passed to the provider, and `T`'s schema.
#[derive(Debug)]
pub struct Question<T> {
    system: String,
    name: &'static str,
    schema: String,
    answer: PhantomData<fn() -> T>,
}

impl<T: DeserializeOwned + JsonSchema> Question<T> {
    /// Builds the question `name`, asking for a `T` under the synthesis
    /// prose at `paths`.
    ///
    /// # Panics
    ///
    /// Panics only if `T`'s generated schema does not serialise.
    pub fn new(name: &'static str, paths: &[&str]) -> Self {
        let generated = SchemaSettings::draft2020_12().into_generator().into_root_schema_for::<T>();
        let schema = serde_json::to_string(&generated).expect("generated answer schema serialises");

        Self {
            system: system(paths),
            name,
            schema,
            answer: PhantomData,
        }
    }

    /// Runs the judgment over `user`, retrying while `check` reports
    /// findings, up to [`MAX_REPAIRS`] times.
    ///
    /// Only findings are retried; a model failure returns at once.
    ///
    /// # Errors
    ///
    /// Returns `BadGateway` for a model failure and `BadRequest` naming the
    /// final findings once the repairs are exhausted.
    pub async fn ask<M, F>(&self, model: &M, user: &str, mut check: F) -> Result<T, Error>
    where
        M: Model,
        F: FnMut(&T) -> Result<(), Findings>,
    {
        let mut prompt = user.to_string();
        let mut attempt = 0;
        loop {
            let answer = self.complete(model, prompt).await?;
            // A parse failure is one finding, so a malformed answer is
            // repaired like any other miss.
            let judged = serde_json::from_str::<T>(&answer)
                .map_err(|err| vec![format!("answer did not deserialize: {err}")])
                .and_then(|value| check(&value).map(|()| value));
            let findings = match judged {
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

// Joins the synthesis prose at `paths` into one system prompt.
fn system(paths: &[&str]) -> String {
    paths.iter().map(|path| crate::prose::body(path)).collect::<Vec<_>>().join("\n\n---\n\n")
}
