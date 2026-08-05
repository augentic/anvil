//! The Claude Code model backend: a spawned `claude --print` agent behind the
//! `omnia:model/completion` boundary.
//!
//! Structurally a sibling of `omnia_cursor::model` — same dual inactivity/cap
//! watchdog, same two-attempt `Format::repair` loop over a resumed session.
//! What differs:
//!
//! - The prompt is piped on stdin rather than spilled to a file the CLI is
//!   pointed at. Claude audits its instructions: told to "follow every
//!   instruction in the file at <path>" and then to reply with only JSON and
//!   no reasoning, it reads the pair as a prompt-injection sequence and
//!   refuses outright. On stdin the prompt is an ordinary user turn, and the
//!   pipe has no `ARG_MAX` ceiling either.
//! - Schema answers set `--json-schema`, so conformance is the CLI's job
//!   rather than a promise extracted from the model. The terminal event then
//!   carries the parsed value in `structured_output`, and a schema the CLI
//!   will not accept falls back to the prompt instruction alone.
//! - The terminal event's `permission_denials` are logged, so a refused tool
//!   call is visible rather than showing up later as an inexplicable answer.
//! - MCP servers ride on `--mcp-config` rather than a snapshotted project file,
//!   so there is no on-disk guard to install and restore around each spawn.
//! - Tool calls arrive as `tool_use` / `tool_result` content blocks inside
//!   `assistant` / `user` events, not as top-level `tool_call` events, so the
//!   transcript builder pairs them by `tool_use_id`.
//! - The terminal `result` event reports token usage, so `Answer::usage` is
//!   populated rather than always `None`.

use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail, ensure};
use omnia::Backend;
use omnia_wasi_model::{
    Answer, Format, FutureResult, Mcp, Request, ToolHost, ToolTurn, Transcript, Usage, WasiModelCtx,
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{
    AsyncBufReadExt as _, AsyncRead, AsyncReadExt as _, AsyncWriteExt as _, BufReader,
};
use tokio::process::Command;
use tokio::time::Instant;
use tracing::instrument;

use crate::env;

const CLAUDE_BIN: &str = "claude";
const PROMPT_PREVIEW_CHARS: usize = 500;
const TEXT_PREVIEW_CHARS: usize = 300;

/// Spawned, filesystem-capable Claude Code model backend.
#[derive(Clone)]
pub struct Client {
    timeout: Duration,
    /// Kill a spawn after this long with no stream-json events.
    inactivity: Duration,
    /// Default model id when a request leaves `model` unset.
    model: Option<String>,
    /// Force API-key auth (`--bare`) instead of the CLI's stored login.
    bare: bool,
}

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl Backend for Client {
    type ConnectOptions = ConnectOptions;

    #[instrument]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        check_claude().await?;

        Ok(Self {
            timeout: Duration::from_secs(options.timeout_secs),
            inactivity: Duration::from_secs(options.inactivity_secs),
            model: options.model.filter(|id| !id.trim().is_empty()),
            bare: options.bare,
        })
    }
}

/// Connection options for the Claude Code backend, mirroring the `CURSOR_*`
/// set the cursor backend reads.
///
/// The working tree is lent per completion through the guest's
/// `grants.workspace`, which the host resolves to a node-local path on the
/// tool host.
#[derive(Debug, Clone)]
pub struct ConnectOptions {
    /// Default model id when a request leaves `model` unset; omitted means
    /// the `claude` CLI chooses.
    pub model: Option<String>,
    /// Absolute wall-clock cap in seconds on one `claude` spawn; orphaned
    /// processes are killed on timeout.
    pub timeout_secs: u64,
    /// Inactivity bound in seconds: a spawn is killed after this long with no
    /// stream-json events, so a stalled agent dies fast while one that is
    /// still streaming survives up to the absolute cap.
    pub inactivity_secs: u64,
    /// Pass `--bare`, which forces `ANTHROPIC_API_KEY` and ignores the CLI's
    /// stored OAuth/subscription login. Off by default.
    pub bare: bool,
}

impl omnia::FromEnv for ConnectOptions {
    fn load_env() -> Result<Self> {
        Ok(Self {
            model: env::var("CLAUDE_MODEL"),
            timeout_secs: env::secs("CLAUDE_TIMEOUT_SECS", 600)?,
            inactivity_secs: env::secs("CLAUDE_INACTIVITY_SECS", 120)?,
            bare: env::flag("CLAUDE_BARE"),
        })
    }
}

/// Verify `claude` is on `PATH` and responds to `--version`.
///
/// # Errors
///
/// Returns an error when the binary is absent from `PATH` or `--version`
/// exits non-zero.
pub async fn check_claude() -> Result<()> {
    let status = Command::new(CLAUDE_BIN)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .context("claude not found")?;
    ensure!(status.success(), "`{CLAUDE_BIN} --version` failed ({status})");
    Ok(())
}

struct SpawnOptions<'a> {
    model: Option<&'a str>,
    workspace: &'a Path,
    timeout: Duration,
    inactivity: Duration,
    /// The `--mcp-config` payload, when the request granted MCP servers.
    mcp_config: Option<String>,
    /// The `--json-schema` payload, when the format constrains the answer.
    schema: Option<String>,
    bare: bool,
}

