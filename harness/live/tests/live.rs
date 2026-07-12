//! The explicit live-model workflow test (RFC-62): one ignored,
//! operator-invoked native trial that drives the same fixture
//! workflow as the scripted suites — adversarial lead set, real
//! judgment — and grades it with the deterministic validators only.
//!
//! The lead set earns the word non-trivial: a cross-source overlap a
//! correct reconciliation merges into one slice, an authority
//! disagreement a correct synthesis surfaces as a `[divergence]` tag,
//! and an evidence gap a correct synthesis marks `[unknown]` rather
//! than inventing. The pass condition is structural — schema, coverage,
//! provenance, tags, lifecycle, build output — never a second model
//! judging the first. Per-leg repair counts are reported without being
//! asserted: a leg drifting from zero toward the repair budget is the
//! early warning that a prompt or answer-schema change degraded the
//! model's first answer.
//!
//! Run it through `cargo make test-live` (single trial, never CI) —
//! before a release tag and after judgment-prompt or answer-schema
//! changes. Requires cursor-agent on `PATH` with credentials
//! (`cursor-agent login` or `CURSOR_API_KEY`). The temporary project
//! is retained on failure for inspection.

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use artifacts::spec::provenance::{Requirement, RequirementStatus, parse_spec_md};
use omnia::Backend as _;
use omnia_guest::Model;
use omnia_guest::api::invocation::Invocation;
use omnia_guest::api::invoke::Invoker;
use omnia_guest::api::operation::Operation;
use omnia_guest::model::{Effort, Error, Format, Reply, Request, Role, Tool, Usage};
use omnia_testkit::model::Harness;
use omnia_wasi_model as wire;
use omnia_wasi_model::WasiModelCtx as _;
use serde_json::Value;
use workflow::change::plan;

#[path = "../../../crates/workflow/tests/common/mod.rs"]
mod common;

use common::answers;
use common::fixture::FixtureProvider;

/// The live provider: the same fixture seams as the scripted suites,
/// with the recording harness wrapped around the cursor backend.
type LiveProvider = FixtureProvider<Harness<CursorModel>>;

/// Invoke one operation against the live fixture provider.
async fn run<R, B>(
    invoker: &Invoker<LiveProvider>, input: R::Input,
) -> Result<B, workflow::handler::Error>
where
    R: Operation<LiveProvider, Output = B, Error = workflow::handler::Error>,
    B: Send,
{
    invoker.invoke::<R>(Invocation::new(input)).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "dispatches the configured live model; run via `cargo make test-live`"]
