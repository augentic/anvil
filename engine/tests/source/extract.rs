//! Integration tests for `specify source extract`
//! (DECISIONS.md §"Source operations").
//!
//! Covers source resolution against `plan.yaml.sources`, the agent
//! two-phase dispatch (prepare prints the extract handoff envelope —
//! with `evidence-dir`, a single lead, and either `source-dir` or
//! `value-inline` — and emits `source.execution.agent`; finalize
//! validates-before-visible, persists the Evidence, and emits
//! `slice.extract.completed`), the validate-before-visible guarantee
//! that an invalid Evidence document persists no file, the value-bound
//! `intent` path, and the sandbox path-denied eval scenario `5j`
//! (`$PROJECT_DIR` invisible to the adapter; out-of-sandbox Evidence
//! denied).

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::common::{
    Project, TEMPDIR_PLACEHOLDER, expected_cache_dir, init_workspace, omnia_schema_dir,
    parse_stderr, parse_stdout, repo_root, specify_cmd,
};

/// Stage the path-bound `typescript` source adapter (the in-repo
/// fixture ships only `adapter.yaml`; author the `extract` brief the
/// agent reads).
fn stage_typescript(project: &Project) {
    let src = repo_root()
        .join("crates/workflow/tests/fixtures/plugins/adapters/sources/typescript/adapter.yaml");
    let adapter_dir = project.root().join("adapters/sources/typescript");
    fs::create_dir_all(adapter_dir.join("briefs")).expect("create adapter briefs dir");
    fs::copy(&src, adapter_dir.join("adapter.yaml")).expect("copy adapter.yaml");
    fs::write(adapter_dir.join("briefs/extract.md"), "# extract brief\n")
        .expect("write extract brief");
}

/// Author a value-bound `intent` source adapter (`execution: agent`).
fn stage_intent(project: &Project) {
    let adapter_dir = project.root().join("adapters/sources/intent");
    fs::create_dir_all(adapter_dir.join("briefs")).expect("create adapter briefs dir");
    fs::write(
        adapter_dir.join("adapter.yaml"),
        "name: intent
version: 1.0.0
axis: source
execution: agent
briefs:
  survey: briefs/survey.md
  extract: briefs/extract.md
description: Operator-supplied free-form intent.
",
    )
    .expect("write adapter.yaml");
    fs::write(adapter_dir.join("briefs/extract.md"), "# extract brief\n")
        .expect("write extract brief");
}

fn seed_plan_with_legacy_source(project: &Project) {
    project.seed_plan(
        "name: platform-v2
sources:
  legacy:
    adapter: typescript
    path: vendor/legacy
slices:
  - name: identity
    project: default
    status: pending
",
    );
}

fn seed_plan_with_value_source(project: &Project) {
    project.seed_plan(
        "name: platform-v2
sources:
  brief:
    adapter: intent
    value: Build a password reset flow.
slices:
  - name: identity
    project: default
    status: pending
",
    );
}

fn extract_scratch_dir(project: &Project, adapter: &str, slice: &str) -> PathBuf {
    project.root().join(format!(".specify/scratch/{adapter}/{slice}"))
}

fn slice_evidence_path(project: &Project, slice: &str, source: &str) -> PathBuf {
    project.root().join(format!(".specify/slices/{slice}/evidence/{source}.yaml"))
}

fn journal_events(project: &Project) -> Vec<Value> {
    let path = project.root().join(".specify/journal.jsonl");
    let raw = fs::read_to_string(&path).unwrap_or_else(|err| panic!("read journal.jsonl: {err}"));
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("journal line is JSON"))
        .collect()
}

const VALID_EVIDENCE: &str = "\
authority: behaviour
lead: user-registration
claims: []
";

