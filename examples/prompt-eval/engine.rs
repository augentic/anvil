//! Prompt-evaluation example: one live-model trial that drives the
//! Specify engine the same way an operator does.
//!
//! ```text
//! plan        author the change, stamp Gate 1 (`approved`)
//! execute     per slice: plan next → refine → build → merge
//! finalize    archive the drained plan
//! ```
//!
//! Graded by deterministic validators only (see [README.md](README.md)).
//! Run `cargo make prompt-eval` (never CI). Needs `cursor-agent` on
//! `PATH` with credentials. The temporary project is retained on failure.

#![cfg(not(target_arch = "wasm32"))]

mod cursor;

use std::fs;
use std::path::{Path, PathBuf};

use artifacts::spec::provenance::{Requirement, RequirementStatus, parse_spec_md};
use change::plan;
use change::{NextReason, Status};
use cursor::CursorModel;
use omnia_guest::model::{Format, Request};
use omnia_testkit::model::Harness;
use slice::handlers::{Build, BuildInput, MergeRun, MergeRunInput, Refine, RefineInput};
use testkit::{Provider, answers, run};

type EvalProvider = Provider<Harness<CursorModel>>;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let (root, _cache) = scaffold();
    let provider = connect(&root).await;

    // The operator rhythm: plan → execute → finalize.
    plan(&provider, &root).await;
    let drained = execute(&provider, &root).await;
    grade(&provider, &root, &drained);
    finalize(&provider).await;

    fs::remove_dir_all(&root).expect("clean up the passing trial project");
}

/// Scaffold a temp fixture project; retained until success cleans it up.
fn scaffold() -> (PathBuf, testkit::env::CacheGuard) {
    let root = tempfile::TempDir::new().expect("tempdir").keep();
    let root = root.canonicalize().expect("canonical project root");
    eprintln!("prompt evaluation project (retained on failure): {}", root.display());
    let cache = testkit::env::scoped_cache(&root);
    fs::create_dir_all(root.join(".specify")).expect("mkdir .specify");
    fs::write(root.join(".specify/project.yaml"), "name: eval\nadapter: fixture\nrules: {}\n")
        .expect("write project.yaml");
    (root, cache)
}

async fn connect(root: &Path) -> EvalProvider {
    let model = CursorModel::connect(root).await.expect(
        "cursor-agent backend unavailable: install cursor-agent, then `cursor-agent login` or \
         export CURSOR_API_KEY",
    );
    Provider::new(root, Harness::new(model))
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
    assert!(
        merged,
        "the login-flow overlap must merge into one slice: {:?}",
        authored.entries
    );

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

/// Hand-driven execute loop: claim each slice, then refine → build → merge.
///
/// This is the operator fallback (`plan next` + breakouts). Production
/// `specify plan execute` drains the same phases automatically.
async fn execute(provider: &EvalProvider, root: &Path) -> change::Plan {
    while let Some(slice) = next_slice(provider).await {
        eprintln!("execute slice `{slice}`: refine → build → merge");
        refine(provider, &slice).await;
        build(provider, &slice).await;
        merge(provider, &slice).await;
    }

    let plan = read_plan(root);
    assert!(
        plan.entries.iter().all(|entry| entry.status == Status::Done),
        "execute must leave every entry done: {:?}",
        plan.entries
    );
    plan
}

/// `specify plan next` — sole writer of per-entry `in-progress`.
/// Returns `None` when the plan is drained.
async fn next_slice(provider: &EvalProvider) -> Option<String> {
    let body = run::<plan::handlers::Next, _, _>(provider, plan::handlers::NextInput {})
        .await
        .expect("plan next");
    let reason = body.reason;
    if let Some(slice) = body.next.or(body.active) {
        return Some(slice);
    }
    assert_eq!(
        reason,
        Some(NextReason::Drained),
        "plan next with no entry must report drained (reason={reason:?})"
    );
    None
}

/// `specify slice refine <slice>` — extract + synthesize to `refined`.
async fn refine(provider: &EvalProvider, slice: &str) {
    run::<Refine, _, _>(
        provider,
        RefineInput {
            name: slice.to_string(),
        },
    )
    .await
    .unwrap_or_else(|err| panic!("refine `{slice}`: {err}"));
}

/// `specify slice build <slice>` — target build to `built`.
async fn build(provider: &EvalProvider, slice: &str) {
    run::<Build, _, _>(
        provider,
        BuildInput {
            name: slice.to_string(),
        },
    )
    .await
    .unwrap_or_else(|err| panic!("build `{slice}`: {err}"));
}

/// `specify slice merge run <slice>` — merge into baseline; stamps entry `done`.
async fn merge(provider: &EvalProvider, slice: &str) {
    run::<MergeRun, _, _>(
        provider,
        MergeRunInput {
            name: slice.to_string(),
            allow_composition_replace: false,
        },
    )
    .await
    .unwrap_or_else(|err| panic!("merge `{slice}`: {err}"));
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
fn grade(provider: &EvalProvider, root: &Path, plan: &change::Plan) {
    assert_baseline(root);
    assert_build_outputs(root, plan);
    report_repairs(provider, plan.entries.len());
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

/// Per-leg request counts — reported, not asserted (early prompt-drift signal).
fn report_repairs(provider: &EvalProvider, slices: usize) {
    let requests = provider.model().requests();
    let proposal = requests.iter().filter(|r| schema_name(r) == Some("proposal")).count();
    let synthesis = requests.iter().filter(|r| schema_name(r) == Some("synthesis")).count();
    eprintln!("reconcile leg: {proposal} request(s), {} repair(s)", proposal.saturating_sub(1));
    eprintln!(
        "synthesis legs: {synthesis} request(s) across {slices} slice(s), {} repair(s)",
        synthesis.saturating_sub(slices)
    );
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

const fn schema_name(request: &Request) -> Option<&str> {
    match &request.format {
        Format::Schema(schema) => Some(schema.name.as_str()),
        Format::Text | Format::Json => None,
    }
}
