//! Schema-gated judgment calls, with optional bounded repair.

use omnia_guest::Model;
use omnia_guest::model::{Format, Message, Reply, Request, Role, SchemaFormat, Tool};
use serde::de::DeserializeOwned;

use crate::types::{Context, Error};

/// Maximum repairs after the initial answer.
pub const MAX_REPAIRS: usize = 2;

/// Runs one schema-gated judgment and deserializes its answer.
///
/// # Errors
///
/// Returns the mapped model error or [`Error::Internal`] on deserialization.
pub async fn judgment<P: Model, T: DeserializeOwned>(
    model: &P, ctx: &Context<'_>, system: String, user: String, schema_name: &str, schema: &str,
) -> Result<T, Error> {
    let reply = complete(model, ctx, &system, user, schema_name, schema).await?;
    serde_json::from_str(&reply.answer)
        .map_err(|err| Error::Internal(format!("{schema_name} answer did not deserialize: {err}")))
}

/// Runs a schema-gated judgment with bounded `tail` repair.
///
/// Only [`Error::Internal`] from `tail` is retried. Request and model
/// failures return immediately.
///
/// # Errors
///
/// Returns the mapped model error, a non-repairable tail error, or the
/// final tail error after exhausting repairs.
pub async fn repaired<P, T, F>(
    model: &P, ctx: &Context<'_>, system: String, user: String, schema_name: &str, schema: &str,
    mut tail: F,
) -> Result<T, Error>
where
    P: Model,
    F: FnMut(&str) -> Result<T, Error>,
{
    let mut prompt = user.clone();
    let mut attempt = 0;
    loop {
        let reply = complete(model, ctx, &system, prompt, schema_name, schema).await?;
        match tail(&reply.answer) {
            Ok(value) => return Ok(value),
            Err(err @ Error::Internal(_)) if attempt < MAX_REPAIRS => {
                attempt += 1;
                prompt = repair_prompt(&user, &reply.answer, &err);
            }
            Err(err) => return Err(err),
        }
    }
}

async fn complete<P: Model>(
    model: &P, ctx: &Context<'_>, system: &str, user: String, schema_name: &str, schema: &str,
) -> Result<Reply, Error> {
    let builder = Request::builder()
        .system(system)
        .messages(vec![Message {
            role: Role::User,
            content: user,
        }])
        .format(Format::Schema(SchemaFormat::builder().name(schema_name).schema(schema).build()))
        .tools(ctx.grants().into_iter().map(Tool::Mcp).collect());
    let request = match ctx.lend.clone() {
        Some(lend) => builder.workspace(lend).build(),
        None => builder.build(),
    };
    model.complete(request).await.map_err(Error::from)
}

fn repair_prompt(user: &str, failed_answer: &str, err: &Error) -> String {
    format!(
        "{user}\n\n## Previous answer (failed validation)\n\n{failed_answer}\n\n\
         ## Findings\n\n{err}\n\n\
         Produce a corrected, complete answer that resolves every finding."
    )
}