#[test]
fn prepare_prints_envelope_emits_event() {
    let project = Project::init();
    stage_typescript(&project);
    seed_plan_with_legacy_source(&project);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "extract", "legacy", "user-registration"])
        .args(["--slice", "identity"])
        .assert()
        .success();

    let body = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(body["adapter"], "typescript");
    assert_eq!(body["version"], "1.0.0");
    assert_eq!(body["execution"], "agent");

    // extract — unlike survey — carries evidence-dir.
    let evidence = body["evidence-dir"].as_str().expect("evidence-dir str");
    assert!(
        evidence.ends_with(".specify/slices/identity/evidence"),
        "evidence-dir must target the slice evidence tree: {evidence}"
    );
    let scratch = body["scratch-dir"].as_str().expect("scratch-dir str");
    assert!(
        scratch.ends_with(".specify/scratch/typescript/identity"),
        "scratch-dir {scratch} must key under the slice segment"
    );
    let source_dir = body["source-dir"].as_str().expect("path binding carries source-dir");
    assert!(source_dir.ends_with("vendor/legacy"), "source-dir: {source_dir}");
    assert!(
        body.get("value-inline").is_none(),
        "a path binding must not carry value-inline, got:\n{body}"
    );
    let leads: Vec<&str> =
        body["leads"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(leads, vec!["user-registration"], "extract carries the single lead");

    // prepare builds scratch up front and scaffolds the evidence target.
    assert!(
        extract_scratch_dir(&project, "typescript", "identity").is_dir(),
        "prepare must create the scratch dir"
    );
    assert!(
        project.root().join(".specify/slices/identity/evidence").is_dir(),
        "prepare must scaffold the slice evidence/ dir"
    );

    let events = journal_events(&project);
    assert_eq!(events.len(), 1, "prepare emits exactly one event");
    assert_eq!(events[0]["event"], "source.execution.agent");
    assert_eq!(events[0]["payload"]["source"], "legacy");
    assert_eq!(events[0]["payload"]["adapter"], "typescript");
    assert_eq!(events[0]["payload"]["operation"], "extract");
}

#[test]
fn prepare_resolves_via_plan_dir() {
    // Workspace routing: extract runs inside a plan-less slot with
    // `--plan-dir` naming the initiating workspace root. The plan loads
    // from the override, and the binding's *relative* `path:` resolves
    // against the plan's home — the workspace — not the slot.
    let project = Project::init();
    stage_typescript(&project);
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::write(
        workspace.path().join("plan.yaml"),
        "name: platform-v2
sources:
  legacy:
    adapter: typescript
    path: vendor/legacy
slices:
  - name: identity
    project: default
    status: pending
",
    )
    .expect("write workspace plan.yaml");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "--plan-dir"])
        .arg(workspace.path())
        .args(["source", "extract", "legacy", "user-registration"])
        .args(["--slice", "identity"])
        .assert()
        .success();

    let body = parse_stdout(&assert.get_output().stdout, project.root());
    let source_dir = body["source-dir"].as_str().expect("path binding carries source-dir");
    assert_eq!(
        source_dir,
        workspace.path().join("vendor/legacy").to_str().expect("utf8 workspace path"),
        "relative source path must join the plan root, not the slot"
    );
    // Slot-anchored outputs stay slot-anchored.
    let evidence = body["evidence-dir"].as_str().expect("evidence-dir str");
    assert_eq!(evidence, format!("{TEMPDIR_PLACEHOLDER}/.specify/slices/identity/evidence"));
}

