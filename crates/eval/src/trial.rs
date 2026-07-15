//! The live-model trial body: the operator rhythm over the fixture
//! adversarial lead set, graded by deterministic validators only.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use artifacts::spec::provenance::{Requirement, RequirementStatus, parse_spec_md};
use change::{Entry, Status, plan};
use clap::Subcommand;
use omnia::Backend as _;
use project::config::Layout;
use testkit::{Provider, Scripted, answers, run as invoke};

use crate::native::Native;
use crate::telemetry::Telemetry;

/// The live model: cursor-agent behind the harness-local [`Native`]
/// adapter (the guest→wire mapping, the request/answer gates, and the
/// workspace lend), wrapped in the per-leg request tally.
type EvalProvider = Provider<Telemetry<Native<omnia_cursor::Client>>>;

/// One operation in the persistent manual evaluation workflow.
#[derive(Clone, Copy, Debug, Subcommand)]
pub enum Phase {
    /// Initialise a fresh evaluation project.
    Init,
    /// Author the plan and stamp Gate 1 approved.
    Plan,
    /// Execute and grade the approved plan.
    Execute,
    /// Archive the drained plan.
    Finalize,
    /// Remove the persistent evaluation project.
    Clean,
}

/// One full trial: init → plan → execute → finalize → clean. Runs in
/// `sandbox/eval/`; a passing trial removes the project, a failing one
/// retains it for the manual operations to inspect and re-drive.
pub async fn run() {
    Phase::Init.run().await;
    Phase::Plan.run().await;
    Phase::Execute.run().await;
    Phase::Finalize.run().await;
    Phase::Clean.run().await;
}

impl Phase {
    /// Run one operation against the persistent `sandbox/eval` project.
    pub async fn run(&self) {
        match self {
            Phase::Clean => {
                let root = project_root();
                if root.exists() {
                    fs::remove_dir_all(&root).expect("clean up the evaluation project");
                }
            }
            Phase::Init => {
                let root = replace_project();
                println!("prompt evaluation project: {}", root.display());
                let _cache = testkit::env::scoped_cache(&root);
                init(&root).await;
            }
            Phase::Plan => {
                let root = require_project();
                println!("prompt evaluation project: {}", root.display());
                let _cache = testkit::env::scoped_cache(&root);
                let provider = connect(&root).await;
                author(&provider, &root).await;
                approve(&provider).await;
                let plan = read_plan(&root);
                report(&provider.model().counts(), plan.entries.len());
            }
            Phase::Execute => {
                let root = require_project();
                println!("prompt evaluation project: {}", root.display());
                let _cache = testkit::env::scoped_cache(&root);
                let provider = connect(&root).await;
                let drained = execute(&provider, &root).await;
                grade(&root, &drained);
                report(&provider.model().counts(), drained.entries.len());
            }
            Phase::Finalize => {
                let root = require_project();
                println!("prompt evaluation project: {}", root.display());
                let _cache = testkit::env::scoped_cache(&root);
                finalize(&root).await;
            }
        }
    }
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sandbox/eval")
}

fn replace_project() -> PathBuf {
    let root = project_root();
    if root.exists() {
        fs::remove_dir_all(&root).expect("replace the previous evaluation project");
    }
    fs::create_dir_all(&root).expect("create the evaluation project root");
    root.canonicalize().expect("canonical evaluation project root")
}

fn require_project() -> PathBuf {
    let root = project_root();
    assert!(
        root.join(".specify/project.yaml").is_file(),
        "evaluation project is not initialised; run `cargo make eval init` first"
    );
    root.canonicalize().expect("canonical evaluation project root")
}

async fn connect(root: &Path) -> EvalProvider {
    let client = omnia_cursor::Client::connect().await.expect(
        "cursor-agent backend unavailable: install cursor-agent, then `cursor-agent login` or \
         export CURSOR_API_KEY",
    );
    // In-guest the `"."` preopen resolves the lent workspace; natively
    // the trial project root plays that part.
    Provider::new(root, Telemetry::new(Native::new(client, root)))
}