async fn adversarial_loop_is_validator_clean() {
    // Kept up front and removed only after every assertion passes, so
    // a failing trial leaves its project tree behind for inspection.
    let root = tempfile::TempDir::new().expect("tempdir").keep();
    let root = root.canonicalize().expect("canonical project root");
    eprintln!("live trial project (retained on failure): {}", root.display());
    let _cache = common::scoped_cache(&root);
    fs::create_dir_all(root.join(".specify")).expect("mkdir .specify");
    fs::write(root.join(".specify/project.yaml"), "name: live\nadapter: fixture\nrules: {}\n")
        .expect("write project.yaml");

    let model = CursorModel::connect(&root).await.expect(
        "cursor-agent backend unavailable: install cursor-agent, then `cursor-agent login` or \
         export CURSOR_API_KEY",
    );
    let invoker = Invoker::new("specify", FixtureProvider::new(&root, Harness::new(model)));

    // Live reconcile over the adversarial lead catalog. The author
    // orchestration's own kernel enforces coverage (every surveyed
    // lead assigned, none duplicated) inside the repair loop.
    run::<plan::handlers::Author, _>(
        &invoker,
        plan::handlers::AuthorInput {
            name: "auth".to_string(),
            sources: answers::adversarial_bindings(),
            intent: None,
        },
    )
    .await
    .expect("live reconcile authors a validator-clean plan");

    // The cross-source overlap merged: one slice carries the
    // `login-flow` lead from both sources.
    let plan = read_plan(&root);
    let merged = plan.entries.iter().any(|entry| {
        let pairs: Vec<(&str, &str)> = entry
            .sources
            .iter()
            .map(|b| (b.source.as_str(), b.lead.as_deref().unwrap_or(entry.name.as_str())))
            .collect();
        pairs.contains(&("docs", "login-flow")) && pairs.contains(&("code", "login-flow"))
    });
    assert!(merged, "the login-flow overlap must merge into one slice: {:?}", plan.entries);

    run::<plan::handlers::Transition, _>(
        &invoker,
        plan::handlers::TransitionInput {
            name: "auth".to_string(),
            target: Some("approved".to_string()),
            undo: false,
            actor: "operator".to_string(),
        },
    )
    .await
    .expect("the operator stamps Gate 1");

    // Live synthesis per slice, then build and merge. Draining means
    // every synthesis answer survived the deterministic schema,
    // model-vs-spec, and orphan-claim validators.
    let executed = run::<plan::handlers::Execute, _>(&invoker, plan::handlers::ExecuteInput {})
        .await
        .expect("live synthesis drains validator-clean");
    assert_eq!(executed.status, "drained");

    let plan = read_plan(&root);
    assert!(
        plan.entries.iter().all(|entry| entry.status == workflow::change::Status::Done),
        "{:?}",
        plan.entries
    );

    // The merged baseline, parsed with the same requirement-block
    // grammar the validators apply: complete provenance on every
    // evidenced requirement, the authority disagreement surfaced as a
    // status, and the evidence gap marked unknown instead of invented.
    let requirements = baseline_requirements(&root);
    assert!(!requirements.is_empty(), "the baseline carries no requirements");
    for requirement in &requirements {
        assert!(!requirement.id.is_empty(), "requirement `{}` carries no id", requirement.name);
        if requirement.status != Some(RequirementStatus::Unknown) {
            assert!(
                !requirement.sources.is_empty(),
                "evidenced requirement `{}` carries no provenance",
                requirement.name
            );
        }
    }
    assert!(
        requirements.iter().any(|requirement| matches!(
            requirement.status,
            Some(RequirementStatus::Divergence | RequirementStatus::Conflict)
        )),
        "the session-timeout disagreement must surface as a divergence or conflict: {requirements:?}"
    );
    assert!(
        requirements
            .iter()
            .any(|requirement| requirement.status == Some(RequirementStatus::Unknown)),
        "the password-reset gap must be marked unknown — the faithful answer records zero \
         contributing claims for the unevidenced lead (an answer that anchors it to the bare \
         `password-reset.mention` section claim projects `agreed` instead): {requirements:?}"
    );

    // Every slice produced a real, non-empty fixture build output.
    for entry in &plan.entries {
        let artifact = fixtures::build_artifact_path(&root, &entry.name);
        let body = fs::read_to_string(&artifact)
            .unwrap_or_else(|err| panic!("build output for `{}`: {err}", entry.name));
        assert!(!body.trim().is_empty(), "empty build output for `{}`", entry.name);
    }

    // The early-warning report: per-leg request counts, not asserted.
    // A leg drifting from zero repairs toward the budget means a
    // prompt or schema change degraded the model's first answer.
    let requests = invoker.provider().model().requests();
    let proposal = requests.iter().filter(|r| schema_name(r) == Some("proposal")).count();
    let synthesis = requests.iter().filter(|r| schema_name(r) == Some("synthesis")).count();
    let slices = plan.entries.len();
    eprintln!("reconcile leg: {proposal} request(s), {} repair(s)", proposal.saturating_sub(1));
    eprintln!(
        "synthesis legs: {synthesis} request(s) across {slices} slice(s), {} repair(s)",
        synthesis.saturating_sub(slices)
    );

    // Success: release the retained tree.
    fs::remove_dir_all(&root).expect("clean up the passing trial project");
}

/// The parsed `plan.yaml` at the trial root.
fn read_plan(root: &Path) -> workflow::change::Plan {
    serde_saphyr::from_str(&fs::read_to_string(root.join("plan.yaml")).expect("read plan.yaml"))
        .expect("parse plan.yaml")
}

/// Every requirement block across the merged baseline specs under
/// `.specify/specs/` (slice and domain names are model-chosen, so the
/// assertions scan the whole baseline).
fn baseline_requirements(root: &Path) -> Vec<Requirement> {
    let mut requirements = Vec::new();
    for domain in fs::read_dir(root.join(".specify/specs")).expect("baseline specs dir") {
        let spec = domain.expect("domain dir").path().join("spec.md");
        if spec.is_file() {
            let body = fs::read_to_string(&spec).expect("read baseline spec");
            requirements.extend(parse_spec_md(&body).requirements);
        }
    }
    requirements
}

/// The schema name a judgment request was gated on (`proposal` for the
/// reconcile leg, `synthesis` for each slice's synthesis leg).
const fn schema_name(request: &Request) -> Option<&str> {
    match &request.format {
        Format::Schema(schema) => Some(schema.name.as_str()),
        Format::Text | Format::Json => None,
    }
}

// --- the native cursor model ----------------------------------------------
//
// The same thin shim shape the shipped wasm path applies at the WIT
// boundary: map the guest `Request` onto the `omnia:model/completion`
// wire shape, translate `lend_workspace: true` into a `ToolHost`
// whose `local_path` is the project root (the only thing cursor-agent
// reads from it), and run the host's own answer gate before replying.
// `omnia_cursor::Client`'s spawn/repair/transcript machinery is reused
// as a library.
//
// This deliberately duplicates the `CursorModel` shim in
// `specify-adapters/harness/native/src/model.rs` (minus its `DevModel`
// selection and eval-model override): RFC-62's boundary forbids engine
// tests depending on the sibling checkout, and nothing mechanical
// catches drift between the copies — when the guest→wire mapping
// changes upstream, update both.

/// The cursor-agent-backed native [`Model`].
#[derive(Clone, Debug)]
struct CursorModel {
    client: omnia_cursor::Client,
    root: PathBuf,
}