/// The `--json-schema` payload for a format, if it constrains the answer.
///
/// A schema format passes its own schema through. A bare JSON format has none
/// of its own, so it passes the weakest schema that still means "an object" —
/// which is exactly what [`Format::check`] enforces for it. Text formats
/// constrain nothing and go without the flag.
fn json_schema(format: &Format) -> Option<String> {
    match format {
        Format::Schema(spec) => Some(without_dialect(&spec.schema)),
        Format::Json => Some(r#"{"type":"object"}"#.to_owned()),
        Format::Text => None,
    }
}

/// Drop a root `$schema` declaration.
///
/// The CLI resolves `$schema` against a registry of dialects it knows and
/// rejects the whole document when the URI is not one of them — including
/// `https://json-schema.org/draft/2020-12/schema`, which guest schemas
/// routinely declare. Without the key it infers a dialect and accepts the same
/// document. The host has already validated the schema, so nothing is lost by
/// not restating which dialect it was written against.
fn without_dialect(schema: &str) -> String {
    let Ok(Value::Object(mut document)) = serde_json::from_str::<Value>(schema) else {
        return schema.to_owned();
    };
    if document.remove("$schema").is_none() {
        return schema.to_owned();
    }
    Value::Object(document).to_string()
}

#[derive(Debug)]
struct AgentOutput {
    result: String,
    /// The CLI's own schema-validated answer, when the spawn carried
    /// `--json-schema`. Preferred over reparsing `result`.
    structured: Option<Value>,
    transcript: Option<Transcript>,
    usage: Option<Usage>,
    /// The spawn's `session_id` from the stream, for `--resume` repairs.
    session_id: Option<String>,
}

impl WasiModelCtx for Client {
    fn complete(&self, request: Request, tool_host: Arc<dyn ToolHost>) -> FutureResult<Answer> {
        let workspace = tool_host.local_path().map(Path::to_path_buf);
        let timeout = self.timeout;
        let inactivity = self.inactivity;
        let default_model = self.model.clone();
        let bare = self.bare;

        Box::pin(async move {
            let format = &request.format;
            let mut prompt = request.to_string();

            let Some(workspace) = workspace else {
                bail!("no local tree on this node");
            };
            fs::create_dir_all(&workspace)
                .with_context(|| format!("creating {}", workspace.display()))?;
            let workspace = workspace
                .canonicalize()
                .with_context(|| format!("canonicalizing {}", workspace.display()))?;

            // Per-prompt MCP grants carry their own endpoint URL.
            // No grant means no MCP wiring (MCP is opt-in per completion).
            let mcp_servers = request.mcp_servers();
            let mcp_names: Vec<&str> = mcp_servers.iter().map(|s| s.name.as_str()).collect();
            let mcp_config = if mcp_servers.is_empty() {
                None
            } else {
                prompt = format!("{}\n\n{prompt}", mcp_hint(&mcp_servers));
                Some(mcp_config(&mcp_servers))
            };

            // Guest-supplied request.model wins; else CLAUDE_MODEL; else the
            // CLI chooses.
            let spawn = SpawnOptions {
                model: request.model.as_deref().or(default_model.as_deref()),
                workspace: &workspace,
                timeout,
                inactivity,
                mcp_config,
                schema: json_schema(format),
                bare,
            };

            log_completion(
                spawn.model,
                format,
                prompt.len(),
                &mcp_names,
                request.grants.references.is_some(),
            );

            let output = spawn_agent(&prompt, &spawn, None).await?;
            log_attempt(1, &output);
            let session_id = output.session_id.clone();
            let resume;
            match take_answer(format, output, false) {
                Outcome::Done(answer) => return Ok(answer),
                Outcome::Repair { result, reason } => {
                    tracing::debug!(
                        attempt = 1,
                        %reason,
                        resumes = session_id.is_some(),
                        "repairing claude answer"
                    );
                    (prompt, resume) = repair_plan(&prompt, &result, &reason, format, session_id);
                }
            }

            let output = spawn_agent(&prompt, &spawn, resume.as_deref()).await?;
            log_attempt(2, &output);
            match take_answer(format, output, true) {
                Outcome::Done(answer) => Ok(answer),
                Outcome::Repair { result, reason } => {
                    bail!(
                        "claude did not return an answer after 2 attempts: {reason}; it replied: \
                         {}",
                        truncate(&result, TEXT_PREVIEW_CHARS)
                    );
                }
            }
        })
    }
}

/// The second attempt's prompt and the session to resume, if any.
///
/// With a session id from the first spawn, the repair resumes that session and
/// sends only the format-repair instruction (the reason is embedded; the
/// session already carries the failed answer). Without one, it falls back to a
/// cold spawn whose prompt keeps the original as a byte-identical prefix — so
/// provider-side prompt caching stays warm — with the failed answer and the
/// repair instruction appended.
fn repair_plan(
    prompt: &str, answer: &str, reason: &str, format: &Format, session_id: Option<String>,
) -> (String, Option<String>) {
    session_id.map_or_else(
        || (append_repair(prompt, answer, reason, format), None),
        |id| (format.repair(reason), Some(id)),
    )
}

enum Outcome {
    Done(Answer),
    Repair { result: String, reason: String },
}

/// Interpret one spawn's output as an answer, or as grounds for a repair.
///
/// `structured_output` is preferred when present: the CLI already validated it
/// against the same schema, so reparsing the prose `result` can only lose.
fn take_answer(format: &Format, output: AgentOutput, last: bool) -> Outcome {
    let AgentOutput {
        result,
        structured,
        transcript,
        usage,
        ..
    } = output;

    let parsed = structured.map_or_else(|| format.parse(&result), Ok);
    match parsed {
        Ok(value) => match format.check(&value) {
            Err(reason) if !last => Outcome::Repair { result, reason },
            // Wrong shape is better than no answer on the last attempt.
            _ => Outcome::Done(Answer {
                value,
                usage,
                transcript,
            }),
        },
        Err(reason) => Outcome::Repair { result, reason },
    }
}

fn append_repair(prompt: &str, answer: &str, reason: &str, format: &Format) -> String {
    format!("{prompt}\n\nYour previous answer was:\n{answer}\n\n{}", format.repair(reason))
}

/// The `--mcp-config` payload for the granted servers.
///
/// `type` is mandatory: the CLI silently skips a server entry that carries a
/// `url` without one, which surfaces later as an unexplained bad answer rather
/// than a startup error.
fn mcp_config(servers: &[&Mcp]) -> String {
    let entries: serde_json::Map<String, Value> = servers
        .iter()
        .map(|server| (server.name.clone(), json!({ "type": "http", "url": server.url })))
        .collect();
    json!({ "mcpServers": entries }).to_string()
}

// A natural-language hint naming the granted MCP servers and any tool allowlist,
// prepended so the spawned agent prefers them over assumptions.
fn mcp_hint(servers: &[&Mcp]) -> String {
    let lines: Vec<String> = servers
        .iter()
        .map(|server| {
            if server.tools.is_empty() {
                format!("- `{}`", server.name)
            } else {
                format!("- `{}` (use only: {})", server.name, server.tools.join(", "))
            }
        })
        .collect();
    format!(
        "The following read-only MCP servers are available. Consult their tools and resources for \
         authoritative reference material before answering, and prefer that material over \
         assumptions:\n{}",
        lines.join("\n")
    )
}

/// The `claude` invocation for one spawn; `resume` re-enters the named session
/// instead of starting a fresh one, and `schema` is dropped when a previous
/// spawn showed the CLI will not accept it.
///
/// The prompt is not here: it goes in on stdin. Every optional flag still uses
/// the attached `--flag=value` form, because `--add-dir` and `--mcp-config`
/// are variadic (`<directories...>`, `<configs...>`) and `--resume` takes an
/// optional value, so in the detached form each will swallow whatever follows
/// it.
///
/// Permissions are `bypassPermissions`, which is broader than the lent
/// workspace. `acceptEdits` is the tighter mode and does confine a spawn to
/// the allowed directories, but it is not usable here: adapter prompts hand
/// the agent absolute paths into the project tree the workspace was derived
/// from, and MCP tool calls need approval under that mode too, so the gate
/// denies the reference lookups and artifact reads the leg exists to perform.
/// Confining a spawn is therefore the workspace-lending layer's job, not this
/// flag's. Denials are logged by [`warn_denials`] under either mode.
fn agent_command(
    options: &SpawnOptions<'_>, resume: Option<&str>, schema: Option<&str>,
) -> Command {
    let mut cmd = Command::new(CLAUDE_BIN);
    cmd.kill_on_drop(true)
        .current_dir(options.workspace)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(["--print", "--output-format", "stream-json", "--verbose"])
        .arg("--permission-mode=bypassPermissions")
        .arg(format!("--add-dir={}", options.workspace.display()));
    if let Some(config) = &options.mcp_config {
        // `--strict-mcp-config` keeps a project-local `.mcp.json` from leaking
        // servers the request never granted.
        cmd.arg(format!("--mcp-config={config}")).arg("--strict-mcp-config");
    }
    if let Some(schema) = schema {
        cmd.arg(format!("--json-schema={schema}"));
    }
    if options.bare {
        cmd.arg("--bare");
    }
    if let Some(model) = options.model {
        cmd.arg(format!("--model={model}"));
    }
    if let Some(session_id) = resume {
        cmd.arg(format!("--resume={session_id}"));
    }
    cmd
}

/// One completion attempt, retried once without `--json-schema` when the CLI
/// turns out not to accept this schema.
///
/// The schema comes from the guest and may use JSON Schema the CLI's
/// structured-output support does not cover. Dropping the flag costs the hard
/// guarantee but leaves the prompt's own format instruction and the
/// `Format::repair` loop, which is strictly better than failing the leg.
#[instrument(skip(prompt, options, resume), fields(model = options.model))]
async fn spawn_agent(
    prompt: &str, options: &SpawnOptions<'_>, resume: Option<&str>,
) -> Result<AgentOutput> {
    tracing::debug!(
        prompt_len = prompt.len(),
        resume,
        schema = options.schema.is_some(),
        preview = %truncate(prompt, PROMPT_PREVIEW_CHARS),
        "claude prompt"
    );

    let schema = options.schema.as_deref();
    match run_agent(prompt, options, resume, schema).await {
        Err(error) if schema.is_some() && rejected_schema(&error) => {
            tracing::warn!(
                %error,
                "claude rejected --json-schema; retrying on the prompt's format instruction alone"
            );
            run_agent(prompt, options, resume, None).await
        }
        outcome => outcome,
    }
}

/// Whether a failed spawn blamed the schema, rather than the model or the
/// network. Matched loosely: a needless retry costs one spawn, a missed one
/// costs the leg.
fn rejected_schema(error: &anyhow::Error) -> bool {
    error.to_string().to_lowercase().contains("schema")
}

async fn run_agent(
    prompt: &str, options: &SpawnOptions<'_>, resume: Option<&str>, schema: Option<&str>,
) -> Result<AgentOutput> {
    let mut child = agent_command(options, resume, schema)
        .spawn()
        .with_context(|| format!("spawning `{CLAUDE_BIN}`"))?;
    let stdin = child.stdin.take().context("child stdin is piped")?;
    let stdout = child.stdout.take().context("child stdout is piped")?;
    let stderr = child.stderr.take().context("child stderr is piped")?;

    // Parse stdout as it streams so memory stays bounded on chatty runs, and
    // drain stderr concurrently so the child can never block on a full pipe.
    // Feeding stdin joins them rather than preceding them: a prompt larger
    // than the pipe buffer would otherwise block on a child that is itself
    // blocked writing to a stdout nobody is reading.
    let activity = Activity::now();
    let drive = async {
        let (parsed, stderr, ()) =
            tokio::join!(parse_stream(stdout, &activity), drain(stderr), feed(stdin, prompt));
        let status = child.wait().await.with_context(|| format!("waiting on `{CLAUDE_BIN}`"))?;
        anyhow::Ok((parsed, stderr, status))
    };

    // On timeout `drive` is dropped, and `kill_on_drop` reaps the orphaned agent.
    let deadlines = Deadlines {
        inactivity: options.inactivity,
        cap: options.timeout,
    };
    let (parsed, stderr, status) = tokio::select! {
        driven = drive => driven?,
        error = watchdog(&activity, &deadlines) => return Err(error),
    };

    if !status.success() {
        bail!("claude exited with {status}: {}", String::from_utf8_lossy(&stderr).trim());
    }

    parsed
}

/// Write the prompt to the child and close the pipe, which is what tells
/// `--print` the turn is complete.
///
/// A write that fails means the child is already gone, and its exit status and
/// stderr say why far better than a broken pipe does — so this reports nothing
/// upward and lets the caller surface the real failure.
async fn feed(mut stdin: tokio::process::ChildStdin, prompt: &str) {
    if let Err(error) = stdin.write_all(prompt.as_bytes()).await {
        tracing::debug!(%error, "claude closed stdin before the prompt was written");
        return;
    }
    if let Err(error) = stdin.shutdown().await {
        tracing::debug!(%error, "closing claude stdin");
    }
}

/// Last-seen stream progress; every stdout line from the agent counts.
struct Activity(std::sync::Mutex<Instant>);

impl Activity {
    fn now() -> Self {
        Self(std::sync::Mutex::new(Instant::now()))
    }

    fn touch(&self) {
        *self.0.lock().expect("activity lock is never poisoned") = Instant::now();
    }

    fn last(&self) -> Instant {
        *self.0.lock().expect("activity lock is never poisoned")
    }
}

/// The two spawn bounds: a short inactivity window over stream events and a
/// generous absolute wall-clock cap.
struct Deadlines {
    inactivity: Duration,
    cap: Duration,
}

/// Resolves when a spawn breaches either bound; the error names which one, so
/// "stalled agent" and "agent that outlived the cap" stay distinguishable.
async fn watchdog(activity: &Activity, deadlines: &Deadlines) -> anyhow::Error {
    let start = Instant::now();
    loop {
        let now = Instant::now();
        let idle = now.saturating_duration_since(activity.last());
        if idle >= deadlines.inactivity {
            return anyhow!(
                "claude inactive for {}s (no stream events; inactivity limit {}s, absolute cap \
                 {}s)",
                idle.as_secs(),
                deadlines.inactivity.as_secs(),
                deadlines.cap.as_secs()
            );
        }
        let elapsed = now.saturating_duration_since(start);
        if elapsed >= deadlines.cap {
            return anyhow!(
                "claude timed out after {}s (absolute cap exceeded while still active)",
                deadlines.cap.as_secs()
            );
        }
        let next_check =
            deadlines.inactivity.saturating_sub(idle).min(deadlines.cap.saturating_sub(elapsed));
        tokio::time::sleep(next_check).await;
    }
}

async fn parse_stream(stdout: impl AsyncRead + Unpin, activity: &Activity) -> Result<AgentOutput> {
    let mut lines = BufReader::new(stdout).lines();
    let mut parser = OutputParser::default();
    while let Some(line) = lines.next_line().await? {
        activity.touch();
        parser.line(&line)?;
    }
    parser.finish()
}

async fn drain(mut stream: impl AsyncRead + Unpin) -> Vec<u8> {
    let mut buffer = Vec::new();
    drop(stream.read_to_end(&mut buffer).await);
    buffer
}

/// One-line INFO for the completion start.
fn log_completion(
    model: Option<&str>, format: &Format, prompt_len: usize, mcp_servers: &[&str],
    has_references: bool,
) {
    match format {
        Format::Text => {
            tracing::info!(
                model,
                format = "text",
                prompt_len,
                ?mcp_servers,
                has_references,
                "claude completion"
            );
        }
        Format::Json => {
            tracing::info!(
                model,
                format = "json",
                prompt_len,
                ?mcp_servers,
                has_references,
                "claude completion"
            );
        }
        Format::Schema(spec) => {
            tracing::info!(
                model,
                format = "schema",
                schema_name = %spec.name,
                prompt_len,
                ?mcp_servers,
                has_references,
                "claude completion"
            );
            tracing::trace!(
                schema_name = %spec.name,
                schema = %truncate(&spec.schema, PROMPT_PREVIEW_CHARS),
                "claude completion schema"
            );
        }
    }
}

fn log_attempt(attempt: u32, output: &AgentOutput) {
    let (interesting_tools, noisy_tools) = tool_counts(output.transcript.as_ref());
    tracing::debug!(
        attempt,
        result_len = output.result.len(),
        structured = output.structured.is_some(),
        interesting_tools,
        noisy_tools,
        preview = %truncate(&output.result, TEXT_PREVIEW_CHARS),
        "claude answer"
    );
}

/// A refused tool call is how a spawn reaching outside the lent workspace
/// shows up. It is not fatal — the agent sees the denial and carries on — but
/// it is the only signal that the boundary did something, so it gets a WARN.
fn warn_denials(denials: &[PermissionDenial]) {
    for denial in denials {
        tracing::warn!(
            tool = denial.tool_name.as_deref().unwrap_or("unknown"),
            args = %args_summary(&denial.tool_input),
            "claude tool call denied by the permission gate (outside the lent workspace?)"
        );
    }
}

fn tool_counts(transcript: Option<&Transcript>) -> (usize, usize) {
    let Some(transcript) = transcript else {
        return (0, 0);
    };
    let mut interesting = 0;
    let mut noisy = 0;
    for turn in &transcript.turns {
        if is_noisy_tool(&turn.tool) {
            noisy += 1;
        } else {
            interesting += 1;
        }
    }
    (interesting, noisy)
}

/// Compact JSON when parseable; otherwise collapse whitespace so a log field stays one line.
fn single_line(text: &str) -> String {
    serde_json::from_str::<Value>(text.trim()).map_or_else(
        |_| text.split_whitespace().collect::<Vec<_>>().join(" "),
        |value| value.to_string(),
    )
}

fn truncate(text: &str, max: usize) -> String {
    let collapsed = single_line(text);
    let mut chars = collapsed.chars();
    let head: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() { format!("{head}…") } else { head }
}

/// The Claude Code built-in tools whose turns are bulk, not signal.
fn is_noisy_tool(name: &str) -> bool {
    matches!(
        name,
        "Read"
            | "Write"
            | "Edit"
            | "MultiEdit"
            | "Bash"
            | "BashOutput"
            | "Grep"
            | "Glob"
            | "LS"
            | "NotebookEdit"
            | "TodoWrite"
            | "KillShell"
    )
}

fn args_summary(args: &Value) -> String {
    for key in ["file_path", "path", "url", "pattern", "query", "command"] {
        if let Some(text) = args.get(key).and_then(Value::as_str) {
            return truncate(text, TEXT_PREVIEW_CHARS);
        }
    }
    truncate(&args.to_string(), TEXT_PREVIEW_CHARS)
}

/// The subset of Claude Code `stream-json` events the backend consumes.
///
/// `system` (the `init` event) carries the `session_id` used to resume on a
/// repair attempt plus each MCP server's connection status; `assistant`
/// carries prose text and `tool_use` blocks; `user` carries the matching
/// `tool_result` blocks; the terminal `result` carries the answer and token
/// usage. Everything else parses to `Other` without building a JSON tree.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Event {
    System {
        session_id: Option<String>,
        #[serde(default)]
        mcp_servers: Vec<McpStatus>,
    },
    Assistant {
        message: Option<TurnMessage>,
        session_id: Option<String>,
    },
    User {
        message: Option<TurnMessage>,
    },
    Result {
        is_error: Option<bool>,
        result: Option<String>,
        session_id: Option<String>,
        usage: Option<UsageReport>,
        /// The schema-validated answer, present only when the spawn carried
        /// `--json-schema`.
        structured_output: Option<Value>,
        #[serde(default)]
        permission_denials: Vec<PermissionDenial>,
    },
    #[serde(other)]
    Other,
}

