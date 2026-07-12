//! Replay-backed composed coverage for the workflow guest's WASM-only
//! boundary, driven through the in-process composed executor.

#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use quality::executor::{ComposedExecutor, Executor as _};
use scenario::grade::Evaluators;
use scenario::{
    AssertionId, ModelBackend, Outcome, Runtime as ScenarioRuntime, Scenario, evaluate,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn init_dispatches_and_writes_preopens() -> Result<()> {
    let scenario = Scenario::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../quality/scenarios/composed-init.yaml"),
    )
    .context("loading composed scenario")?;
    let profile = scenario
        .profiles
        .iter()
        .find(|profile| profile.id == "replay")
        .context("composed scenario declares replay")?;
    assert_eq!(profile.runtime, ScenarioRuntime::Wasm);
    assert_eq!(profile.model, ModelBackend::Replay);
    assert_eq!(
        scenario.workflow[0].run,
        "specify init ./echo-target.wasm --name composed-smoke --scaffold-only"
    );

    let workspace = tempfile::tempdir().context("creating trial workspace")?;
    let executor = ComposedExecutor::replay(workflow_wasm())
        .adapter("target:echo-target", echo_target_wasm())
        .stage(echo_target_wasm(), "echo-target.wasm")
        .fixtures_root(repo_root());
    let execution = executor
        .execute(&scenario, profile, workspace.path())
        .await
        .context("driving the composed init scenario")?;

    let project_yaml = std::fs::read_to_string(execution.root().join(".specify/project.yaml"))
        .context("reading project.yaml written through the project preopen")?;
    assert!(project_yaml.contains("name: composed-smoke"), "{project_yaml}");
    assert!(project_yaml.contains("adapter: file://"), "{project_yaml}");
    assert!(
        execution.root().join(".specify-cache/components/echo-target.wasm").is_file(),
        "init mirrors the dispatched adapter through the writable cache preopen"
    );
    assert!(
        execution.root().join(".specify-cache/components/component-meta.yaml").is_file(),
        "init records component provenance through the writable cache preopen"
    );
    let assertions = scenario::grade::hard(&scenario, &execution);
    assert!(
        assertions.iter().all(|assertion| assertion.outcome == Outcome::Pass),
        "{assertions:?}"
    );
    assert_eq!(scenario::grade::missing_outputs(&scenario, &execution), Vec::<String>::new());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn replay_drives_full_loop() -> Result<()> {
    let scenario = Scenario::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../quality/scenarios/composed-loop.yaml"),
    )
    .context("loading composed loop scenario")?;
    let profile = scenario
        .profiles
        .iter()
        .find(|profile| profile.id == "replay")
        .context("composed loop scenario declares replay")?;

    let workspace = tempfile::tempdir().context("creating trial workspace")?;
    let executor = ComposedExecutor::replay(workflow_wasm())
        .adapter("source:echo-source", echo_source_wasm())
        .adapter("target:echo-target", echo_target_wasm())
        .stage(echo_target_wasm(), "echo-target.wasm")
        .fixtures_root(repo_root());
    let execution = executor
        .execute(&scenario, profile, workspace.path())
        .await
        .context("driving the composed loop scenario")?;
    for (id, step) in execution.steps() {
        assert_eq!(step.exit_code, 0, "composed step `{id}` failed:\n{}", step.stderr);
    }

    let evaluators = Evaluators::default()
        .with(AssertionId::ComposedPlanDrained, evaluate::composed::plan_drained)
        .with(AssertionId::ComposedArtifactsComplete, evaluate::composed::artifacts_complete)
        .with(
            AssertionId::ComposedBaselineMergeVisible,
            evaluate::composed::baseline_merge_visible,
        );
    let assertions = scenario::grade::hard_with(&scenario, &execution, &evaluators);
    assert!(
        assertions.iter().all(|assertion| assertion.outcome == Outcome::Pass),
        "{assertions:?}"
    );
    assert!(
        execution.root().join(".specify/specs/echo/spec.md").is_file(),
        "merge writes the baseline spec"
    );
    assert_eq!(scenario::grade::missing_outputs(&scenario, &execution), Vec::<String>::new());
    Ok(())
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn workflow_wasm() -> PathBuf {
    guest_wasm("specify.wasm")
}

fn echo_target_wasm() -> PathBuf {
    guest_wasm("examples/echo_target.wasm")
}

fn echo_source_wasm() -> PathBuf {
    guest_wasm("examples/echo_source.wasm")
}

fn guest_wasm(relative: &str) -> PathBuf {
    let path = repo_root().join("target/wasm32-wasip2/debug").join(relative);
    assert!(
        path.is_file(),
        "guest `{relative}` not found at {}; run `cargo make guests` in harness/",
        path.display()
    );
    path
}
