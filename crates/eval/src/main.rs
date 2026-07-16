//! # Prompt-evaluation harness
//!
//! A live-model trial that drives the Specify engine the same way an operator does.
//!
//! ```text
//! init        scaffold the fixture-bound project
//! plan        author the change, stamp Gate 1 (`approved`)
//! execute     drain the loop: refine → build → merge per slice
//! finalize    archive the drained plan
//! ```
//!
//! The crate is a declarative binding over the shared harness: it
//! links the testkit fixture catalog and supplies the adversarial
//! trial profile plus deterministic fixture assertions; phase
//! sequencing, provider construction, sandbox lifecycle, and telemetry
//! reporting live in `harness`. See [README.md](../README.md).

mod grade;

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Result, ensure};
use change::{Entry, Plan};
use harness::catalog::{Binding, Catalog};
use harness::trial::{self, Profile};
use omnia_guest::Model;
use testkit::fixture::{Fixture, FixtureCode, FixtureDocs};

/// The Specify-owned fixture adapters linked into the engine trial.
struct Fixtures;

impl Binding for Fixtures {
    fn catalog<M: Model>() -> Catalog<M> {
        Catalog::builder()
            .source::<Fixture>()
            .source::<FixtureDocs>()
            .source::<FixtureCode>()
            .target::<Fixture>()
            .build()
    }
}

const CHANGE: &str = "auth";

/// The sandbox root for the trial project.
const SANDBOX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../sandbox");

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

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match trial::run::<Fixtures>(&profile(), &args).await {
        Ok(code) => code,
        Err(err) => {
            eprintln!("eval: {err:#}");
            ExitCode::FAILURE
        }
    }
}
