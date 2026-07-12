//! The composed live loop: the canonical `guest-execute-loop`
//! scenario's `wasm-live` profile driven once through the in-process
//! composed executor over the release-built sibling adapter
//! components, then graded with the scenario's hard assertions.
//!
//! `#[ignore]`: requires an authenticated cursor-agent on `PATH`, the
//! sibling checkout release-built (`cargo make release` there), the
//! workflow guest built (`cargo build -p specify --lib --target
//! wasm32-wasip2`), and makes real model calls. The quality binary
//! owns repeated trials and semantic grading.

use std::path::PathBuf;
use std::{env, fs};

use quality::executor::{ComposedExecutor, Executor as _};
use quality::{manifest, verify};
use scenario::grade::Evaluators;
use scenario::{AssertionId, Grading, ModelBackend, Outcome, Runtime, catalog, evaluate, grade};

fn cursor_agent_on_path() -> bool {
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| directory.join("cursor-agent").is_file())
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live: needs authenticated cursor-agent and release-built adapters; run with -- --ignored"]
async fn wasm_live_trial_passes() {
    let scenario = catalog::load("guest-execute-loop").expect("canonical scenario");
    let profile = scenario
        .profiles
        .iter()
        .find(|profile| profile.id == "wasm-live")
        .expect("wasm-live profile");
    assert_eq!(profile.runtime, Runtime::Wasm);
    assert_eq!(profile.model, ModelBackend::Live);
    assert_eq!(profile.grading, Grading::Semantic);

    assert!(
        cursor_agent_on_path(),
        "cursor-agent not found on PATH; install it, then `cursor-agent login` or export \
         CURSOR_API_KEY"
    );

    let framework = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let adapters = env::var_os("SPECIFY_ADAPTERS")
        .map_or_else(|| framework.join("../specify-adapters"), PathBuf::from);
    let release = adapters.join("target/wasm32-wasip2/release");
    assert!(
        release.join("omnia.wasm").is_file(),
        "release-built adapter components not found at {}; run `cargo make release` in the \
         sibling specify-adapters checkout",
        release.display()
    );
    let workflow = framework.join("target/wasm32-wasip2/debug/specify.wasm");
    assert!(
        workflow.is_file(),
        "workflow guest not found at {}; run `cargo build -p specify --lib --target \
         wasm32-wasip2`",
        workflow.display()
    );

    // Persisted (not a tempdir) so a failing live run leaves its
    // evidence behind for inspection.
    let sandbox = env::temp_dir().join(format!("guest-execute-loop-wasm-{}", std::process::id()));
    if sandbox.exists() {
        fs::remove_dir_all(&sandbox).expect("clearing a stale sandbox");
    }
    eprintln!("sandbox: {}", sandbox.display());

    let mut executor = ComposedExecutor::live(&workflow)
        .fixtures_root(&framework)
        .stage(release.join("omnia.wasm"), "omnia.wasm")
        .seed(["init", "./omnia.wasm", "--name", "demo", "--scaffold-only"]);
    for id in manifest::ADAPTERS {
        let name = id.split_once(':').map_or(id, |(_, name)| name);
        executor = executor.adapter(id, release.join(format!("{name}.wasm")));
    }

    let execution =
        executor.execute(&scenario, profile, &sandbox).await.expect("the executor completes setup");
    for (id, step) in execution.steps() {
        assert_eq!(step.exit_code, 0, "step `{id}` failed:\n{}\n{}", step.stdout, step.stderr);
    }

    let evaluators = Evaluators::default()
        .with(AssertionId::GuestJournalCadence, evaluate::guest::journal_cadence)
        .with(AssertionId::GuestGeneratedCrateVerifies, verify::generated_crates_verify);
    let results = grade::hard_with(&scenario, &execution, &evaluators);
    for result in &results {
        assert_eq!(
            result.outcome,
            Outcome::Pass,
            "hard assertion `{}` failed: {:?}",
            result.id,
            result.detail
        );
    }
    assert_eq!(grade::missing_outputs(&scenario, &execution), Vec::<String>::new());
}
