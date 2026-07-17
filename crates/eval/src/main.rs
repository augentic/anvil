//! `eval` — live-model trial over the fixture catalog: the shared
//! harness entrypoint with the adversarial trial profile and
//! deterministic fixture grading.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, ensure};
use artifacts::spec::provenance::{Requirement, RequirementStatus};
use change::plan::handlers::ExecuteBody;
use change::{Entry, Plan};
use eval::{Adapters, SANDBOX};
use fixture::behaviour::build_artifact_path;
use harness::trial::{self, Profile};

const CHANGE: &str = "auth";

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    report(trial::run::<Adapters>(&profile(), &argv).await)
}

fn report(outcome: Result<ExitCode>) -> ExitCode {
    outcome.unwrap_or_else(|err| {
        eprintln!("eval: {err:#}");
        ExitCode::FAILURE
    })
}

// The adversarial two-source pair: a docs source and a code source,
// both served by the fixture core under different adapter names.
fn profile() -> Profile {
    Profile {
        sandbox: SANDBOX.into(),
        seed: None,
        init: argv(&["init", "fixture", "--name", "eval"]),
        author: argv(&[
            "plan",
            "author",
            CHANGE,
            "--source",
            "docs=fixture-docs:value:The docs source.",
            "--source",
            "code=fixture-code:value:The code source.",
        ]),
        change: CHANGE.to_string(),
        authored: Some(authored),
        grade,
        scenarios: None,
    }
}

// `specify plan author` — live reconcile over the adversarial lead
// catalog: every surveyed lead assigned, `login-flow` overlap merged
// into one slice.
fn authored(_root: &Path, plan: &Plan) -> Result<()> {
    ensure!(
        plan.entries
            .iter()
            .any(|entry| binds(entry, "docs", "login-flow") && binds(entry, "code", "login-flow")),
        "the login-flow overlap must merge into one slice: {:?}",
        plan.entries
    );
    Ok(())
}

fn binds(entry: &Entry, source: &str, lead: &str) -> bool {
    entry
        .sources
        .iter()
        .any(|b| b.source == source && b.lead.as_deref().unwrap_or(entry.name.as_str()) == lead)
}

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(ToString::to_string).collect()
}

// Grade the drained plan against the adversarial fixture contract:
// one failing assertion at a time, with the evidence inline.
fn grade(root: &Path, plan: &Plan, _executed: &ExecuteBody) -> Result<()> {
    let requirements = harness::grade::baseline(root)?;
    harness::grade::provenance(&requirements)?;
    adversarial(&requirements)?;
    outputs(root, plan)?;
    Ok(())
}

fn adversarial(requirements: &[Requirement]) -> Result<()> {
    ensure!(
        requirements.iter().any(|requirement| matches!(
            requirement.status,
            Some(RequirementStatus::Divergence | RequirementStatus::Conflict)
        )),
        "the session-timeout disagreement must surface as a divergence or conflict: \
         {requirements:?}"
    );
    ensure!(
        requirements
            .iter()
            .any(|requirement| requirement.status == Some(RequirementStatus::Unknown)),
        "the password-reset gap must be marked unknown — the faithful answer records zero \
         contributing claims for the unevidenced lead (an answer that anchors it to the bare \
         `password-reset.mention` section claim projects `agreed` instead): {requirements:?}"
    );
    Ok(())
}

fn outputs(root: &Path, plan: &Plan) -> Result<()> {
    for entry in &plan.entries {
        let artifact = build_artifact_path(root, &entry.name);
        let body = fs::read_to_string(&artifact)
            .with_context(|| format!("build output for `{}`", entry.name))?;
        ensure!(!body.trim().is_empty(), "empty build output for `{}`", entry.name);
    }
    Ok(())
}