#[test]
fn slot_extract_resolves_after_sync() {
    // Slot adapter provisioning: the source adapter is vendored
    // only at the workspace; `specify workspace sync` mirrors it into
    // the slot's manifest cache, and slot-side extract resolves it
    // through ordinary project-local probing — no new resolution
    // semantics, no manual cache staging.
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    init_workspace(&workspace, "platform-workspace");

    // A local peer that is itself a Specify project, bound as a slot.
    let peer = workspace.path().join("peer");
    fs::create_dir_all(&peer).expect("create peer dir");
    specify_cmd()
        .current_dir(&peer)
        .args(["init"])
        .arg(omnia_schema_dir())
        .args(["--name", "peer"])
        .assert()
        .success();
    fs::write(
        workspace.path().join("registry.yaml"),
        "version: 1
projects:
  - name: peer
    url: ./peer
    adapter: omnia@1.0.0
",
    )
    .expect("write registry.yaml");
    fs::write(
        workspace.path().join("plan.yaml"),
        "name: platform-v2
sources:
  legacy:
    adapter: typescript
    path: vendor/legacy
slices:
  - name: identity
    project: peer
    status: pending
",
    )
    .expect("write workspace plan.yaml");

    // Vendor the source adapter at the workspace only.
    let adapter_src = repo_root()
        .join("crates/workflow/tests/fixtures/plugins/adapters/sources/typescript/adapter.yaml");
    let adapter_dir = workspace.path().join("adapters/sources/typescript");
    fs::create_dir_all(adapter_dir.join("briefs")).expect("create workspace adapter dir");
    fs::copy(&adapter_src, adapter_dir.join("adapter.yaml")).expect("copy adapter.yaml");
    fs::write(adapter_dir.join("briefs/extract.md"), "# extract brief\n")
        .expect("write extract brief");

    specify_cmd().current_dir(workspace.path()).args(["workspace", "sync"]).assert().success();

    let slot = workspace.path().join("workspace/peer");
    assert!(
        expected_cache_dir(&slot).join("manifests/sources/typescript/adapter.yaml").is_file(),
        "sync must mirror the workspace adapter into the slot manifest cache"
    );

    let assert = specify_cmd()
        .current_dir(&slot)
        .args(["--format", "json", "--plan-dir"])
        .arg(workspace.path())
        .args(["source", "extract", "legacy", "user-registration"])
        .args(["--slice", "identity"])
        .assert()
        .success();

    let body = parse_stdout(&assert.get_output().stdout, &peer);
    assert_eq!(body["adapter"], "typescript", "the mirrored adapter must resolve in the slot");
    let evidence = body["evidence-dir"].as_str().expect("evidence-dir str");
    assert!(
        evidence.ends_with(".specify/slices/identity/evidence"),
        "slice state stays slot-local: {evidence}"
    );
}

#[test]
fn prepare_value_bound_carries_inline() {
    let project = Project::init();
    stage_intent(&project);
    seed_plan_with_value_source(&project);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "extract", "brief", "password-reset"])
        .args(["--slice", "identity"])
        .assert()
        .success();

    let body = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(body["adapter"], "intent");
    assert!(
        body.get("source-dir").is_none(),
        "value-bound source must NOT carry source-dir, got:\n{body}"
    );
    assert_eq!(
        body["value-inline"], "Build a password reset flow.",
        "value-bound source carries the literal binding body"
    );
    assert!(body.get("evidence-dir").is_some(), "extract always carries evidence-dir");
    let leads: Vec<&str> =
        body["leads"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(leads, vec!["password-reset"]);
}

#[test]
fn finalize_persists_and_completes() {
    let project = Project::init();
    stage_typescript(&project);
    seed_plan_with_legacy_source(&project);

    // Stand in for the agent: write the produced Evidence into scratch.
    let scratch = extract_scratch_dir(&project, "typescript", "identity");
    fs::create_dir_all(&scratch).expect("create scratch dir");
    fs::write(scratch.join("evidence.yaml"), VALID_EVIDENCE).expect("write evidence.yaml");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "extract", "legacy", "user-registration"])
        .args(["--slice", "identity", "--phase", "finalize"])
        .assert()
        .success();

    let body = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(body["adapter"], "typescript");
    assert_eq!(body["source"], "legacy");
    assert_eq!(body["slice"], "identity");
    assert_eq!(body["lead"], "user-registration");

    // The validated Evidence is now persisted to the slice evidence path.
    let persisted = slice_evidence_path(&project, "identity", "legacy");
    assert!(persisted.is_file(), "Evidence persisted to {}", persisted.display());
    assert_eq!(fs::read_to_string(&persisted).expect("read persisted"), VALID_EVIDENCE);

    let events = journal_events(&project);
    let completed = events
        .iter()
        .find(|e| e["event"] == "slice.extract.completed")
        .expect("a slice.extract.completed event");
    assert_eq!(completed["payload"]["slice-name"], "identity");
    assert_eq!(completed["payload"]["source"], "legacy");
}

