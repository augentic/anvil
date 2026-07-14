//! Prompt-evaluation example: one live-model trial that drives the
//! Specify engine the same way an operator does.
//!
//! ```text
//! init        scaffold the fixture-bound project
//! plan        author the change, stamp Gate 1 (`approved`)
//! execute     drain the loop: refine → build → merge per slice
//! finalize    archive the drained plan
//! ```
//!
//! Graded by deterministic validators only (see [README.md](README.md)).
//! Run `cargo make prompt-eval` (never CI). Needs `cursor-agent` on
//! `PATH` with credentials. The temporary project is retained on failure.

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use artifacts::spec::provenance::{Requirement, RequirementStatus, parse_spec_md};
use change::{Status, plan};
use omnia::Backend as _;
use testkit::{Provider, answers, run};

use crate::native::Native;

mod native;

/// The live model: cursor-agent behind the example-local [`Native`]
/// adapter, which carries the guest→wire mapping, the request/answer
/// gates, and the workspace lend natively.
type EvalProvider = Provider<Native<omnia_cursor::Client>>;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let (root, _cache) = scaffold();
    let provider = connect(&root).await;

    // The operator rhythm: init → plan → execute → finalize.
    init(&provider).await;
    plan(&provider, &root).await;
    let drained = execute(&provider, &root).await;
    grade(&root, &drained);
    finalize(&provider).await;

    fs::remove_dir_all(&root).expect("clean up the passing trial project");
}

/// A bare temp tree with the adapter cache pinned inside it; retained
/// until success cleans it up. The project scaffold itself comes from
/// the `Init` operation.
fn scaffold() -> (PathBuf, testkit::env::CacheGuard) {
    let root = tempfile::TempDir::new().expect("tempdir").keep();
    let root = root.canonicalize().expect("canonical project root");
    eprintln!("prompt evaluation project (retained on failure): {}", root.display());
    let cache = testkit::env::scoped_cache(&root);
    (root, cache)
}

async fn connect(root: &Path) -> EvalProvider {
    let client = omnia_cursor::Client::connect().await.expect(
        "cursor-agent backend unavailable: install cursor-agent, then `cursor-agent login` or \
         export CURSOR_API_KEY",
    );
    // In-guest the `"."` preopen resolves the lent workspace; natively
    // the trial project root plays that part.
    Provider::new(root, Native::new(client, root))
}

/// `specify init fixture` — scaffold the fixture-bound project through
/// the real operation.
async fn init(provider: &EvalProvider) {
    run::<project::init::handlers::Init, _, _>(
        provider,
        project::init::handlers::InitInput {
            adapter: Some("fixture".to_string()),
            name: Some("eval".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("init scaffolds the fixture-bound project");
}

// --- plan -------------------------------------------------------------------

/// `specify plan author` + Gate 1 `approved` stamp.
///
/// Live reconcile over the adversarial lead catalog: every surveyed lead
/// assigned, `login-flow` overlap merged into one slice.
async fn plan(provider: &EvalProvider, root: &Path) {
    run::<plan::handlers::Author, _, _>(
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
    let merged = authored.entries.iter().any(|entry| {
        let pairs: Vec<(&str, &str)> = entry
            .sources
            .iter()
            .map(|b| (b.source.as_str(), b.lead.as_deref().unwrap_or(entry.name.as_str())))
            .collect();
        pairs.contains(&("docs", "login-flow")) && pairs.contains(&("code", "login-flow"))
    });
    assert!(merged, "the login-flow overlap must merge into one slice: {:?}", authored.entries);

    run::<plan::handlers::Transition, _, _>(
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

// --- execute ----------------------------------------------------------------

/// `specify plan execute` — the production drained loop: refine →
/// build → merge per entry until the plan is drained.
async fn execute(provider: &EvalProvider, root: &Path) -> change::Plan {
    let executed = run::<plan::handlers::Execute, _, _>(provider, plan::handlers::ExecuteInput {})
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

// --- finalize ---------------------------------------------------------------

/// `specify plan archive` — close the drained change (`/spec:finalize`).
async fn finalize(provider: &EvalProvider) {
    run::<plan::handlers::Archive, _, _>(provider, plan::handlers::ArchiveInput { force: false })
        .await
        .expect("finalize archives the drained plan");
}

// --- grade ------------------------------------------------------------------

/// Structural checks after execute, before finalize (plan.yaml still live).
fn grade(root: &Path, plan: &change::Plan) {
    assert_baseline(root);
    assert_build_outputs(root, plan);
}

fn assert_baseline(root: &Path) {
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
}

fn assert_build_outputs(root: &Path, plan: &change::Plan) {
    for entry in &plan.entries {
        let artifact = testkit::adapter::build_artifact_path(root, &entry.name);
        let body = fs::read_to_string(&artifact)
            .unwrap_or_else(|err| panic!("build output for `{}`: {err}", entry.name));
        assert!(!body.trim().is_empty(), "empty build output for `{}`", entry.name);
    }
}

fn read_plan(root: &Path) -> change::Plan {
    serde_saphyr::from_str(&fs::read_to_string(root.join("plan.yaml")).expect("read plan.yaml"))
        .expect("parse plan.yaml")
}

fn requirements(root: &Path) -> Vec<Requirement> {
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