/// One tool call the permission gate refused, reported on the terminal event.
#[derive(Deserialize)]
struct PermissionDenial {
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_input: Value,
}

/// One `mcp_servers[]` entry on the init event.
#[derive(Deserialize)]
struct McpStatus {
    name: String,
    #[serde(default)]
    status: Option<String>,
}

/// The `message` body of an `assistant` or `user` event.
#[derive(Deserialize)]
struct TurnMessage {
    #[serde(default)]
    content: Vec<ContentBlock>,
}

/// One `message.content[]` entry. Claude tags each block by `type`; the
/// backend reads prose from `text`, opens a tool turn on `tool_use`, and
/// closes it on the `tool_result` that quotes the same `tool_use_id`.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text {
        #[serde(default)]
        text: String,
    },
    Thinking {
        #[serde(default)]
        thinking: String,
    },
    ToolUse {
        id: Option<String>,
        name: Option<String>,
        #[serde(default)]
        input: Value,
    },
    ToolResult {
        tool_use_id: Option<String>,
        #[serde(default)]
        content: Value,
    },
    #[serde(other)]
    Other,
}

/// The `usage` object on the terminal result event. Claude reports cache
/// counters separately; they fold into the input total, which is what the
/// `omnia:model/completion` `usage` record means.
#[derive(Deserialize)]
#[expect(clippy::struct_field_names, reason = "the field names are the wire field names")]
struct UsageReport {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    cache_creation_input_tokens: u32,
    #[serde(default)]
    cache_read_input_tokens: u32,
}

