//! The composed live loop: the canonical `guest-execute-loop`
//! scenario's `wasm-live` profile driven once through the shipped
//! `specify` binary over the release-built sibling adapter components,
//! then graded with the scenario's hard assertions.
//!
//! `#[ignore]`: requires an authenticated cursor-agent on `PATH`, the
//! sibling checkout release-built (`cargo make release` there), and
//! makes real model calls. The quality orchestrator
//! (`scripts/quality.rs`) owns repeated trials and semantic grading.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::{env, fs};

use scenario::{Grading, ModelBackend, Outcome, Runtime, catalog, evaluate, grade};
use specify_live_harness::driver;

fn cursor_agent_on_path() -> bool {
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| directory.join("cursor-agent").is_file())
    })
}

#[test]
#[ignore = "live: needs authenticated cursor-agent and release-built adapters; run with -- --ignored"]
fn wasm_live_trial_passes() {
    let scenario = catalog::load(driver::SCENARIO).expect("canonical scenario");
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

    // Persisted (not a tempdir) so a failing live run leaves its
    // evidence behind for inspection.
    let sandbox = env::temp_dir().join(format!("guest-execute-loop-wasm-{}", std::process::id()));
    if sandbox.exists() {
        fs::remove_dir_all(&sandbox).expect("clearing a stale sandbox");
    }
    eprintln!("sandbox: {}", sandbox.display());

    let steps = driver::drive(&driver::Config {
        framework,
        adapters,
        sandbox: sandbox.clone(),
    })
    .expect("the driver completes setup");
    for (id, step) in &steps {
        assert_eq!(step.exit_code, 0, "step `{id}` failed:\n{}\n{}", step.stdout, step.stderr);
    }

    let execution = grade::Execution::new(&sandbox, steps.into_iter().collect::<BTreeMap<_, _>>());
    let mut results = grade::hard(&scenario, &execution);
    evaluate::guest::guest(&mut results, &sandbox);
    for result in &results {
        assert_eq!(
            result.outcome,
            Outcome::Pass,
            "hard assertion `{}` failed: {:?}",
            result.id,
            result.detail
        );
    }
}
