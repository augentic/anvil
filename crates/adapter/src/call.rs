//! Shared judgment-call shape every adapter operation uses.
//!
//! Source legs use [`repaired`] (deterministic answer tail inside a
//! bounded repair loop). Target legs use one-shot [`judgment`] — build
//! and merge mutate the workspace, so they are never replayed.

use omnia_guest::Model;
use omnia_guest::model::{Format, Message, Reply, Request, Role, SchemaFormat, Tool};
use serde::de::DeserializeOwned;

use crate::seam::{Context, Error};

/// Maximum repair attempts after the first answer.
pub const MAX_REPAIRS: usize = 2;

/// One schema-gated judgment leg; deserializes the host-validated answer.
///
/// # Errors
///
/// [`Error::InvalidRequest`] when the model rejects the request;
/// [`Error::Internal`] for other model failures or a deserialize miss.
pub async fn judgment<P: Model, T: DeserializeOwned>(
    model: &P, ctx: &Context<'_>, system: String, user: String, schema_name: &str, schema: &str,
) -> Result<T, Error> {
    let reply = create(model, ctx, &system, user, schema_name, schema).await?;
    serde_json::from_str(&reply.answer)
        .map_err(|err| Error::Internal(format!("{schema_name} answer did not deserialize: {err}")))
}

/// Schema-gated judgment leg with a bounded repair loop over `tail`.
///
/// Only answer-tail failures ([`Error::Internal`] from `tail`) are
/// retried; model / invalid-request / I/O failures return immediately.
///
/// # Errors
///
/// The mapped model failure, a non-repairable tail failure, or the last
/// tail failure once the repair budget is exhausted.
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
        let reply = create(model, ctx, &system, prompt, schema_name, schema).await?;
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

async fn create<P: Model>(
    model: &P, ctx: &Context<'_>, system: &str, user: String, schema_name: &str, schema: &str,
) -> Result<Reply, Error> {
    model
        .create(
            Request::builder()
                .system(system)
                .messages(vec![Message {
                    role: Role::User,
                    content: user,
                }])
                .format(Format::Schema(
                    SchemaFormat::builder().name(schema_name).schema(schema).build(),
                ))
                .tools(ctx.grants().into_iter().map(Tool::Mcp).collect())
                .lend_workspace(true)
                .build(),
        )
        .await
        .map_err(Error::from)
}

fn repair_prompt(user: &str, failed_answer: &str, err: &Error) -> String {
    format!(
        "{user}\n\n## Previous answer (failed validation)\n\n{failed_answer}\n\n\
         ## Findings\n\n{err}\n\n\
         Produce a corrected, complete answer that resolves every finding."
    )
}