#[test]
fn finalize_value_bound_persists() {
    let project = Project::init();
    stage_intent(&project);
    seed_plan_with_value_source(&project);

    let scratch = extract_scratch_dir(&project, "intent", "identity");
    fs::create_dir_all(&scratch).expect("create scratch dir");
    fs::write(
        scratch.join("evidence.yaml"),
        "authority: intent\nlead: password-reset\nclaims: []\n",
    )
    .expect("write evidence.yaml");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "extract", "brief", "password-reset"])
        .args(["--slice", "identity", "--phase", "finalize"])
        .assert()
        .success();

    let body = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(body["adapter"], "intent");

    assert!(
        slice_evidence_path(&project, "identity", "brief").is_file(),
        "value-bound Evidence persists with no $SOURCE_DIR present"
    );
}

#[test]
fn finalize_invalid_persists_no_file() {
    let project = Project::init();
    stage_typescript(&project);
    seed_plan_with_legacy_source(&project);

    // Missing the required `claims` field — parses as YAML but fails the schema.
    let scratch = extract_scratch_dir(&project, "typescript", "identity");
    fs::create_dir_all(&scratch).expect("create scratch dir");
    fs::write(scratch.join("evidence.yaml"), "authority: behaviour\nlead: user-registration\n")
        .expect("write invalid evidence.yaml");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "extract", "legacy", "user-registration"])
        .args(["--slice", "identity", "--phase", "finalize"])
        .assert()
        .failure();

    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "evidence-schema");
    assert_eq!(stderr["exit-code"], 2);

    // Validate-before-visible: no Evidence file lands on the slice path.
    assert!(
        !slice_evidence_path(&project, "identity", "legacy").exists(),
        "an invalid Evidence document must persist no file"
    );
    // No completion event fires for an invalid Evidence document.
    assert!(
        !project.root().join(".specify/journal.jsonl").exists()
            || !journal_events(&project).iter().any(|e| e["event"] == "slice.extract.completed"),
        "invalid Evidence must not emit a completion event"
    );
}

/// Acceptance scenario `extract-failure` — the extract step fails to
/// produce Evidence (the agent's extract brief ran but staged nothing in
/// `$SCRATCH_DIR`). finalize fails closed with `extract-evidence-missing`,
/// persists no Evidence, emits no completion event, and leaves the slice
/// `refining` so no synthesis can run. Distinct from
/// `finalize_invalid_persists_no_file` (schema failure on a *present*
/// document) and `sandbox_denies_out_of_scope` (a document staged outside
/// the granted scratch root).
#[test]
fn finalize_missing_evidence_stays_refining() {
    let project = Project::init();
    stage_typescript(&project);
    seed_plan_with_legacy_source(&project);

    // The agent produced nothing: scratch exists but holds no evidence.yaml.
    let scratch = extract_scratch_dir(&project, "typescript", "identity");
    fs::create_dir_all(&scratch).expect("create empty scratch dir");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "extract", "legacy", "user-registration"])
        .args(["--slice", "identity", "--phase", "finalize"])
        .assert()
        .failure();

    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "extract-evidence-missing");
    assert_eq!(stderr["exit-code"], 1);

    // No Evidence persisted: the slice never leaves refining, so no
    // synthesis can run against it.
    assert!(
        !slice_evidence_path(&project, "identity", "legacy").exists(),
        "a failed extract must persist no Evidence"
    );
    // A failed extract fires no completion event.
    assert!(
        !project.root().join(".specify/journal.jsonl").exists()
            || !journal_events(&project).iter().any(|e| e["event"] == "slice.extract.completed"),
        "a failed extract must not emit a completion event"
    );
}