impl UsageReport {
    const fn into_usage(self) -> Usage {
        Usage {
            input_tokens: self
                .input_tokens
                .saturating_add(self.cache_creation_input_tokens)
                .saturating_add(self.cache_read_input_tokens),
            output_tokens: self.output_tokens,
            // Claude Code bills thinking within `output_tokens`.
            reasoning_tokens: None,
        }
    }
}

#[derive(Default)]
struct OutputParser {
    result: Option<String>,
    /// The `--json-schema` validated answer, when the CLI produced one.
    structured: Option<Value>,
    session_id: Option<String>,
    usage: Option<Usage>,
    /// `tool_use_id` -> (tool name, arguments), awaiting its `tool_result`.
    pending_tools: HashMap<String, (String, Value)>,
    turns: Vec<ToolTurn>,
}

impl OutputParser {
    fn line(&mut self, line: &str) -> Result<()> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }

        // One garbled line must not cost an otherwise-successful answer.
        let event = match serde_json::from_str::<Event>(line) {
            Ok(event) => event,
            Err(error) => {
                tracing::debug!(
                    %error,
                    line = %truncate(line, TEXT_PREVIEW_CHARS),
                    "skipping unparsable claude event"
                );
                return Ok(());
            }
        };

        match event {
            Event::System {
                session_id,
                mcp_servers,
            } => {
                self.session(session_id);
                warn_unhealthy_mcp(&mcp_servers);
            }
            Event::Assistant { message, session_id } => {
                self.session(session_id);
                if let Some(message) = message {
                    self.assistant_blocks(message.content);
                }
            }
            Event::User { message } => {
                if let Some(message) = message {
                    self.user_blocks(message.content);
                }
            }
            Event::Result {
                is_error,
                result,
                session_id,
                usage,
                structured_output,
                permission_denials,
            } => {
                self.session(session_id);
                warn_denials(&permission_denials);
                if let Some(usage) = usage {
                    self.usage = Some(usage.into_usage());
                }
                if is_error == Some(true) {
                    bail!(
                        "claude reported an error: {}",
                        result.as_deref().unwrap_or("<no detail>")
                    );
                }
                if structured_output.is_some() {
                    self.structured = structured_output;
                }
                if result.is_some() {
                    self.result = result;
                }
            }
            Event::Other => {
                tracing::trace!(line = %truncate(line, TEXT_PREVIEW_CHARS), "claude other event");
            }
        }
        Ok(())
    }

    /// Keep the first `session_id` seen (the `init` event's; later events
    /// repeat it as a fallback).
    fn session(&mut self, session_id: Option<String>) {
        if self.session_id.is_none() {
            self.session_id = session_id;
        }
    }

    fn assistant_blocks(&mut self, blocks: Vec<ContentBlock>) {
        for block in blocks {
            match block {
                ContentBlock::Text { text } if !text.is_empty() => {
                    tracing::debug!(
                        text = %truncate(&text, TEXT_PREVIEW_CHARS),
                        "claude assistant text"
                    );
                }
                ContentBlock::Thinking { thinking } if !thinking.is_empty() => {
                    tracing::debug!("thinking: {}", truncate(&thinking, TEXT_PREVIEW_CHARS));
                }
                ContentBlock::ToolUse { id, name, input } => {
                    let tool = name.unwrap_or_else(|| "unknown".to_owned());
                    if is_noisy_tool(&tool) {
                        tracing::trace!(%tool, "claude tool call");
                    } else {
                        tracing::debug!(%tool, args = %args_summary(&input), "claude tool");
                    }
                    if let Some(id) = id {
                        self.pending_tools.insert(id, (tool, input));
                    }
                }
                _ => {}
            }
        }
    }

    /// A `user` event closes the tool turns opened by the preceding
    /// `assistant` event. A result quoting an unknown id is dropped rather
    /// than recorded against a guessed tool.
    fn user_blocks(&mut self, blocks: Vec<ContentBlock>) {
        for block in blocks {
            if let ContentBlock::ToolResult {
                tool_use_id: Some(id),
                content,
            } = block
                && let Some((tool, args)) = self.pending_tools.remove(&id)
            {
                self.turns.push(ToolTurn {
                    tool,
                    args,
                    result: content,
                });
            }
        }
    }

    fn finish(self) -> Result<AgentOutput> {
        let Some(result) = self.result else {
            bail!("claude did not emit a terminal result event");
        };
        let transcript =
            if self.turns.is_empty() { None } else { Some(Transcript { turns: self.turns }) };
        Ok(AgentOutput {
            result,
            structured: self.structured,
            transcript,
            usage: self.usage,
            session_id: self.session_id,
        })
    }
}

