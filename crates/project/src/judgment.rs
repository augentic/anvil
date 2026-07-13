//! The workflow guests' judgment kernel.
//!
//! Each leg is one schema-gated [`Model::create`] bracketed by
//! deterministic tails (schema gate, parse, projection kernel) inside a
//! bounded repair loop: a tail failure re-prompts with the findings
//! inlined, up to [`MAX_REPAIRS`] times. These legs are the sole
//! judgment path, driven by the guest orchestrators — the propose leg
//! in the change crate and the synthesize leg in the slice crate.
//!
//! The model capability is the upstream [`omnia_guest::Model`]: typed
//! errors, the full `omnia:model/completion` request mirror, and a
//! WASI-backed default body on `wasm32`. Native tests bind the scripted
//! mock in each test binary's `mock` module.

use error::Error;
use omnia_guest::Model;
use omnia_guest::model::{Format, Message, Reply, Request, Role, SchemaFormat};

/// Maximum repair attempts after the first answer — a tail failure
/// re-prompts with the findings inlined at most this many times before
/// the leg surfaces the last failure.
pub const MAX_REPAIRS: usize = 2;

/// Issue one schema-gated judgment leg with a bounded repair loop.
///
/// `tail` is the deterministic validation over the raw answer (schema
/// gate, parse, projection kernel). On a tail failure the leg
/// re-prompts with the failed answer and the findings inlined; a model
/// failure is never repaired (the request did not change).
///
/// The tail's schema gate is redundant with the host `create` gate on
/// the live backend (a schema-invalid answer surfaces there as
/// `invalid-answer`, never reaching the tail) — it is belt-and-braces
/// for the mock and replay backends, whose answers are unvalidated.
///
/// # Errors
///
/// The mapped model failure or the last tail failure once the repair
/// budget is exhausted.
pub async fn schema_gated<P, T, F>(
    model: &P, system: &str, user: String, schema_name: &str, schema: &str, mut tail: F,
) -> Result<T, Error>
where
    P: Model,
    F: FnMut(&str) -> Result<T, Error>,
{
    let mut prompt = user.clone();
    let mut attempt = 0;
    loop {
        let reply = create(model, system, prompt, schema_name, schema).await?;
        match tail(&reply.answer) {
            Ok(value) => return Ok(value),
            Err(err) if attempt < MAX_REPAIRS => {
                attempt += 1;
                prompt = repair_prompt(&user, &reply.answer, &err);
            }
            Err(err) => return Err(err),
        }
    }
}

/// One `create` call: schema-constrained output, no MCP grants, the
/// guest's own `"."` preopen lent as the shared workspace.
async fn create<P: Model>(
    model: &P, system: &str, user: String, schema_name: &str, schema: &str,
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
                .lend_workspace(true)
                .build(),
        )
        .await
        .map_err(|err| model_error(schema_name, &err))
}

/// Map a typed model failure onto the workflow error currency. One code
/// covers every variant — the variant detail rides in the message and
/// no caller recovers differently per variant.
fn model_error(schema_name: &str, err: &omnia_guest::model::Error) -> Error {
    Error::Diag {
        code: "judgment-model-failed",
        detail: format!("{schema_name} judgment call failed: {err}"),
    }
}

/// Serialise a prompt payload as pretty JSON, mapping the (practically
/// impossible) serialisation failure onto the workflow error currency.
///
/// # Errors
///
/// `judgment-request-serialise` when serialisation fails.
pub fn render_json<T: serde::Serialize>(value: &T, what: &str) -> Result<String, Error> {
    serde_json::to_string_pretty(value).map_err(|err| Error::Diag {
        code: "judgment-request-serialise",
        detail: format!("failed to serialise {what}: {err}"),
    })
}

/// Assemble the repair prompt: the original request, the answer that
/// failed the deterministic tail, and the findings to correct.
fn repair_prompt(user: &str, failed_answer: &str, err: &Error) -> String {
    format!(
        "{user}\n\n## Previous answer (failed validation)\n\n{failed_answer}\n\n\
         ## Findings\n\n{err}\n\n\
         Produce a corrected, complete answer that resolves every finding."
    )
}
