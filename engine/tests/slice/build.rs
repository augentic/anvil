//! Integration tests for `specify slice build`.
//!
//! Covers the two-phase agent contract that mirrors `specify source
//! survey` / `extract`: `prepare` assembles + persists a schema-valid
//! build request and emits `target.execution.agent` without
//! transitioning the slice; `finalize` frames entry with
//! `slice.build.started`, validates the agent-produced report, gates the
//! `built` transition, and emits `slice.build.succeeded` /
//! `slice.build.failed`. Also covers the `success`-with-blocking-finding
//! rejection and the `execution: tool` unsupported seam.

use std::fs;

use serde_json::Value;

use crate::common::{
    Project, expected_cache_dir, parse_json, read_journal_normalized, specify_cmd,
};

/// Create `my-slice`, seed a `specs/<domain>/spec.md` so the assembled
/// request carries a non-empty `specs[]`, and transition it to
/// `refined` — the lifecycle state `slice build` gates out of.
fn stage_refined_slice(project: &Project) {
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "my-slice"])
        .assert()
        .success();
    let spec_dir = project.slices_dir().join("my-slice/specs/identity");
    fs::create_dir_all(&spec_dir).expect("mkdir specs/identity");
    fs::write(spec_dir.join("spec.md"), "# Identity spec\n").expect("write spec.md");
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "transition", "my-slice", "refined"])
        .assert()
        .success();
}

/// Write `report` to `.specify/slices/my-slice/build/report.yaml`,
/// standing in for the agent's `build` brief output.
fn write_report(project: &Project, report: &str) {
    let build_dir = project.slices_dir().join("my-slice/build");
    fs::create_dir_all(&build_dir).expect("mkdir build dir");
    fs::write(build_dir.join("report.yaml"), report).expect("write report.yaml");
}

/// Write `composition` to `.specify/slices/my-slice/composition.yaml`,
/// the artifact the A4 coherence check inspects at finalize.
fn write_composition(project: &Project, composition: &str) {
    let slice_dir = project.slices_dir().join("my-slice");
    fs::write(slice_dir.join("composition.yaml"), composition).expect("write composition.yaml");
}

/// Collect the `event` ids in the slice's journal, in append order.
fn event_ids(events: &[Value]) -> Vec<&str> {
    events.iter().filter_map(|e| e["event"].as_str()).collect()
}

fn metadata(project: &Project) -> String {
    fs::read_to_string(project.slices_dir().join("my-slice/metadata.yaml")).expect("read metadata")
}

const SUCCESS_REPORT: &str = "\
version: 1
slice: my-slice
target: omnia@1.0.0
status: success
findings: []
";

/// A `status: success` report carrying a blocking (`critical`,
/// default-`open` `violation`) finding — the CLI rejects this with
/// `target-build-success-with-blocking-finding`.
const SUCCESS_WITH_BLOCKING_REPORT: &str = "\
version: 1
slice: my-slice
target: omnia@1.0.0
status: success
findings:
  - id: DIAG-0001
    title: Generated code fails to compile
    severity: critical
    source: tool
    artifact: code
    evidence:
      kind: snippet
      value: \"error[E0382]: borrow of moved value\"
    impact: The generated crate does not compile, so the slice cannot merge.
    remediation: Fix the borrow error before reporting success.
    fingerprint: \"sha256:0000000000000000000000000000000000000000000000000000000000000000\"
";

/// A success report declaring no UI surface (`ui-surface.screens: 0`).
const SUCCESS_REPORT_NO_UI: &str = "\
version: 1
slice: my-slice
target: omnia@1.0.0
status: success
findings: []
ui-surface:
  screens: 0
";

/// A success report declaring a UI surface (`ui-surface.screens: 2`).
const SUCCESS_REPORT_UI: &str = "\
version: 1
slice: my-slice
target: omnia@1.0.0
status: success
findings: []
ui-surface:
  screens: 2
";

/// A non-empty whole-document composition (one screen).
const COMPOSITION_ONE_SCREEN: &str = "\
version: 1
screens:
  home:
    name: Home
";

// ---------------------------------------------------------------------------
// agent prepare
// ---------------------------------------------------------------------------