/// A server the CLI silently skipped otherwise looks like a bad answer, so the
/// init event's per-server status is worth a WARN of its own.
fn warn_unhealthy_mcp(servers: &[McpStatus]) {
    for server in servers {
        let status = server.status.as_deref().unwrap_or("unknown");
        if !status.eq_ignore_ascii_case("connected") {
            tracing::warn!(
                server = %server.name,
                %status,
                "claude MCP server is not connected; its tools are unavailable to this completion"
            );
        }
    }
}

// Deliberate unit tests: pure stream-parse, MCP-config, and prompt-build logic.
// The live acceptance gate is `tests/live.rs`, which is `#[ignore]`d.
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use omnia_wasi_model::{Format, Mcp};
    use serde_json::json;

    use super::{
        Activity, AgentOutput, Deadlines, OutputParser, SpawnOptions, agent_command, json_schema,
        mcp_config, repair_plan, single_line, truncate, watchdog,
    };

    fn parse_output(stdout: &str) -> anyhow::Result<AgentOutput> {
        let mut parser = OutputParser::default();
        for line in stdout.lines() {
            parser.line(line)?;
        }
        parser.finish()
    }

    #[test]
    fn single_line_compacts_json() {
        let pretty = "{\n  \"verdict\": \"pass\"\n}";
        assert_eq!(single_line(pretty), r#"{"verdict":"pass"}"#);
    }

    #[test]
    fn truncate_appends_ellipsis() {
        assert_eq!(truncate("abcdef", 3), "abc…");
        assert_eq!(truncate("ab", 3), "ab");
    }

    #[test]
    fn parse_stream_json() {
        let stdout = r#"{"type":"system","subtype":"init","cwd":"/ws","session_id":"s-init","tools":["Read"],"mcp_servers":[]}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"I'll read the README"},{"type":"tool_use","id":"toolu_1","name":"Read","input":{"file_path":"README.md"}}]},"session_id":"s-init"}
{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"hi"}]},"session_id":"s-init"}
{"type":"result","subtype":"success","is_error":false,"result":"{\"verdict\":\"pass\"}","session_id":"s-init","usage":{"input_tokens":10,"output_tokens":4,"cache_read_input_tokens":90}}"#;
        let output = parse_output(stdout).expect("parse stream");
        assert_eq!(output.result, r#"{"verdict":"pass"}"#);
        assert_eq!(output.session_id.as_deref(), Some("s-init"));

        let transcript = output.transcript.expect("tool transcript");
        assert_eq!(transcript.turns.len(), 1);
        assert_eq!(transcript.turns[0].tool, "Read");
        assert_eq!(transcript.turns[0].args, json!({ "file_path": "README.md" }));
        assert_eq!(transcript.turns[0].result, json!("hi"));

        let usage = output.usage.expect("the result event reports usage");
        assert_eq!(usage.input_tokens, 100, "cache counters fold into the input total");
        assert_eq!(usage.output_tokens, 4);
        assert!(usage.reasoning_tokens.is_none());
    }

    #[test]
    fn parse_result_error() {
        let stdout = r#"{"type":"result","subtype":"error_during_execution","is_error":true,"result":"boom"}"#;
        let error = parse_output(stdout).expect_err("an agent error must surface");
        assert!(error.to_string().contains("claude reported an error"), "unexpected: {error}");
    }

    #[test]
    fn parse_missing_result() {
        let stdout = r#"{"type":"system","subtype":"init","session_id":"s-1"}"#;
        let error = parse_output(stdout).expect_err("a stream with no result must fail");
        assert!(
            error.to_string().contains("did not emit a terminal result event"),
            "unexpected: {error}"
        );
    }

    #[test]
    fn skip_garbled_line() {
        let stdout =
            "warning: not an event\n{\"type\":\"result\",\"is_error\":false,\"result\":\"ok\"}";
        let output = parse_output(stdout).expect("garbled line is skipped");
        assert_eq!(output.result, "ok");
        assert!(output.usage.is_none(), "a result without usage reports none");
    }

    #[test]
    fn unknown_event_type_is_ignored() {
        let stdout = r#"{"type":"stream_event","event":{"whatever":true}}
{"type":"result","is_error":false,"result":"ok"}"#;
        let output = parse_output(stdout).expect("unknown events tolerate");
        assert_eq!(output.result, "ok");
    }

    #[test]
    fn unpaired_tool_result_is_dropped() {
        let stdout = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_missing","content":"orphan"}]}}
{"type":"result","is_error":false,"result":"ok"}"#;
        let output = parse_output(stdout).expect("parse stream");
        assert!(output.transcript.is_none(), "an unpaired result is not a tool turn");
    }

    #[test]
    fn mcp_config_tags_the_transport() {
        let servers = [
            Mcp {
                name: "docs".to_owned(),
                tools: vec![],
                url: "http://127.0.0.1:8080/mcp".to_owned(),
            },
            Mcp {
                name: "specs".to_owned(),
                tools: vec!["search".to_owned()],
                url: "http://127.0.0.1:8080/mcp/specs".to_owned(),
            },
        ];
        let refs: Vec<&Mcp> = servers.iter().collect();
        let config: serde_json::Value =
            serde_json::from_str(&mcp_config(&refs)).expect("the config is JSON");
        assert_eq!(
            config,
            json!({
                "mcpServers": {
                    "docs": { "type": "http", "url": "http://127.0.0.1:8080/mcp" },
                    "specs": { "type": "http", "url": "http://127.0.0.1:8080/mcp/specs" },
                }
            }),
            "every entry carries the mandatory `type`, and the tool allowlist stays in the prompt"
        );
    }

    #[test]
    fn structured_output_beats_the_prose_result() {
        let stdout = r#"{"type":"result","subtype":"success","is_error":false,"result":"Here you go: {\"verdict\":\"pass\"}","structured_output":{"verdict":"pass"},"session_id":"s-1"}"#;
        let output = parse_output(stdout).expect("parse stream");
        assert_eq!(
            output.structured,
            Some(json!({ "verdict": "pass" })),
            "the CLI's validated answer is kept apart from the prose it was wrapped in"
        );
    }

    #[test]
    fn schema_only_for_constrained_formats() {
        assert!(json_schema(&Format::Text).is_none(), "text constrains nothing");
        assert_eq!(json_schema(&Format::Json).as_deref(), Some(r#"{"type":"object"}"#));
        let spec = Format::Schema(omnia_wasi_model::Schema {
            name: "verdict".to_owned(),
            schema: r#"{"type":"object","required":["verdict"]}"#.to_owned(),
        });
        assert_eq!(
            json_schema(&spec).as_deref(),
            Some(r#"{"type":"object","required":["verdict"]}"#),
            "a schema format passes its own schema through"
        );
    }

    /// The CLI rejects the whole document when `$schema` names a dialect it
    /// does not have registered, which the 2020-12 URI guests declare is.
    #[test]
    fn dialect_declaration_is_dropped() {
        let declared = Format::Schema(omnia_wasi_model::Schema {
            name: "verdict".to_owned(),
            schema: r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object"}"#
                .to_owned(),
        });
        assert_eq!(json_schema(&declared).as_deref(), Some(r#"{"type":"object"}"#));
    }

    #[test]
    fn unparseable_schema_is_left_alone() {
        assert_eq!(super::without_dialect("not json"), "not json");
    }

    mod repair {
        use super::{Format, repair_plan};

        #[test]
        fn resumes_with_findings_only() {
            let (prompt, resume) = repair_plan(
                "the original prompt",
                "not json",
                "answer is not valid JSON",
                &Format::Json,
                Some("s-1".to_owned()),
            );
            assert_eq!(resume.as_deref(), Some("s-1"));
            assert!(
                !prompt.contains("the original prompt"),
                "a resumed repair must not re-send the original prompt: {prompt}"
            );
            assert!(prompt.contains("answer is not valid JSON"), "findings ride along: {prompt}");
        }

        #[test]
        fn cold_fallback_keeps_prompt_prefix() {
            let (prompt, resume) = repair_plan(
                "the original prompt",
                "not json",
                "answer is not valid JSON",
                &Format::Json,
                None,
            );
            assert!(resume.is_none());
            assert!(
                prompt.starts_with("the original prompt"),
                "the fallback keeps a byte-identical prompt prefix for provider caching: {prompt}"
            );
            assert!(prompt.contains("not json"), "the failed answer is appended: {prompt}");
        }
    }

    mod spawn_args {
        use super::{Duration, SpawnOptions, agent_command};

        fn options(workspace: &std::path::Path, mcp_config: Option<String>) -> SpawnOptions<'_> {
            SpawnOptions {
                model: None,
                workspace,
                timeout: Duration::from_mins(10),
                inactivity: Duration::from_mins(2),
                mcp_config,
                schema: None,
                bare: false,
            }
        }

        fn args(cmd: &tokio::process::Command) -> Vec<String> {
            cmd.as_std().get_args().map(|a| a.to_string_lossy().into_owned()).collect()
        }

        /// `--add-dir` and `--mcp-config` are variadic and `--resume` takes an
        /// optional value, so in the detached form each swallows what follows.
        #[test]
        fn every_optional_flag_uses_the_attached_form() {
            let workspace = std::env::temp_dir();
            let mut options = options(&workspace, Some(r#"{"mcpServers":{}}"#.to_owned()));
            options.model = Some("sonnet");
            options.bare = true;

            let args = args(&agent_command(&options, Some("s-1"), Some(r#"{"type":"object"}"#)));
            for flag in ["--add-dir", "--mcp-config", "--model", "--resume", "--json-schema"] {
                assert!(
                    args.iter().any(|arg| arg.starts_with(&format!("{flag}="))),
                    "`{flag}` must use the attached form: {args:?}"
                );
            }
        }

        /// The prompt goes in on stdin, so no positional argument may appear —
        /// one would be read as the prompt and the piped turn ignored.
        #[test]
        fn the_prompt_is_not_an_argument() {
            let workspace = std::env::temp_dir();
            let args = args(&agent_command(&options(&workspace, None), None, None));
            assert!(
                args.iter().all(|arg| arg.starts_with('-') || is_flag_value(arg)),
                "no positional argument may reach the CLI: {args:?}"
            );
        }

        fn is_flag_value(arg: &str) -> bool {
            matches!(arg, "stream-json" | "--verbose")
        }

        #[test]
        fn stream_json_needs_verbose() {
            let workspace = std::env::temp_dir();
            let args = args(&agent_command(&options(&workspace, None), None, None));
            assert!(args.contains(&"--verbose".to_owned()), "args: {args:?}");
            assert!(args.contains(&"stream-json".to_owned()), "args: {args:?}");
        }

        /// A leg reads artifacts by absolute path outside the lent workspace
        /// and calls MCP tools, both of which `acceptEdits` denies.
        #[test]
        fn permissions_do_not_gate_the_leg() {
            let workspace = std::env::temp_dir();
            let args = args(&agent_command(&options(&workspace, None), None, None));
            assert!(
                args.contains(&"--permission-mode=bypassPermissions".to_owned()),
                "args: {args:?}"
            );
        }

        #[test]
        fn resume_uses_attached_form() {
            let workspace = std::env::temp_dir();
            let args = args(&agent_command(&options(&workspace, None), Some("s-1"), None));
            assert!(args.contains(&"--resume=s-1".to_owned()), "args: {args:?}");
        }

        #[test]
        fn no_schema_means_no_flag() {
            let workspace = std::env::temp_dir();
            let args = args(&agent_command(&options(&workspace, None), None, None));
            assert!(!args.iter().any(|a| a.starts_with("--json-schema")), "args: {args:?}");
        }

        #[test]
        fn no_mcp_grant_means_no_mcp_flags() {
            let workspace = std::env::temp_dir();
            let args = args(&agent_command(&options(&workspace, None), None, None));
            assert!(!args.iter().any(|a| a.starts_with("--mcp-config")), "args: {args:?}");
            assert!(!args.iter().any(|a| a.starts_with("--strict")), "args: {args:?}");
        }

        #[test]
        fn mcp_grant_is_strict() {
            let workspace = std::env::temp_dir();
            let config = r#"{"mcpServers":{}}"#.to_owned();
            let args = args(&agent_command(&options(&workspace, Some(config)), None, None));
            assert!(args.iter().any(|arg| arg.starts_with("--mcp-config=")), "args: {args:?}");
            assert!(
                args.contains(&"--strict-mcp-config".to_owned()),
                "a project .mcp.json must not leak in: {args:?}"
            );
        }
    }

    mod timeouts {
        use tokio::time::{Duration, sleep};

        use super::{Activity, Deadlines, watchdog};

        const DEADLINES: Deadlines = Deadlines {
            inactivity: Duration::from_mins(2),
            cap: Duration::from_mins(10),
        };

        #[tokio::test(start_paused = true)]
        async fn silent_stream_dies_at_inactivity_window() {
            let activity = Activity::now();
            let started = tokio::time::Instant::now();
            let error = watchdog(&activity, &DEADLINES).await;
            assert_eq!(started.elapsed(), Duration::from_mins(2));
            assert!(
                error.to_string().contains("inactive for 120s"),
                "the inactivity kill names the idle span: {error}"
            );
        }

        #[tokio::test(start_paused = true)]
        async fn steady_activity_survives_to_absolute_cap() {
            let activity = Activity::now();
            let started = tokio::time::Instant::now();
            let toucher = async {
                loop {
                    sleep(Duration::from_mins(1)).await;
                    activity.touch();
                }
            };
            let error = tokio::select! {
                error = watchdog(&activity, &DEADLINES) => error,
                () = toucher => unreachable!("the toucher never finishes"),
            };
            assert_eq!(started.elapsed(), Duration::from_mins(10));
            assert!(
                error.to_string().contains("timed out after 600s"),
                "the cap kill names the absolute bound: {error}"
            );
        }
    }
}