/// Scenario `5j` — source-adapter sandbox path-denied (the parent
/// `augentic/specify` repo's `docs/contributing/evals.md`
/// §Scenario IDs, stub `05j-source-sandbox-denied.md`).
///
/// Proves the two halves of the four-root sandbox the C5 prep seam
/// lays out (`$SOURCE_DIR` read-only, `$CAPABILITY_DIR` read-only,
/// `$SCRATCH_DIR` write-only, `$PROJECT_DIR` none):
///
/// (a) `$PROJECT_DIR` is invisible to the adapter operation — the agent
///     handoff envelope carries no `project-dir`, and never grants the
///     project root itself (the directory holding `plan.yaml` and the
///     `.specify/` lifecycle state). Only descendant subpaths are
///     handed over.
/// (b) An out-of-sandbox path is denied — the runner reads the
///     agent-produced Evidence *only* from the granted `$SCRATCH_DIR`.
///     Evidence the adapter stages outside its sandbox roots (here at
///     the project root, which `$PROJECT_DIR: none` makes unreachable)
///     is not honoured: finalize fails closed with
///     `extract-evidence-missing`, persists no Evidence, and leaves the
///     slice `refining`.
///
/// Source operations are agent-only, so the denial is structural —
/// the runner never mounts or hands over `$PROJECT_DIR` — rather than
/// a live WASI preopen rejection.
#[test]
fn sandbox_denies_out_of_scope() {
    let project = Project::init();
    stage_typescript(&project);
    seed_plan_with_legacy_source(&project);

    // (a) prepare: the handoff envelope must not expose $PROJECT_DIR.
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "extract", "legacy", "user-registration"])
        .args(["--slice", "identity"])
        .assert()
        .success();
    let body = parse_stdout(&assert.get_output().stdout, project.root());
    assert!(
        body.get("project-dir").is_none(),
        "the sandbox must not hand $PROJECT_DIR to the adapter, got:\n{body}"
    );
    // Every granted root is a strict descendant of the project root; the
    // project root itself (= $PROJECT_DIR, holding plan.yaml and the
    // .specify/ lifecycle state) is never a grant. parse_stdout has
    // rewritten the project root to the TEMPDIR placeholder.
    for key in ["briefs-dir", "source-dir", "scratch-dir", "evidence-dir"] {
        let value = body[key].as_str().unwrap_or_else(|| panic!("{key} str in:\n{body}"));
        assert_ne!(value, TEMPDIR_PLACEHOLDER, "{key} must not grant the project root itself");
        assert!(
            value.starts_with(&format!("{TEMPDIR_PLACEHOLDER}/")),
            "{key} {value} must sit under the project root, not escape it"
        );
    }

    // (b) finalize: stage the Evidence OUTSIDE the granted $SCRATCH_DIR,
    // at the project root that $PROJECT_DIR: none makes unreachable. The
    // runner reads only $SCRATCH_DIR/evidence.yaml, so an out-of-sandbox
    // document is denied — never read, never persisted.
    fs::write(project.root().join("evidence.yaml"), VALID_EVIDENCE)
        .expect("stage out-of-sandbox evidence");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "extract", "legacy", "user-registration"])
        .args(["--slice", "identity", "--phase", "finalize"])
        .assert()
        .failure();
    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "extract-evidence-missing");
    assert_eq!(stderr["exit-code"], 1);

    // No Evidence lands on the slice path; the slice stays refining.
    assert!(
        !slice_evidence_path(&project, "identity", "legacy").exists(),
        "out-of-sandbox Evidence must not be persisted"
    );
    // A denied finalize fails before any completion event is emitted.
    assert!(
        !journal_events(&project).iter().any(|e| e["event"] == "slice.extract.completed"),
        "a denied out-of-sandbox extract must not emit a completion event"
    );
}

#[test]
fn unknown_source_errors() {
    let project = Project::init();
    stage_typescript(&project);
    seed_plan_with_legacy_source(&project);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "extract", "not-a-source", "user-registration"])
        .args(["--slice", "identity"])
        .assert()
        .failure();

    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "source-unknown");
    assert_eq!(stderr["exit-code"], 1);
}
