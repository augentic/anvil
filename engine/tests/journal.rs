//! Integration tests for the workflow §Observability journal events.
//!
//! Verifies that each CLI-owned emit site writes the documented wire
//! shape into `.specify/journal.jsonl` and that the agent-facing
//! [`specify_workflow::journal::append_batch`] helper can be driven directly
//! from a synthesised event. Golden files under
//! `tests/fixtures/journal/` pin the canonical JSON-line shape; rerun
//! with `REGENERATE_GOLDENS=1 cargo nextest run --test journal` to refresh.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use specify_workflow::change::Divergence;
use specify_workflow::config::Layout;
use specify_workflow::journal::{self, Event, EventKind};

mod common;
use common::{Project, assert_golden_at, parse_stderr, parse_stdout, repo_root, specify_cmd};

/// Pinned RFC 3339 timestamp used by every golden snapshot. CLI-driven
/// emits use `Timestamp::now()`; tests normalise the value to this
/// placeholder before diffing so goldens stay deterministic.
const FIXED_TIMESTAMP: &str = "2026-05-21T20:00:00Z";

fn fixtures_dir() -> PathBuf {
    repo_root().join("tests/fixtures/journal")
}

fn journal_path(project_root: &Path) -> PathBuf {
    project_root.join(".specify").join("journal.jsonl")
}

/// Read `.specify/journal.jsonl`, return one `Value` per line. Strips
/// trailing blank lines so the asserted shape matches the golden
/// regardless of writer quirks.
fn read_journal(project_root: &Path) -> Vec<Value> {
    let raw = fs::read_to_string(journal_path(project_root))
        .unwrap_or_else(|err| panic!("read journal.jsonl: {err}"));
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("journal line is JSON"))
        .collect()
}

/// Normalise every event's `timestamp` to [`FIXED_TIMESTAMP`] so the
/// golden snapshot is stable across runs.
fn normalise_timestamps(mut events: Vec<Value>) -> Vec<Value> {
    for event in &mut events {
        if let Value::Object(map) = event
            && map.contains_key("timestamp")
        {
            map.insert("timestamp".to_string(), Value::String(FIXED_TIMESTAMP.to_string()));
        }
    }
    events
}

fn assert_journal_golden(name: &str, events: Vec<Value>) {
    let actual = Value::Array(normalise_timestamps(events));
    assert_golden_at(&fixtures_dir(), name, actual);
}

// -- plan.transition.approved ----------------------------------------

#[test]
fn plan_transition_emits_event() {
    let project = Project::init();
    project.seed_plan(
        "name: platform-v2
slices:
  - name: a
    project: default
    status: done
",
    );

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "transition", "platform-v2", "approved"])
        .assert()
        .success();

    let events = read_journal(project.root());
    assert_eq!(events.len(), 1, "expected one journal event, got {}", events.len());
    assert_eq!(events[0]["event"], "plan.transition.approved");
    assert_eq!(events[0]["payload"]["plan-name"], "platform-v2");
    assert_eq!(
        events[0]["payload"]["actor"], "operator",
        "bare `plan transition` must record the default actor"
    );
    assert!(
        events[0]["timestamp"].as_str().is_some(),
        "timestamp must be present, got:\n{}",
        events[0]
    );
    assert_journal_golden("plan-transition-approved.json", events);
}

#[test]
fn plan_transition_records_agent_actor() {
    // `--actor agent` is self-reported grading evidence for the eval
    // probes (`gate-1-not-auto-stamped`): the stamp succeeds either
    // way, but the journal records who drove it.
    let project = Project::init();
    project.seed_plan(
        "name: platform-v2
slices:
  - name: a
    project: default
    status: done
",
    );

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "transition", "platform-v2", "approved", "--actor", "agent"])
        .assert()
        .success();

    let events = read_journal(project.root());
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "plan.transition.approved");
    assert_eq!(events[0]["payload"]["actor"], "agent");
}

