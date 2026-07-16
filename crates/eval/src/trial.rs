//! The live-model trial body: the operator rhythm over the fixture
//! adversarial lead set, graded by deterministic validators only.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use change::plan::handlers::{
    Archive, ArchiveInput, Author, AuthorInput, Execute, ExecuteInput, Transition, TransitionInput,
};
use change::{Entry, Plan, Status};
use clap::Subcommand;
use project::config::Layout;
use project::init::handlers::{Init, InitInput};
use testkit::{Provider, Scripted, answers};

use crate::grade;
use crate::model::DevModel;
use crate::telemetry::Telemetry;

// The live model: cursor backend behind the request tally — 
// deterministic steps never demand cursor-agent.
type EvalProvider = Provider<Telemetry<DevModel>>;

const CHANGE: &str = "auth";

/// One operation in the persistent manual evaluation workflow.
#[derive(Clone, Copy, Debug, Subcommand)]
pub enum Phase {
    Init,
    Plan,
    Execute,
    Finalize,
    Clean,
}

/// One full trial: init → plan → execute → finalize → clean.
pub async fn run() {
    for phase in [Phase::Init, Phase::Plan, Phase::Execute, Phase::Finalize, Phase::Clean] {
        phase.run().await;
    }
}

impl Phase {
    /// Run one operation against the persistent `sandbox/eval` project.
    pub async fn run(self) {
        match self {
            Self::Init => init().await,
            Self::Plan => plan().await,
            Self::Execute => execute().await,
            Self::Finalize => finalize().await,
            Self::Clean => clean(),
        }
    }
}

async fn init() {
    let root = replace();
    println!("prompt evaluation project: {}", root.display());
    let _cache = testkit::env::scoped_cache(&root);
    let provider = Scripted::scripted_at(&root, Vec::new());

    testkit::run::<Init, _, _>(
        &provider,
        InitInput {
            adapter: Some("fixture".to_string()),
            name: Some("eval".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("init scaffolds the fixture-bound project");
}

async fn plan() {
    let root = require();
    println!("prompt evaluation project: {}", root.display());
    let _cache = testkit::env::scoped_cache(&root);
    let provider = live(&root);

    author(&provider, &root).await;
    approve(&provider).await;

    let plan = read_plan(&root);
    report(&provider.model().counts(), plan.entries.len());
}

async fn execute() {
    let root = require();
    println!("prompt evaluation project: {}", root.display());
    let _cache = testkit::env::scoped_cache(&root);
    let provider = live(&root);

    let executed = testkit::run::<Execute, _, _>(&provider, ExecuteInput {})
        .await
        .expect("execute drains the plan");
    for phase in &executed.phases {
        eprintln!("executed {} {}", phase.step, phase.slice);
    }
    assert_eq!(executed.status, "drained", "execute must exit drained");

    let plan = read_plan(&root);
    assert!(
        plan.entries.iter().all(|entry| entry.status == Status::Done),
        "execute must leave every entry done: {:?}",
        plan.entries
    );

    grade::run(&root, &plan);
    report(&provider.model().counts(), plan.entries.len());
}

async fn finalize() {
    let root = require();
    println!("prompt evaluation project: {}", root.display());
    let _cache = testkit::env::scoped_cache(&root);
    let provider = Scripted::scripted_at(&root, Vec::new());

    testkit::run::<Archive, _, _>(&provider, ArchiveInput { force: false })
        .await
        .expect("finalize archives the drained plan");
}

fn clean() {
    let root = root();
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean up the evaluation project");
    }
}

// `specify plan author` — live reconcile over the adversarial lead
// catalog: every surveyed lead assigned, `login-flow` overlap merged
// into one slice.
async fn author(provider: &EvalProvider, root: &Path) {
    testkit::run::<Author, _, _>(
        provider,
        AuthorInput {
            name: CHANGE.to_string(),
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
    testkit::run::<Transition, _, _>(
        provider,
        TransitionInput {
            name: CHANGE.to_string(),
            target: Some("approved".to_string()),
            undo: false,
            actor: "operator".to_string(),
        },
    )
    .await
    .expect("Gate 1: operator stamps approved");
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sandbox/eval")
}

fn replace() -> PathBuf {
    let root = root();
    if root.exists() {
        fs::remove_dir_all(&root).expect("replace the previous project");
    }
    fs::create_dir_all(&root).expect("create the project root");
    root.canonicalize().expect("canonical project root")
}

fn require() -> PathBuf {
    let root = root();
    assert!(
        root.join(".specify/project.yaml").is_file(),
        "project is not initialised; run `cargo make eval init` first"
    );
    root.canonicalize().expect("canonical project root")
}

// The live provider: cursor-agent connects lazily on the first
// judgment leg (`DevModel`), so building the provider is free.
fn live(root: &Path) -> EvalProvider {
    Provider::new(root, Telemetry::new(DevModel::new(root)))
}

fn read_plan(root: &Path) -> Plan {
    Plan::load(&Layout::new(root).plan_path()).expect("load plan.yaml")
}

fn binds(entry: &Entry, source: &str, lead: &str) -> bool {
    entry
        .sources
        .iter()
        .any(|b| b.source == source && b.lead.as_deref().unwrap_or(entry.name.as_str()) == lead)
}

// Report per-leg request counts.
//
// Requests beyond one per leg invocation are repairs — the early signal that a
// prompt, or answer-schema change degraded the model's first answer.
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
