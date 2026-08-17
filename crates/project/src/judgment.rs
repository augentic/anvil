//! The engine guests' judgment kernel.
//!
//! Each leg is one schema-gated [`Model::create`] bracketed by
//! deterministic tails inside a bounded repair loop ([`MAX_REPAIRS`]).

use error::Error;
use omnia_guest::Model;
use omnia_guest::model::{Format, McpGrant, Message, Reply, Request, Role, SchemaFormat, Tool};
use tracing::Instrument as _;

/// Maximum repair attempts after the first answer — a tail failure
/// re-prompts with the findings inlined at most this many times before
/// the leg surfaces the last failure.
pub const MAX_REPAIRS: usize = 2;

/// The agent-facing surface one leg lends beyond the prompt.
///
/// MCP servers granted on every attempt (RFC-96 D9, synthesis only)
/// and the lent workspace — `None` lends the guest's own `"."`
/// preopen; synthesis lends its staged tree (RFC-96 D10).
#[derive(Debug, Default)]
pub struct Lent {
    /// MCP servers offered to the agent on every attempt.
    pub grants: Vec<McpGrant>,
    /// The lent tree; `None` is the guest's own `"."` preopen.
    pub workspace: Option<String>,
}

/// Issue one schema-gated judgment leg with a bounded repair loop.
///
/// `tail` is the deterministic validation over the raw answer: a tail
/// failure re-prompts with the findings inlined; a model failure is
/// never repaired. The host `create` gate enforces the schema live;
/// the tail's typed parse covers the scripted backends. `lent` is the
/// agent-facing surface — MCP grants plus the lent workspace.
/// `subject` is the bounded id of what the leg judges (a domain, a
/// plan, a slice) — a span label and repair-line prefix, never prose.
///
/// # Errors
///
/// The mapped model failure or the last tail failure once the repair
/// budget is exhausted.
#[expect(
    clippy::too_many_arguments,
    reason = "one leg is one call: prompt channels, identity, and lent surface travel together"
)]
pub async fn repaired<P, T, F>(
    model: &P, system: &str, user: String, schema_name: &str, subject: Option<&str>, schema: &str,
    lent: Lent, mut tail: F,
) -> Result<T, Error>
where
    P: Model,
    F: FnMut(&str) -> Result<T, Error>,
{
    // The span carries only the bounded leg name, subject id, and
    // repair count — never prompts or answers.
    let span = tracing::info_span!(
        "judgment.leg",
        leg = %schema_name,
        subject = subject.unwrap_or(""),
        repairs = tracing::field::Empty,
    );
    let label = subject
        .map_or_else(|| schema_name.to_string(), |subject| format!("{schema_name} {subject}"));
    async {
        let mut prompt = user.clone();
        let mut attempt = 0;

        loop {
            let reply = create(model, system, prompt, schema_name, schema, &lent).await?;
            match tail(&reply.answer) {
                Ok(value) => {
                    tracing::Span::current().record("repairs", attempt);
                    return Ok(value);
                }
                Err(err) if attempt < MAX_REPAIRS => {
                    attempt += 1;
                    tracing::Span::current().record("repairs", attempt);
                    tracing::info!("{label} — repair {attempt}: {}", err.variant_str());
                    prompt = repair_prompt(&user, &reply.answer, &err);
                }
                Err(err) => {
                    tracing::Span::current().record("repairs", attempt);
                    return Err(err);
                }
            }
        }
    }
    .instrument(span)
    .await
}

/// One `create` call: schema-constrained output plus the leg's lent
/// agent surface (MCP grants, workspace).
async fn create<P: Model>(
    model: &P, system: &str, user: String, schema_name: &str, schema: &str, lent: &Lent,
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
                .tools(lent.grants.iter().cloned().map(Tool::Mcp).collect())
                .workspace(lent.workspace.as_deref().unwrap_or("."))
                .build(),
        )
        .await
        .map_err(|err| model_error(schema_name, &err))
}

/// Map a typed model failure onto the engine error currency. One code
/// covers every variant — the variant detail rides in the message and
/// no caller recovers differently per variant.
fn model_error(schema_name: &str, err: &omnia_guest::model::Error) -> Error {
    Error::Diag {
        code: "judgment-model-failed",
        detail: format!("{schema_name} judgment call failed: {err}"),
    }
}

/// Serialise a prompt payload as pretty JSON, mapping the (practically
/// impossible) serialisation failure onto the engine error currency.
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