#[test]
fn plan_transition_rejects_unknown_actor() {
    let project = Project::init();
    project.seed_plan(
        "name: platform-v2
slices:
  - name: a
    project: default
    status: done
",
    );

    let assert = specify_cmd()
        .current_dir(project.root())
        .args([
            "--format",
            "json",
            "plan",
            "transition",
            "platform-v2",
            "approved",
            "--actor",
            "robot",
        ])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let actual = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(actual["error"], "argument");
    assert!(
        !journal_path(project.root()).exists(),
        "a rejected --actor must not stamp the plan nor append to the journal"
    );
}

// -- plan.amend.divergence -------------------------------------------

const TWO_SLICE_PLAN: &str = "\
name: platform-v2
slices:
  - name: checkout
    project: default
    status: pending
  - name: billing
    project: default
    status: pending
";

#[test]
fn amend_divergence_none_to_accepted() {
    // source/target split note: the implicit-default first transition
    // serialises `from: none` because the on-disk slice has no
    // `divergence:` key.
    let project = Project::init();
    project.seed_plan(TWO_SLICE_PLAN);

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "checkout", "--divergence", "accepted"])
        .assert()
        .success();

    let events = read_journal(project.root());
    assert_eq!(events.len(), 1);
    let payload = &events[0]["payload"];
    assert_eq!(events[0]["event"], "plan.amend.divergence");
    assert_eq!(payload["plan-name"], "platform-v2");
    assert_eq!(payload["slice-name"], "checkout");
    assert_eq!(payload["from"], "none");
    assert_eq!(payload["to"], "accepted");
    assert_journal_golden("plan-amend-divergence-none-to-accepted.json", events);
}

#[test]
fn plan_amend_divergence_none_to_likely() {
    // divergence and writer-ownership contract: the CLI is the single writer of every variant of
    // `slices[].divergence`, including `likely`. A `plan amend
    // --divergence likely` against a slice with no prior divergence
    // emits one `plan.amend.divergence` event with `from: none, to:
    // likely`.
    let project = Project::init();
    project.seed_plan(TWO_SLICE_PLAN);

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "checkout", "--divergence", "likely"])
        .assert()
        .success();

    let events = read_journal(project.root());
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "plan.amend.divergence");
    assert_eq!(events[0]["payload"]["from"], "none");
    assert_eq!(events[0]["payload"]["to"], "likely");
}

#[test]
fn plan_amend_divergence_likely_round_trips() {
    // divergence and writer-ownership contract: `specify plan amend --divergence likely` is the
    // bare-skill fallback writer of `slices[].divergence: likely`.
    // The CLI must persist the field byte-identically to the legacy
    // skill-written form so existing fixtures keep round-tripping.
    let project = Project::init();
    project.seed_plan(
        "name: demo
slices:
  - name: checkout
    project: default
    status: pending
",
    );

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "checkout", "--divergence", "likely"])
        .assert()
        .success();

    let saved = fs::read_to_string(project.root().join("plan.yaml")).expect("read plan");
    assert!(
        saved.contains("divergence: likely"),
        "amend --divergence likely must persist the field byte-identically:\n{saved}"
    );

    let events = read_journal(project.root());
    assert_eq!(events.len(), 1, "exactly one journal event per CLI write");
    assert_eq!(events[0]["event"], "plan.amend.divergence");
    assert_eq!(events[0]["payload"]["to"], "likely");
}

#[test]
fn amend_divergence_likely_to_rejected() {
    // source/target split note: `propose` writes `divergence: likely` and
    // the operator may transition it to `rejected` at Gate 1.
    let project = Project::init();
    project.seed_plan(
        "name: platform-v2
slices:
  - name: checkout
    project: default
    status: pending
    divergence: likely
",
    );

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "checkout", "--divergence", "rejected"])
        .assert()
        .success();

    let events = read_journal(project.root());
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "plan.amend.divergence");
    assert_eq!(events[0]["payload"]["from"], "likely");
    assert_eq!(events[0]["payload"]["to"], "rejected");
    assert_journal_golden("plan-amend-divergence-likely-to-rejected.json", events);
}