#[test]
fn prepare_writes_request_no_transition() {
    let project = Project::init();
    stage_refined_slice(&project);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "build", "my-slice"])
        .assert()
        .success();

    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["slice"], "my-slice");
    assert_eq!(body["target"], "omnia");
    assert_eq!(body["execution"], "agent");
    let request_field = body["request"].as_str().expect("request path string");
    assert!(
        request_field.ends_with(".specify/slices/my-slice/build/request.yaml"),
        "handoff request path: {request_field}"
    );
    let report_field = body["report"].as_str().expect("report path string");
    assert!(
        report_field.ends_with(".specify/slices/my-slice/build/report.yaml"),
        "handoff report path: {report_field}"
    );
    assert!(
        body["build-brief"].as_str().expect("build-brief string").ends_with("briefs/build.md"),
        "handoff must point at the build brief"
    );

    // prepare wrote a schema-valid request (it schema-validates before
    // the write, so a successful prepare proves validity); spot-check
    // the closed-shape fields.
    let request_path = project.slices_dir().join("my-slice/build/request.yaml");
    assert!(request_path.is_file(), "prepare must write build/request.yaml");
    let raw = fs::read_to_string(&request_path).expect("read request.yaml");
    assert!(raw.contains("version: 1"), "request carries version, got:\n{raw}");
    assert!(raw.contains("slice: my-slice"), "request carries slice, got:\n{raw}");
    assert!(raw.contains("project-dir:"), "request carries project-dir, got:\n{raw}");
    assert!(raw.contains("specs/identity/spec.md"), "request enumerates the spec, got:\n{raw}");

    // prepare emits the agent-dispatch signal, but NOT the
    // `slice.build.started` frame — that is owned by finalize so a
    // prepare-time abort never leaves a dangling `started`.
    let events = read_journal_normalized(project.root());
    let agent = events
        .iter()
        .find(|e| e["event"] == "target.execution.agent")
        .expect("prepare emits target.execution.agent");
    assert_eq!(agent["payload"]["slice"], "my-slice", "agent event names the slice: {agent}");
    assert_eq!(agent["payload"]["target"], "omnia", "agent event names the target: {agent}");
    assert!(
        !event_ids(&events).contains(&"slice.build.started"),
        "prepare must NOT emit slice.build.started (finalize owns it), got: {:?}",
        event_ids(&events)
    );

    // prepare must not transition the slice.
    assert!(
        metadata(&project).contains("status: refined"),
        "prepare must leave the slice at refined; got:\n{}",
        metadata(&project)
    );
}

// ---------------------------------------------------------------------------
// agent finalize
// ---------------------------------------------------------------------------

#[test]
fn finalize_validates_and_gates_built() {
    let project = Project::init();
    stage_refined_slice(&project);
    write_report(&project, SUCCESS_REPORT);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "build", "my-slice", "--phase", "finalize"])
        .assert()
        .success();

    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["slice"], "my-slice");
    assert_eq!(body["target"], "omnia@1.0.0");
    assert_eq!(body["status"], "success");
    assert_eq!(body["findings"], 0);

    let events = read_journal_normalized(project.root());
    let ids = event_ids(&events);
    assert!(
        ids.contains(&"slice.build.started"),
        "finalize frames entry with slice.build.started, got: {ids:?}"
    );
    assert!(
        ids.contains(&"slice.build.succeeded"),
        "finalize emits slice.build.succeeded, got: {ids:?}"
    );

    // The gate transitioned the slice to `built`.
    assert!(
        metadata(&project).contains("status: built"),
        "finalize gates the built transition; got:\n{}",
        metadata(&project)
    );
}

#[test]
fn finalize_rejects_success_blocking() {
    let project = Project::init();
    stage_refined_slice(&project);
    write_report(&project, SUCCESS_WITH_BLOCKING_REPORT);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "build", "my-slice", "--phase", "finalize"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let stderr = parse_json(&assert.get_output().stderr);
    assert_eq!(stderr["error"], "target-build-success-with-blocking-finding");

    // The rejection neither transitions the slice nor records success.
    assert!(
        metadata(&project).contains("status: refined"),
        "a rejected report must not transition; got:\n{}",
        metadata(&project)
    );
    let events = read_journal_normalized(project.root());
    let ids = event_ids(&events);
    assert!(
        ids.contains(&"slice.build.failed"),
        "a rejected report emits slice.build.failed, got: {ids:?}"
    );
    assert!(
        !ids.contains(&"slice.build.succeeded"),
        "a rejected report must not emit slice.build.succeeded, got: {ids:?}"
    );
}