impl CursorModel {
    /// Connect cursor-agent (asserting it is on `PATH`) rooted at the
    /// project directory the workspace lend resolves to.
    async fn connect(root: impl Into<PathBuf>) -> Result<Self> {
        let client = omnia_cursor::Client::connect().await?;
        Ok(Self {
            client,
            root: root.into(),
        })
    }
}

impl Model for CursorModel {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        // The lend translation: in-guest the `"."` preopen resolves the
        // lent workspace; natively the project root plays that part.
        let workspace = request.lend_workspace.then(|| self.root.clone());
        let format = wire_format(&request.format);
        let wire = wire_request(request);

        let answer = self
            .client
            .complete(wire, Arc::new(LocalToolHost { workspace }))
            .await
            .map_err(|err| Error::Backend(err.to_string()))?;

        // The same answer gate the host runs after its backends.
        format.check(&answer.value).map_err(Error::InvalidAnswer)?;
        reply(answer)
    }
}

/// Guest [`Request`] → the `omnia:model/completion` wire request — the
/// mapping the wasm default body performs at the WIT boundary. The
/// lent workspace never crosses (`grants.workspace` is host plumbing;
/// cursor resolves the tree through the tool host instead).
fn wire_request(request: Request) -> wire::Request {
    wire::Request {
        model: request.model,
        system: request.system,
        messages: request
            .messages
            .into_iter()
            .map(|message| wire::Message {
                role: match message.role {
                    Role::System => wire::Role::System,
                    Role::User => wire::Role::User,
                    Role::Assistant => wire::Role::Assistant,
                },
                content: message.content,
            })
            .collect(),
        generation: request.generation.map(|generation| wire::Generation {
            temperature: generation.temperature,
            top_p: generation.top_p,
            max_tokens: generation.max_tokens,
            stop: generation.stop,
            seed: generation.seed,
            effort: generation.effort.map(|effort| match effort {
                Effort::Minimal => wire::Effort::Minimal,
                Effort::Low => wire::Effort::Low,
                Effort::Medium => wire::Effort::Medium,
                Effort::High => wire::Effort::High,
            }),
        }),
        format: wire_format(&request.format),
        tools: request
            .tools
            .into_iter()
            .map(|tool| match tool {
                Tool::Function(function) => wire::Tool::Function(wire::Function {
                    name: function.name,
                    description: function.description,
                    parameters: function.parameters,
                }),
                Tool::Mcp(grant) => wire::Tool::Mcp(wire::Mcp {
                    name: grant.name,
                    tools: grant.tools,
                    url: grant.url,
                }),
            })
            .collect(),
        grants: wire::Grants {
            references: request.references,
            workspace: None,
            verify: request.verify,
        },
    }
}

/// Guest [`Format`] → the wire format.
fn wire_format(format: &Format) -> wire::Format {
    match format {
        Format::Text => wire::Format::Text,
        Format::Json => wire::Format::Json,
        Format::Schema(schema) => wire::Format::Schema(wire::Schema {
            name: schema.name.clone(),
            schema: schema.schema.clone(),
        }),
    }
}

/// A backend [`wire::Answer`] → the guest [`Reply`]: `text` answers
/// are plain text, JSON formats carry the serialized document — the
/// host gate's own projection.
fn reply(answer: wire::Answer) -> Result<Reply, Error> {
    let text = match answer.value {
        Value::String(text) => text,
        value => serde_json::to_string(&value)
            .map_err(|err| Error::InvalidAnswer(format!("answer is not serializable: {err}")))?,
    };
    Ok(Reply {
        answer: text,
        usage: answer.usage.map(|usage| Usage {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            reasoning_tokens: usage.reasoning_tokens,
        }),
    })
}

/// The minimal per-completion tool host the cursor backend reads: only
/// `local_path` matters (cursor-agent does its own filesystem work);
/// the bounded-capability methods are never called on this backend.
struct LocalToolHost {
    workspace: Option<PathBuf>,
}

impl wire::ToolHost for LocalToolHost {
    fn resolve(&self, _reference: wire::Reference) -> wire::FutureResult<Vec<u8>> {
        Box::pin(async { Err(anyhow::anyhow!("the native tool host serves no references")) })
    }

    fn read(&self, _path: String) -> wire::FutureResult<Vec<u8>> {
        Box::pin(async { Err(anyhow::anyhow!("the native tool host serves no reads")) })
    }

    fn list(&self, _path: String) -> wire::FutureResult<Vec<wire::DirEntry>> {
        Box::pin(async { Err(anyhow::anyhow!("the native tool host serves no listings")) })
    }

    fn write(&self, _path: String, _bytes: Vec<u8>) -> wire::FutureResult<()> {
        Box::pin(async { Err(anyhow::anyhow!("the native tool host serves no writes")) })
    }

    fn verify(&self, _check: String) -> wire::FutureResult<wire::VerifyReport> {
        Box::pin(async { Err(anyhow::anyhow!("the native tool host serves no verification")) })
    }

    fn local_path(&self) -> Option<&Path> {
        self.workspace.as_deref()
    }
}