#[test]
fn amend_divergence_accepted_to_rejected() {
    let project = Project::init();
    project.seed_plan(
        "name: platform-v2
slices:
  - name: checkout
    project: default
    status: pending
    divergence: accepted
",
    );

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "checkout", "--divergence", "rejected"])
        .assert()
        .success();

    let events = read_journal(project.root());
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["payload"]["from"], "accepted");
    assert_eq!(events[0]["payload"]["to"], "rejected");
    assert_journal_golden("plan-amend-divergence-accepted-to-rejected.json", events);
}

#[test]
fn amend_divergence_rejected_to_accepted() {
    let project = Project::init();
    project.seed_plan(
        "name: platform-v2
slices:
  - name: checkout
    project: default
    status: pending
    divergence: rejected
",
    );

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "checkout", "--divergence", "accepted"])
        .assert()
        .success();

    let events = read_journal(project.root());
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["payload"]["from"], "rejected");
    assert_eq!(events[0]["payload"]["to"], "accepted");
    assert_journal_golden("plan-amend-divergence-rejected-to-accepted.json", events);
}

#[test]
fn plan_amend_without_divergence_no_event() {
    let project = Project::init();
    project.seed_plan(TWO_SLICE_PLAN);

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "checkout", "--description", "scope hint"])
        .assert()
        .success();

    assert!(
        !journal_path(project.root()).exists(),
        "amends without --divergence must not write a journal event"
    );
}

// -- slice.transition.refined ----------------------------------------

#[test]
fn slice_create_writes_no_refined_journal() {
    let project = Project::init();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "checkout"])
        .assert()
        .success();
    assert!(
        !journal_path(project.root()).exists(),
        "slice create must not emit slice.transition.refined"
    );
}

#[test]
fn slice_transition_refined_writes() {
    let project = Project::init();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "checkout"])
        .assert()
        .success();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "transition", "checkout", "refined"])
        .assert()
        .success();

    let events = normalise_timestamps(read_journal(project.root()));
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "slice.transition.refined");
    assert_eq!(events[0]["payload"]["slice-name"], "checkout");
    assert_journal_golden("slice-transition-refined.json", events);
}

#[test]
fn slice_transition_built_no_refined_event() {
    let project = Project::init();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "checkout"])
        .assert()
        .success();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "transition", "checkout", "refined"])
        .assert()
        .success();
    let before = read_journal(project.root()).len();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "transition", "checkout", "built"])
        .assert()
        .success();
    assert_eq!(
        read_journal(project.root()).len(),
        before,
        "built transition must not append slice.transition.refined"
    );
}

// -- slice.synthesis.* (specify slice validate) ----------------------

const PLAN_WITH_LEGACY_MONOLITH: &str = "\
name: workflow-prov
lifecycle: pending
sources:
  legacy-monolith:
    adapter: typescript
    path: ./legacy
slices:
  - name: my-slice
    status: pending
    sources:
      - { source: legacy-monolith, lead: my-slice }
";

const TAGGED_SPEC_UNKNOWN: &str = "# Login Specification

## Purpose

Password reset flow for registered users.

### Requirement: Password reset request [unknown]

ID: REQ-001
Sources: [legacy-monolith]
Status: unknown

The system lets a registered user request a password reset link by email.

#### Scenario: Reset requested

- **WHEN** a user submits a registered email
- **THEN** the system acknowledges the request
";

fn stage_slice_for_synthesis_journal() -> Project {
    let project = Project::init().with_schemas();
    project.stage_slice("good-slice");
    project.seed_plan(PLAN_WITH_LEGACY_MONOLITH);
    let spec_path = project.slices_dir().join("my-slice/specs/login/spec.md");
    fs::write(&spec_path, TAGGED_SPEC_UNKNOWN).expect("write tagged spec.md");
    project
}