#[test]
fn finalize_missing_report_errors() {
    let project = Project::init();
    stage_refined_slice(&project);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "build", "my-slice", "--phase", "finalize"])
        .assert()
        .failure();
    let stderr = parse_json(&assert.get_output().stderr);
    assert_eq!(stderr["error"], "target-build-report-missing");
    assert!(
        metadata(&project).contains("status: refined"),
        "a missing report must not transition the slice"
    );
}

// ---------------------------------------------------------------------------
// A4: ui-surface coherence warnings (non-blocking)
// ---------------------------------------------------------------------------

#[test]
fn finalize_warns_unexpected_composition() {
    let project = Project::init();
    stage_refined_slice(&project);
    write_report(&project, SUCCESS_REPORT_NO_UI);
    write_composition(&project, COMPOSITION_ONE_SCREEN);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "build", "my-slice", "--phase", "finalize"])
        .assert()
        .success();
    assert_eq!(assert.get_output().status.code(), Some(0), "A4 warnings never alter the exit code");

    let body = parse_json(&assert.get_output().stdout);
    let warnings = body["warnings"].as_array().expect("warnings array present");
    assert_eq!(warnings.len(), 1, "one coherence warning expected: {body}");
    assert_eq!(warnings[0]["rule-id"], "composition-unexpected-for-non-ui-slice");

    // The warning never gates the build: the slice still reached `built`.
    assert!(
        metadata(&project).contains("status: built"),
        "an A4 warning never blocks the built transition; got:\n{}",
        metadata(&project)
    );
}

#[test]
fn finalize_warns_empty_composition() {
    let project = Project::init();
    stage_refined_slice(&project);
    write_report(&project, SUCCESS_REPORT_UI);
    // No composition.yaml staged: an absent composition is "empty".

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "build", "my-slice", "--phase", "finalize"])
        .assert()
        .success();
    assert_eq!(assert.get_output().status.code(), Some(0), "A4 warnings never alter the exit code");

    let body = parse_json(&assert.get_output().stdout);
    let warnings = body["warnings"].as_array().expect("warnings array present");
    assert_eq!(warnings.len(), 1, "one coherence warning expected: {body}");
    assert_eq!(warnings[0]["rule-id"], "composition-empty-for-ui-slice");
}

#[test]
fn finalize_matched_ui_surface_no_warnings() {
    let project = Project::init();
    stage_refined_slice(&project);
    write_report(&project, SUCCESS_REPORT_UI);
    write_composition(&project, COMPOSITION_ONE_SCREEN);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "build", "my-slice", "--phase", "finalize"])
        .assert()
        .success();

    let body = parse_json(&assert.get_output().stdout);
    assert!(
        body.get("warnings").is_none(),
        "a coherent ui-surface emits no warnings (field skipped): {body}"
    );
}

#[test]
fn finalize_absent_ui_surface_no_warnings() {
    let project = Project::init();
    stage_refined_slice(&project);
    write_report(&project, SUCCESS_REPORT);
    write_composition(&project, COMPOSITION_ONE_SCREEN);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "build", "my-slice", "--phase", "finalize"])
        .assert()
        .success();

    let body = parse_json(&assert.get_output().stdout);
    assert!(
        body.get("warnings").is_none(),
        "a report without ui-surface emits no warnings (back-compat): {body}"
    );
}

// ---------------------------------------------------------------------------
// execution: tool seam
// ---------------------------------------------------------------------------

#[test]
fn tool_execution_reports_unsupported_seam() {
    let project = Project::init();
    stage_refined_slice(&project);

    // `init` caches the resolved manifest; flip it to `execution: tool`
    // so the verb takes the tool branch. No build tool dispatch is
    // wired, so the dispatch is a clear unsupported seam.
    let cached = expected_cache_dir(project.root()).join("manifests/targets/omnia/adapter.yaml");
    let raw = fs::read_to_string(&cached).expect("read cached adapter.yaml");
    fs::write(&cached, raw.replace("execution: agent", "execution: tool"))
        .expect("rewrite adapter execution mode");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "build", "my-slice"])
        .assert()
        .failure();
    let stderr = parse_json(&assert.get_output().stderr);
    assert_eq!(stderr["error"], "target-build-tool-unsupported");
}
