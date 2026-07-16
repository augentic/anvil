//! `eval` — live-model trial over the fixture catalog: the shared
//! harness entrypoint with the adversarial trial profile and
//! deterministic fixture grading.

mod grade;

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Result, ensure};
use change::{Entry, Plan};
use eval::{Fixtures, SANDBOX};
use harness::trial::{self, Profile};

const CHANGE: &str = "auth";

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    report(trial::run::<Fixtures>(&profile(), &argv).await)
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
        grade: grade::run,
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