#[test]
fn slice_validate_appends_synthesis() {
    let project = stage_slice_for_synthesis_journal();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "validate", "my-slice"])
        .assert()
        .success();

    let events = normalise_timestamps(read_journal(project.root()));
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "slice.synthesis.unknown");
    assert_eq!(events[0]["payload"]["slice-name"], "my-slice");
    assert_eq!(events[0]["payload"]["requirement-id"], "REQ-001");
    assert_journal_golden("slice-validate-synthesis-unknown.json", events);
}

#[test]
fn slice_validate_provenance_no_journal() {
    let project = stage_slice_for_synthesis_journal();
    let spec_path = project.slices_dir().join("my-slice/specs/login/spec.md");
    let bad = TAGGED_SPEC_UNKNOWN.replace("Status: unknown", "Status: agreed");
    fs::write(&spec_path, bad).expect("rewrite spec with tag/status mismatch");

    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "validate", "my-slice"])
        .assert()
        .failure();

    assert!(
        !journal_path(project.root()).exists(),
        "provenance failure must not append slice.synthesis.* events"
    );
}

// -- append_batch helper (plan.amend.divergence, slice.extract.completed)

#[test]
fn agent_emit_one_event_per_line() {
    // Exercises the public Rust `append_batch` helper. The harness
    // drives `append` directly because the CLI does not own a
    // `journal append` verb (workflow §"What was cut and why").
    // `slice.synthesis.*` is CLI-owned via `specify slice validate`;
    // `slice.extract.completed` is CLI-owned via
    // `specify source extract --phase finalize`.
    let project = Project::init();
    let layout = Layout::new(project.root());
    let fixed: jiff::Timestamp =
        FIXED_TIMESTAMP.parse().expect("fixed timestamp parses as rfc3339");

    let events = [
        Event::new(
            fixed,
            EventKind::PlanAmendDivergence {
                plan_name: "platform-v2".into(),
                slice_name: "checkout".into(),
                from: Divergence::None,
                to: Divergence::Likely,
            },
        ),
        Event::new(
            fixed,
            EventKind::SliceExtractCompleted {
                slice_name: "checkout".into(),
                source: "monolith".to_string(),
            },
        ),
    ];
    for event in &events {
        journal::append_batch(layout, std::slice::from_ref(event)).expect("append helper succeeds");
    }

    let raw = fs::read_to_string(journal_path(project.root())).expect("read journal");
    let lines: Vec<&str> = raw.lines().collect();
    assert_eq!(lines.len(), 2, "expected two JSON lines, got {}", lines.len());
    for line in &lines {
        let parsed: Map<String, Value> = serde_json::from_str(line).expect("each line is JSON");
        assert!(parsed.contains_key("timestamp"), "line missing timestamp: {line}");
        assert!(parsed.contains_key("event"), "line missing event id: {line}");
        assert!(parsed.contains_key("payload"), "line missing payload: {line}");
    }

    let values: Vec<Value> = lines.iter().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_journal_golden("agent-emit-helper.json", values);
}

// -- journal emit (source.* M1 events) -------------------------------