async fn init(root: &Path) {
    invoke::<project::init::handlers::Init, _, _>(
        &Scripted::scripted_at(root, Vec::new()),
        project::init::handlers::InitInput {
            adapter: Some("fixture".to_string()),
            name: Some("eval".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("init scaffolds the fixture-bound project");
}

// `specify plan author` — live reconcile over the adversarial lead
// catalog: every surveyed lead assigned, `login-flow` overlap merged
// into one slice.
async fn author(provider: &EvalProvider, root: &Path) {
    invoke::<plan::handlers::Author, _, _>(
        provider,
        plan::handlers::AuthorInput {
            name: "auth".to_string(),
            sources: answers::adversarial_bindings(),
            intent: None,
        },
    )
    .await
    .expect("plan author produces a validator-clean plan");

    let authored = read_plan(root);
    assert!(
        authored.entries.iter().any(|entry| {
            binds(entry, "docs", "login-flow") && binds(entry, "code", "login-flow")
        }),
        "the login-flow overlap must merge into one slice: {:?}",
        authored.entries
    );
}

// Gate 1: operator stamps `approved`.
async fn approve(provider: &EvalProvider) {
    invoke::<plan::handlers::Transition, _, _>(
        provider,
        plan::handlers::TransitionInput {
            name: "auth".to_string(),
            target: Some("approved".to_string()),
            undo: false,
            actor: "operator".to_string(),
        },
    )
    .await
    .expect("Gate 1: operator stamps approved");
}

fn binds(entry: &Entry, source: &str, lead: &str) -> bool {
    entry
        .sources
        .iter()
        .any(|b| b.source == source && b.lead.as_deref().unwrap_or(entry.name.as_str()) == lead)
}

// `specify plan execute` — the production drained loop: refine →
// build → merge per entry until the plan is drained.
async fn execute(provider: &EvalProvider, root: &Path) -> change::Plan {
    let executed =
        invoke::<plan::handlers::Execute, _, _>(provider, plan::handlers::ExecuteInput {})
            .await
            .expect("execute drains the plan");
    for phase in &executed.phases {
        eprintln!("executed {} {}", phase.step, phase.slice);
    }
    assert_eq!(executed.status, "drained", "execute must exit drained");

    let plan = read_plan(root);
    assert!(
        plan.entries.iter().all(|entry| entry.status == Status::Done),
        "execute must leave every entry done: {:?}",
        plan.entries
    );
    plan
}

async fn finalize(root: &Path) {
    invoke::<plan::handlers::Archive, _, _>(
        &Scripted::scripted_at(root, Vec::new()),
        plan::handlers::ArchiveInput { force: false },
    )
    .await
    .expect("finalize archives the drained plan");
}

// Structural checks after execute, before finalize (plan.yaml still live).
fn grade(root: &Path, plan: &change::Plan) {
    let requirements = requirements(root);
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

    for entry in &plan.entries {
        let artifact = testkit::adapter::build_artifact_path(root, &entry.name);
        let body = fs::read_to_string(&artifact)
            .unwrap_or_else(|err| panic!("build output for `{}`: {err}", entry.name));
        assert!(!body.trim().is_empty(), "empty build output for `{}`", entry.name);
    }
}

// Per-leg request counts, reported (never asserted): requests beyond
// one per leg invocation are repairs — the early signal that a prompt
// or answer-schema change degraded the model's first answer.
fn report(counts: &BTreeMap<String, usize>, slices: usize) {
    for (leg, requests) in counts {
        // One propose invocation per trial, one synthesis invocation
        // per plan entry; other legs carry no invocation baseline.
        match leg.as_str() {
            "proposal" => {
                let repairs = requests.saturating_sub(1);
                eprintln!("leg proposal: {requests} request(s), {repairs} repair(s)");
            }
            "synthesis" => {
                let repairs = requests.saturating_sub(slices);
                eprintln!(
                    "leg synthesis: {requests} request(s) over {slices} slice(s), \
                     {repairs} repair(s)"
                );
            }
            other => eprintln!("leg {other}: {requests} request(s)"),
        }
    }
}

fn read_plan(root: &Path) -> change::Plan {
    change::Plan::load(&Layout::new(root).plan_path()).expect("load plan.yaml")
}

fn requirements(root: &Path) -> Vec<Requirement> {
    let mut requirements = Vec::new();
    for domain in fs::read_dir(Layout::new(root).specs_dir()).expect("baseline specs dir") {
        let spec = domain.expect("domain dir").path().join("spec.md");
        if spec.is_file() {
            let body = fs::read_to_string(&spec).expect("read baseline spec");
            requirements.extend(parse_spec_md(&body).requirements);
        }
    }
    requirements
}