#[test]
fn emit_appends_one_line_per_event() {
    // The source events round-trip through the `journal emit` front
    // door: id + --payload deserialise into the closed taxonomy, the
    // CLI stamps the timestamp, and exactly one line lands per call.
    let project = Project::init();
    let cases: [(&str, &str, &str); 3] = [
        ("source.survey.completed", r#"{"source":"runtime","adapter":"captures"}"#, "adapter"),
        (
            "slice.extract.completed",
            r#"{"slice-name":"checkout","source":"runtime"}"#,
            "slice-name",
        ),
        (
            "source.execution.agent",
            r#"{"source":"runtime","adapter":"captures","operation":"survey"}"#,
            "operation",
        ),
    ];

    for (event_id, payload, _) in cases {
        specify_cmd()
            .current_dir(project.root())
            .args(["journal", "emit", event_id, "--payload", payload])
            .assert()
            .success();
    }

    let events = read_journal(project.root());
    assert_eq!(events.len(), 3, "expected one line per emit, got {}", events.len());

    assert_eq!(events[0]["event"], "source.survey.completed");
    assert_eq!(events[0]["payload"]["source"], "runtime");
    assert_eq!(events[0]["payload"]["adapter"], "captures");

    assert_eq!(events[1]["event"], "slice.extract.completed");
    assert_eq!(events[1]["payload"]["slice-name"], "checkout");

    assert_eq!(events[2]["event"], "source.execution.agent");
    assert_eq!(events[2]["payload"]["operation"], "survey");

    for event in &events {
        assert!(
            event["timestamp"].as_str().is_some(),
            "emit must stamp a timestamp, got:\n{event}"
        );
    }
}

#[test]
fn emit_appends_m3_build_merge() {
    // The build/merge lifecycle events and
    // `target.execution.agent` round-trip through the `journal emit`
    // front door with no new wiring — the closed taxonomy is the
    // payload schema, so id + --payload deserialise straight into the
    // existing variants.
    let project = Project::init();
    let cases: [(&str, Option<&str>); 7] = [
        ("slice.build.started", Some(r#"{"slice-name":"checkout"}"#)),
        ("slice.build.succeeded", Some(r#"{"slice-name":"checkout"}"#)),
        ("slice.build.failed", Some(r#"{"slice-name":"checkout","reason":"cargo-check-failed"}"#)),
        ("slice.merge.started", Some(r#"{"slice-name":"checkout"}"#)),
        ("slice.merge.succeeded", Some(r#"{"slice-name":"checkout"}"#)),
        ("slice.merge.failed", Some(r#"{"slice-name":"checkout","reason":"baseline-conflict"}"#)),
        ("target.execution.agent", Some(r#"{"slice":"checkout","target":"omnia"}"#)),
    ];

    for (event_id, payload) in cases {
        let mut cmd = specify_cmd();
        cmd.current_dir(project.root()).args(["journal", "emit", event_id]);
        if let Some(payload) = payload {
            cmd.args(["--payload", payload]);
        }
        cmd.assert().success();
    }

    let events = read_journal(project.root());
    assert_eq!(events.len(), 7, "expected one line per emit, got {}", events.len());
    let ids: Vec<&str> = events.iter().map(|e| e["event"].as_str().expect("event id")).collect();
    assert_eq!(
        ids,
        [
            "slice.build.started",
            "slice.build.succeeded",
            "slice.build.failed",
            "slice.merge.started",
            "slice.merge.succeeded",
            "slice.merge.failed",
            "target.execution.agent",
        ]
    );
    assert_eq!(events[2]["payload"]["reason"], "cargo-check-failed");
    assert_eq!(events[5]["payload"]["reason"], "baseline-conflict");
    assert_eq!(events[6]["payload"]["slice"], "checkout");
    assert_eq!(events[6]["payload"]["target"], "omnia");
}

#[test]
fn emit_m3_failed_requires_reason() {
    // A `*.failed` variant without its `reason` field fails the single
    // serde round-trip as `journal-emit-payload-schema`.
    let project = Project::init();
    let assert = specify_cmd()
        .current_dir(project.root())
        .args([
            "--format",
            "json",
            "journal",
            "emit",
            "slice.build.failed",
            "--payload",
            r#"{"slice-name":"checkout"}"#,
        ])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let actual = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(actual["error"], "journal-emit-payload-schema");
    assert!(
        !journal_path(project.root()).exists(),
        "a rejected emit must not append to the journal"
    );
}

#[test]
fn journal_emit_unknown_event_is_rejected() {
    let project = Project::init();
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "journal", "emit", "not.a.real.event", "--payload", "{}"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let actual = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(actual["error"], "journal-emit-unknown-event");
    assert!(
        !journal_path(project.root()).exists(),
        "a rejected emit must not append to the journal"
    );
}

#[test]
fn emit_incomplete_payload_rejected() {
    // A known event id whose payload omits a required field fails the
    // single serde round-trip as `journal-emit-payload-schema`.
    let project = Project::init();
    let assert = specify_cmd()
        .current_dir(project.root())
        .args([
            "--format",
            "json",
            "journal",
            "emit",
            "source.survey.completed",
            "--payload",
            r#"{"source":"runtime"}"#,
        ])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let actual = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(actual["error"], "journal-emit-payload-schema");
    assert!(
        !journal_path(project.root()).exists(),
        "a rejected emit must not append to the journal"
    );
}

#[test]
fn divergence_kebab_case_round_trip() {
    // Wire-format guard: snake_case lifecycle values are never
    // produced anywhere on disk (workflow §Wire format).
    for state in [Divergence::None, Divergence::Likely, Divergence::Accepted, Divergence::Rejected]
    {
        let rendered = serde_json::to_string(&state).expect("Divergence serialises");
        assert!(
            !rendered.contains('_'),
            "Divergence `{state:?}` must not contain `_` on the wire; got {rendered}"
        );
    }
}

// -- journal show (the journal read surface) ----------------------------

/// Seed three build/merge events through the `emit` front door so
/// `show` reads a journal written by the production append path.
fn seed_show_journal(project: &Project) {
    let cases: [(&str, &str); 3] = [
        ("slice.build.started", r#"{"slice-name":"checkout"}"#),
        ("slice.build.succeeded", r#"{"slice-name":"checkout"}"#),
        ("slice.merge.started", r#"{"slice-name":"checkout"}"#),
    ];
    for (event_id, payload) in cases {
        specify_cmd()
            .current_dir(project.root())
            .args(["journal", "emit", event_id, "--payload", payload])
            .assert()
            .success();
    }
}

#[test]
fn show_text_is_canonical_jsonl() {
    // Text mode replaces the eval probes' `jq` bridge: the lines it
    // prints are byte-identical to the on-disk journal lines.
    let project = Project::init();
    seed_show_journal(&project);

    let assert =
        specify_cmd().current_dir(project.root()).args(["journal", "show"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let on_disk = fs::read_to_string(journal_path(project.root())).expect("read journal");
    assert_eq!(stdout, on_disk, "show must emit the journal's own JSONL lines in append order");
}

#[test]
fn show_filter_and_limit() {
    let project = Project::init();
    seed_show_journal(&project);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["journal", "show", "--filter", "slice.build"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let ids: Vec<Value> = stdout
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("line is JSON")["event"].clone())
        .collect();
    assert_eq!(
        ids,
        vec![Value::from("slice.build.started"), Value::from("slice.build.succeeded")],
        "--filter must keep the dotted-prefix family only"
    );

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["journal", "show", "--filter", "slice.build", "--limit", "1"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "--limit must keep the most recent match only");
    let event: Value = serde_json::from_str(lines[0]).expect("line is JSON");
    assert_eq!(event["event"], "slice.build.succeeded", "the newest build event wins the tail");
}

#[test]
fn show_json_envelope() {
    let project = Project::init();
    seed_show_journal(&project);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "journal", "show", "--filter", "slice.merge."])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["count"], 1);
    assert_eq!(actual["events"][0]["event"], "slice.merge.started");
    assert_eq!(actual["events"][0]["payload"]["slice-name"], "checkout");
    assert!(
        actual["events"][0]["timestamp"].as_str().is_some(),
        "envelope events must carry the wire timestamp, got:\n{actual}"
    );
}

#[test]
fn show_missing_journal_is_empty() {
    // A read verb on a project with no journal succeeds with nothing
    // to print — probes can run it before any event has fired.
    let project = Project::init();

    let assert =
        specify_cmd().current_dir(project.root()).args(["journal", "show"]).assert().success();
    assert!(
        assert.get_output().stdout.is_empty(),
        "text mode on a missing journal must print nothing"
    );

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "journal", "show"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["count"], 0);
    assert_eq!(actual["events"], Value::Array(vec![]));
}
