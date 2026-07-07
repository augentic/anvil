//! Integration tests for the `specify plan *` surface: the CLI
//! orchestration verbs and the fan-in/fan-out reconciliation. CLI
//! submodules live under `workflow/`; the fan-in/out suite lives under
//! `plan/`. Pure plan-schema tests live in
//! `crates/workflow/tests/plan_schema.rs`. Shared helpers live in
//! [`common`]; the orchestration submodules pull their shared surface
//! in via [`support`].

mod support {
    //! Shared fixtures, seeds, and re-exports for the `workflow`
    //! integration suite.
    //!
    //! The suite is split across themed submodules grouped by `plan`
    //! command family (`validate`, `next`, `mutate`, `source_binding`,
    //! `transition`, `create`, `archive`, `authority`). Every
    //! submodule pulls its shared surface in with `use super::support::*;`,
    //! so the common imports, helpers, and plan seeds live here once.

    pub use std::fs;
    pub use std::path::PathBuf;
    pub use std::process::Command as ProcessCommand;

    pub use serde_json::Value;
    pub use specify_workflow::change::{Plan, Status};
    pub use tempfile::{TempDir, tempdir};

    pub use crate::common::{
        Project, assert_golden_at, omnia_component, parse_stderr, parse_stdout, repo_root,
        specify_cmd, stage_store_component,
    };

    pub fn plan_fixtures() -> PathBuf {
        repo_root().join("tests/fixtures/plan")
    }

    pub fn assert_golden(name: &str, actual: Value) {
        assert_golden_at(&plan_fixtures(), name, actual);
    }

    // -- setup helpers -----------------------------------------------------

    /// Load and parse the project's `plan.yaml` into the in-memory model.
    /// Used by setup helpers (and tests) that must assert a write actually
    /// landed rather than trusting a bare `.assert().success()`.
    pub fn load_plan(project: &Project) -> Plan {
        Plan::load(&project.plan_path()).unwrap_or_else(|err| panic!("load plan.yaml: {err}"))
    }

    /// Run `specify plan add <name>` as a setup step, asserting BOTH that
    /// it exits 0 AND that the entry actually landed in `plan.yaml` as a
    /// `pending` row. Most call sites previously asserted only `.success()`,
    /// so a silent regression in the plan writer would have slipped past
    /// the setup and surfaced as a confusing failure in the assertion under
    /// test.
    pub fn add_pending_entry(project: &Project, name: &str) {
        add_entry_with(project, name, &[]);
    }

    /// [`add_pending_entry`] with extra `plan add` flags (e.g. `--sources
    /// <key>=<lead>`). Asserts the entry is present and `pending` after the
    /// write so the binding-shaping tests start from a verified state.
    pub fn add_entry_with(project: &Project, name: &str, extra: &[&str]) {
        let mut args = vec!["plan", "add", name];
        args.extend_from_slice(extra);
        specify_cmd().current_dir(project.root()).args(&args).assert().success();

        let plan = load_plan(project);
        let entry = plan.entries.iter().find(|e| e.name == name).unwrap_or_else(|| {
            panic!("`plan add {name}` did not append an entry; entries: {:?}", plan.entries)
        });
        assert_eq!(
            entry.status,
            Status::Pending,
            "`plan add {name}` must land a pending entry, got {:?}",
            entry.status
        );
    }

    // -- test seeds --------------------------------------------------------

    pub const CLEAN_PLAN: &str = "\
name: demo
slices:
  - name: a
    project: default
    status: pending
  - name: b
    project: default
    status: pending
    depends-on: [a]
";

    pub const DUPLICATE_NAME_PLAN: &str = "\
name: demo
slices:
  - name: foo
    project: default
    status: pending
  - name: foo
    project: default
    status: pending
";

    pub const A_DONE_B_PENDING: &str = "\
name: demo
slices:
  - name: a
    project: default
    status: done
  - name: b
    project: default
    status: pending
";

    pub const A_IN_PROGRESS: &str = "\
name: demo
slices:
  - name: a
    project: default
    status: in-progress
";

    /// One pending entry. Shared by the `mutate` (amend-on-missing) and
    /// `transition` submodules.
    pub const SINGLE_PENDING: &str = "\
name: demo
slices:
  - name: foo
    project: default
    status: pending
";

    pub const ALL_DONE: &str = "\
name: demo
slices:
  - name: a
    project: default
    status: done
  - name: b
    project: default
    status: done
";
}

mod validate {
    //! `specify plan validate` CLI tests: base shape rules, the
    //! registry-shape hook, the planning-path smoke, and the surviving
    //! health diagnostics (`cycle-in-depends-on`, `orphan-source`,
    //! `stale-workspace-clone`, `topology-cache-stale`).

    use super::support::*;

    // -- base shape rules --------------------------------------------------

    #[test]
    fn plan_validate_clean_json() {
        let project = Project::init();
        project.seed_plan(CLEAN_PLAN);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "validate"])
            .assert()
            .success();
        assert_eq!(assert.get_output().status.code(), Some(0));

        // The wire shape is the neutral `DiagnosticReport` envelope:
        // `{ version, summary, findings }`. A clean plan carries no
        // findings and an all-zero summary; the exit code (0) signals pass.
        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(actual["version"], 1);
        assert_eq!(actual["findings"], Value::Array(vec![]));
        assert_golden("validate-clean.json", actual);
    }

    #[test]
    fn plan_validate_tolerates_in_progress() {
        // Transient window: `specify change transition <name> in-progress`
        // can run a moment before `.specify/slices/<name>/` exists.
        // `specify plan validate` must surface a *warning* (not an
        // error) so `passed == true` and skills don't stall on start-up.
        let project = Project::init();
        project.seed_plan(A_IN_PROGRESS);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "validate"])
            .assert()
            .success();
        assert_eq!(
            assert.get_output().status.code(),
            Some(0),
            "warning-only validate must exit 0 (EXIT_SUCCESS)"
        );

        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        let findings = actual["findings"].as_array().expect("findings array");
        let matching: Vec<&Value> = findings
            .iter()
            .filter(|r| r["rule-id"] == "missing-slice-dir-for-in-progress")
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "expected exactly one missing-slice-dir-for-in-progress finding, got: {findings:#?}"
        );
        // A missing-slice-dir-for-in-progress finding is a non-blocking
        // `suggestion`, so exit 0 above already proves it does not gate.
        assert_eq!(matching[0]["severity"], "suggestion");
        assert_eq!(matching[0]["slice"], "a");
    }

    #[test]
    fn plan_validate_with_errors_json() {
        let project = Project::init();
        project.seed_plan(DUPLICATE_NAME_PLAN);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "validate"])
            .assert()
            .failure();
        assert_eq!(
            assert.get_output().status.code(),
            Some(2),
            "duplicate-name must exit 2 (EXIT_VALIDATION_FAILED)"
        );

        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        let findings = actual["findings"].as_array().expect("findings array");
        assert!(
            findings
                .iter()
                .any(|r| r["rule-id"] == "duplicate-name" && r["severity"] == "important"),
            "expected a blocking duplicate-name finding, got: {findings:#?}"
        );
        assert_golden("validate-duplicate-name.json", actual);
    }

    // -- registry-shape hook ----------------------------------------------

    /// `specify plan validate` surfaces a malformed `registry.yaml`
    /// alongside plan validation results — the shape-validation hook
    /// complementing the dedicated `specify registry validate`
    /// verb.
    #[test]
    fn plan_validate_surfaces_registry_errors() {
        let project = Project::init();
        // Seed a minimal, structurally-valid plan so `change plan validate`
        // doesn't exit on the plan load itself.
        project.seed_plan("name: demo\nslices: []\n");
        // Then stomp the registry with an illegal version.
        fs::write(project.root().join("registry.yaml"), "version: 2\nprojects: []\n")
            .expect("write bad registry");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "validate"])
            .assert()
            .failure();
        let value = parse_stdout(&assert.get_output().stdout, project.root());
        let findings = value["findings"].as_array().expect("findings array");
        let registry_findings: Vec<&Value> =
            findings.iter().filter(|r| r["rule-id"] == "registry-shape").collect();
        assert_eq!(
            registry_findings.len(),
            1,
            "expected one registry-shape finding, got: {findings:#?}"
        );
        assert_eq!(registry_findings[0]["severity"], "important");
        let msg = registry_findings[0]["impact"].as_str().expect("impact string");
        assert!(msg.contains("version"), "expected version in impact, got: {msg}");
    }

    // ---- planning-path workspace smoke — planning-path smoke (Stage A/B, manifest, Layer 2) ----

    #[test]
    fn planning_stage_ab_brief_and_validate() {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "create", "planning-path", "--source", "app=typescript:."])
            .assert()
            .success();

        // Don't trust the bare exit code: assert `plan create` actually
        // wrote the named plan with the requested source binding before we
        // validate it.
        let plan = load_plan(&project);
        assert_eq!(plan.name, "planning-path", "create must persist the plan name");
        assert!(plan.sources.contains_key("app"), "create must persist the `app` source binding");

        specify_cmd().current_dir(project.root()).args(["plan", "validate"]).assert().success();
    }

    // ---- specify plan validate health diagnostics (plan validate health diagnostics) ----
    //
    // `plan validate` carries the three surviving health diagnostics
    // (`cycle-in-depends-on`, `orphan-source`,
    // `stale-workspace-clone`) alongside its base shape rules. The
    // `unreachable-entry` diagnostic retired in source/target adapter split alongside the
    // per-entry `failed`/`skipped` states it relied on.

    fn init_omnia_project(tmp: &TempDir) {
        specify_cmd()
            .current_dir(tmp.path())
            .args(["init"])
            .arg(omnia_component())
            .args(["--name", "demo"])
            .assert()
            .success();
    }

    #[test]
    fn validate_reports_all_health_diagnostics() {
        let tmp = tempdir().unwrap();
        init_omnia_project(&tmp);

        // Authoring a plan that intentionally exercises all four doctor
        // checks at once. We hand-write `plan.yaml` because the CLI's own
        // `plan create` path enforces validation at write time and would
        // refuse the cycle / unknown-source cases below.
        fs::write(
            tmp.path().join("plan.yaml"),
            "name: demo\n\
             sources:\n\
             \x20\x20monolith:\n\
             \x20\x20\x20\x20adapter: typescript\n\
             \x20\x20\x20\x20path: /tmp/legacy\n\
             \x20\x20orphaned:\n\
             \x20\x20\x20\x20adapter: typescript\n\
             \x20\x20\x20\x20path: /tmp/elsewhere\n\
             slices:\n\
             \x20\x20- name: cyclic-a\n\
             \x20\x20\x20\x20project: alpha\n\
             \x20\x20\x20\x20status: pending\n\
             \x20\x20\x20\x20depends-on: [cyclic-b]\n\
             \x20\x20- name: cyclic-b\n\
             \x20\x20\x20\x20project: alpha\n\
             \x20\x20\x20\x20status: pending\n\
             \x20\x20\x20\x20depends-on: [cyclic-a]\n\
             \x20\x20- name: orphaned-source-user\n\
             \x20\x20\x20\x20project: alpha\n\
             \x20\x20\x20\x20status: pending\n\
             \x20\x20\x20\x20sources: [monolith]\n",
        )
        .unwrap();

        // Hand-write a registry at the repo root, so we can exercise
        // stale-clone with a deterministic fixture: a clone slot whose
        // origin remote disagrees with the registry.
        fs::write(
            tmp.path().join("registry.yaml"),
            "version: 1\n\
             projects:\n\
             \x20\x20- name: alpha\n\
             \x20\x20\x20\x20url: git@github.com:org/alpha.git\n\
             \x20\x20\x20\x20adapter: omnia@1.0.0\n",
        )
        .unwrap();
        let slot = tmp.path().join("workspace/alpha");
        fs::create_dir_all(&slot).unwrap();
        let init = ProcessCommand::new("git").arg("-C").arg(&slot).arg("init").output().unwrap();
        assert!(
            init.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&init.stderr)
        );
        let remote = ProcessCommand::new("git")
            .arg("-C")
            .arg(&slot)
            .args(["remote", "add", "origin", "git@github.com:old/alpha.git"])
            .output()
            .unwrap();
        assert!(
            remote.status.success(),
            "git remote add failed: {}",
            String::from_utf8_lossy(&remote.stderr)
        );

        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "plan", "validate"])
            .assert();
        let output = assert.get_output();
        let stdout = String::from_utf8(output.stdout.clone()).expect("utf8");
        let value: Value = serde_json::from_str(&stdout).expect("stdout is JSON");

        let findings = value["findings"].as_array().expect("findings array");
        assert!(!findings.is_empty(), "validate with broken plan must surface findings: {value}");
        let codes: Vec<&str> = findings.iter().filter_map(|r| r["rule-id"].as_str()).collect();

        for expected in ["cycle-in-depends-on", "orphan-source", "stale-workspace-clone"] {
            assert!(
                codes.contains(&expected),
                "validate must emit `{expected}` for the synthetic fixture; saw: {codes:?}"
            );
        }

        // Exit code must be ValidationFailed (2) because the cycle is
        // error-severity.
        let code = output.status.code().expect("exit code");
        assert_eq!(code, 2, "error-severity diagnostics must yield exit 2, got {code}");
    }

    #[test]
    fn validate_reports_topology_cache_stale() {
        // A slot's `project.yaml` is the authored home for its
        // facets; `.specify/topology.lock` is the derived projection. When a
        // materialised slot drifts from the committed cache, `plan validate`
        // emits the warning-only `topology-cache-stale` diagnostic whose fix
        // is `specify workspace sync`. (Replaces the former
        // `adapter-mismatch-workspace` check.)
        let tmp = tempdir().unwrap();
        init_omnia_project(&tmp);

        fs::write(
            tmp.path().join("plan.yaml"),
            "name: demo\n\
             slices:\n\
             \x20\x20- name: alpha-slice\n\
             \x20\x20\x20\x20status: pending\n\
             \x20\x20\x20\x20project: alpha\n",
        )
        .unwrap();
        fs::write(
            tmp.path().join("registry.yaml"),
            "version: 1\n\
             projects:\n\
             \x20\x20- name: alpha\n\
             \x20\x20\x20\x20url: git@github.com:org/alpha.git\n",
        )
        .unwrap();

        // Materialise the slot with a resolvable adapter and an authored
        // description, then seed a topology.lock whose entry disagrees.
        let slot_specify = tmp.path().join("workspace/alpha/.specify");
        fs::create_dir_all(&slot_specify).unwrap();
        fs::write(
            slot_specify.join("project.yaml"),
            "name: alpha\nadapter: omnia@1.0.0\ndescription: Fresh description\n",
        )
        .unwrap();
        stage_store_component("omnia", "1.0.0");

        fs::write(
            tmp.path().join(".specify/topology.lock"),
            "version: 1\n\
             projects:\n\
             \x20\x20- name: alpha\n\
             \x20\x20\x20\x20target: omnia@1.0.0\n\
             \x20\x20\x20\x20description: Stale description\n",
        )
        .unwrap();

        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "plan", "validate"])
            .assert();
        let value: Value = serde_json::from_str(
            &String::from_utf8(assert.get_output().stdout.clone()).expect("utf8"),
        )
        .expect("stdout is JSON");
        let findings = value["findings"].as_array().expect("findings array");
        let stale: Vec<&Value> =
            findings.iter().filter(|r| r["rule-id"] == "topology-cache-stale").collect();
        assert_eq!(stale.len(), 1, "expected one topology-cache-stale finding, got: {findings:#?}");
        assert_eq!(stale[0]["severity"], "suggestion");
        let msg = stale[0]["impact"].as_str().expect("impact string");
        assert!(msg.contains("alpha"), "expected slot name in impact, got: {msg}");
        assert!(msg.contains("workspace sync"), "expected the fix command in impact, got: {msg}");
        assert_eq!(
            assert.get_output().status.code(),
            Some(0),
            "stale cache is a suggestion-only finding, so validate must exit 0"
        );
    }

    #[test]
    fn plan_validate_payloads_round_trip_typed() {
        let tmp = tempdir().unwrap();
        init_omnia_project(&tmp);

        // Minimal plan that exercises just the cycle and orphan-source
        // checks — enough to confirm the typed payload deserialises
        // cleanly.
        fs::write(
            tmp.path().join("plan.yaml"),
            "name: demo\n\
             sources:\n\
             \x20\x20orphan-key:\n\
             \x20\x20\x20\x20adapter: typescript\n\
             \x20\x20\x20\x20path: /tmp/somewhere\n\
             slices:\n\
             \x20\x20- name: cyc-a\n\
             \x20\x20\x20\x20project: default\n\
             \x20\x20\x20\x20status: pending\n\
             \x20\x20\x20\x20depends-on: [cyc-b]\n\
             \x20\x20- name: cyc-b\n\
             \x20\x20\x20\x20project: default\n\
             \x20\x20\x20\x20status: pending\n\
             \x20\x20\x20\x20depends-on: [cyc-a]\n",
        )
        .unwrap();

        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "plan", "validate"])
            .assert();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
        let value: Value = serde_json::from_str(&stdout).expect("stdout is JSON");
        let findings = value["findings"].as_array().expect("findings array");

        // The health checks carry their machine-readable payload on the
        // neutral diagnostic's structured evidence (`evidence.data`) rather
        // than a bespoke `data` field — unified onto the currency without
        // loss.
        let cycle = findings
            .iter()
            .find(|d| d["rule-id"] == "cycle-in-depends-on")
            .expect("expected cycle-in-depends-on diagnostic");
        assert_eq!(cycle["evidence"]["kind"], "structured");
        let cycle_path =
            cycle["evidence"]["data"]["cycle"].as_array().expect("cycle path is array");
        let names: Vec<String> =
            cycle_path.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        assert_eq!(
            names,
            vec!["cyc-a".to_string(), "cyc-b".to_string(), "cyc-a".to_string()],
            "cycle path must close on the first node"
        );

        let orphan = findings
            .iter()
            .find(|d| d["rule-id"] == "orphan-source")
            .expect("expected orphan-source diagnostic");
        assert_eq!(orphan["evidence"]["kind"], "structured");
        assert_eq!(orphan["evidence"]["data"]["key"], "orphan-key");
        assert_eq!(orphan["severity"], "suggestion");
    }

    #[test]
    fn plan_validate_healthy_exits_zero() {
        let tmp = tempdir().unwrap();
        init_omnia_project(&tmp);

        specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "plan", "create", "demo"])
            .assert()
            .success();

        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "plan", "validate"])
            .assert()
            .success();
        let value: Value = serde_json::from_slice(&assert.get_output().stdout).expect("json");
        assert_eq!(
            value["findings"].as_array().unwrap().len(),
            0,
            "empty plan must emit zero findings: {value}"
        );
    }
}

mod next {
    //! `specify plan next` CLI tests.

    use super::support::*;

    #[test]
    fn plan_next_picks_first_pending_text() {
        let project = Project::init();
        project.seed_plan(A_DONE_B_PENDING);

        let assert =
            specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();
        let stdout = std::str::from_utf8(&assert.get_output().stdout).expect("utf8");
        assert_eq!(stdout, "b\n", "text next should be bare '<name>\\n', got: {stdout:?}");
    }

    #[test]
    fn plan_next_picks_first_pending_json() {
        let project = Project::init();
        project.seed_plan(A_DONE_B_PENDING);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "next"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(actual["next"], "b");
        assert_eq!(actual["reason"], Value::Null);
        assert_eq!(actual["active"], Value::Null);
        assert_eq!(actual["project"], "default", "project should match seeded value");
        assert_eq!(actual["description"], Value::Null, "description should be present");
        assert!(
            actual.get("sources").is_some(),
            "sources field should be present in plan next response"
        );
        assert_golden("next-first-pending.json", actual);
    }

    #[test]
    fn plan_next_journals_entry_advanced() {
        // `plan next` is the sole writer of per-entry `in-progress`; the
        // matching `plan.entry.advanced` event fires only on the write.
        let project = Project::init();
        project.seed_plan(A_DONE_B_PENDING);

        specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();

        let journal = project.root().join(".specify").join("journal.jsonl");
        let raw = fs::read_to_string(&journal).expect("read journal.jsonl");
        let lines: Vec<&str> = raw.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 1, "exactly one event per fresh advance, got:\n{raw}");
        assert!(
            lines[0].contains(r#""event":"plan.entry.advanced""#),
            "advance must journal plan.entry.advanced, got:\n{}",
            lines[0]
        );
        assert!(lines[0].contains(r#""plan-name":"demo""#), "got:\n{}", lines[0]);
        assert!(lines[0].contains(r#""slice-name":"b""#), "got:\n{}", lines[0]);

        // Re-running `plan next` returns the active entry unchanged — no
        // second advance event, so probes can read "did not advance"
        // from the journal window.
        specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();
        let raw_after = fs::read_to_string(&journal).expect("read journal.jsonl");
        assert_eq!(
            raw_after.lines().filter(|l| !l.is_empty()).count(),
            1,
            "returning the active entry must not append a second event, got:\n{raw_after}"
        );
    }

    #[test]
    fn plan_next_drained_no_journal() {
        let project = Project::init();
        project.seed_plan(ALL_DONE);

        specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();
        assert!(
            !project.root().join(".specify").join("journal.jsonl").exists(),
            "a drained plan must not journal plan.entry.advanced"
        );
    }

    #[test]
    fn plan_next_reports_in_progress() {
        let project = Project::init();
        project.seed_plan(A_IN_PROGRESS);

        let text =
            specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();
        let stdout = std::str::from_utf8(&text.get_output().stdout).expect("utf8");
        assert!(stdout.contains('a'), "text output should mention 'a': {stdout:?}");

        let json = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "next"])
            .assert()
            .success();
        let actual = parse_stdout(&json.get_output().stdout, project.root());
        assert_eq!(actual["next"], Value::Null);
        assert_eq!(actual["reason"], "in-progress");
        assert_eq!(actual["active"], "a");
        assert_golden("next-in-progress.json", actual);
    }

    #[test]
    fn plan_next_all_done_text() {
        let project = Project::init();
        project.seed_plan(ALL_DONE);

        let text =
            specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();
        let stdout = std::str::from_utf8(&text.get_output().stdout).expect("utf8");
        assert!(stdout.contains("drained"), "drained text should mention drained, got: {stdout:?}");

        let json = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "next"])
            .assert()
            .success();
        let actual = parse_stdout(&json.get_output().stdout, project.root());
        assert_eq!(actual["reason"], "drained");
        assert_eq!(actual["next"], Value::Null);
        assert_eq!(actual["active"], Value::Null);
        assert_golden("next-all-done.json", actual);
    }
}

mod status {
    //! `specify plan status` CLI tests — the read-only next-action
    //! projection. The projection matrix is unit-tested in
    //! `crates/workflow/src/change/plan/core/status/tests.rs`; this suite
    //! pins the verb's wire shapes and its read-only contract.

    use super::support::*;

    const APPROVED_IN_PROGRESS: &str = "\
name: demo
lifecycle: approved
slices:
  - name: a
    project: default
    status: in-progress
";

    const APPROVED_ALL_DONE: &str = "\
name: demo
lifecycle: approved
slices:
  - name: a
    project: default
    status: done
";

    /// Seed `<slice>/metadata.yaml` with the given lifecycle status.
    fn seed_slice(project: &Project, name: &str, status: &str) {
        let slice_dir = project.slices_dir().join(name);
        fs::create_dir_all(&slice_dir).expect("mkdir slice");
        fs::write(
            slice_dir.join("metadata.yaml"),
            format!("target: omnia@1.0.0\nstatus: {status}\n"),
        )
        .expect("write metadata.yaml");
    }

    /// Append raw journal lines (the projection only reads; tests own the
    /// fixture file).
    fn seed_journal(project: &Project, lines: &[&str]) {
        let path = project.root().join(".specify").join("journal.jsonl");
        let mut body = lines.join("\n");
        body.push('\n');
        fs::write(path, body).expect("write journal.jsonl");
    }

    #[test]
    fn status_pending_plan_stops() {
        let project = Project::init();
        project.seed_plan(A_IN_PROGRESS);

        let assert =
            specify_cmd().current_dir(project.root()).args(["plan", "status"]).assert().success();
        let stdout = std::str::from_utf8(&assert.get_output().stdout).expect("utf8");
        assert!(
            stdout.contains("stop: plan-not-approved"),
            "unapproved plan must stop, got: {stdout:?}"
        );
        assert!(stdout.contains("hint:"), "stop must carry a hint line, got: {stdout:?}");
    }

    #[test]
    fn status_active_refine_json() {
        let project = Project::init();
        project.seed_plan(APPROVED_IN_PROGRESS);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "status"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(actual["next-action"], "refine a");
        assert_eq!(actual["action"], "refine");
        assert_eq!(actual["active"], "a");
        assert_golden("status-refine.json", actual);
    }

    #[test]
    fn status_build_failure_stops() {
        let project = Project::init();
        project.seed_plan(APPROVED_IN_PROGRESS);
        seed_slice(&project, "a", "refined");
        seed_journal(
            &project,
            &[
                r#"{"timestamp":"2026-01-01T00:00:00Z","event":"plan.entry.advanced","payload":{"plan-name":"demo","slice-name":"a"}}"#,
                r#"{"timestamp":"2026-01-01T00:01:00Z","event":"slice.build.failed","payload":{"slice-name":"a","reason":"exhausted repair budget"}}"#,
            ],
        );

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "status"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(actual["next-action"], "stop build-failed");
        assert_eq!(actual["stop"]["reason"], "build-failed");
        assert_eq!(actual["stop"]["detail"], "exhausted repair budget");
        assert_eq!(actual["resume"], "/spec:build a", "RM-15 re-entry point");
        assert_golden("status-build-failed.json", actual);

        let text =
            specify_cmd().current_dir(project.root()).args(["plan", "status"]).assert().success();
        let stdout = std::str::from_utf8(&text.get_output().stdout).expect("utf8");
        assert!(stdout.contains("stop: build-failed"), "got: {stdout:?}");
        assert!(stdout.contains("  slice: a"), "stop block must name the slice, got: {stdout:?}");
        assert!(stdout.contains("  detail: exhausted repair budget"), "got: {stdout:?}");
        assert!(stdout.contains("resume: /spec:build a"), "got: {stdout:?}");
    }

    #[test]
    fn status_built_slice_dispatches_merge() {
        let project = Project::init();
        project.seed_plan(APPROVED_IN_PROGRESS);
        seed_slice(&project, "a", "built");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "status"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(actual["next-action"], "merge a");
    }

    #[test]
    fn status_drained_renders_finalize_hint() {
        let project = Project::init();
        project.seed_plan(APPROVED_ALL_DONE);

        let assert =
            specify_cmd().current_dir(project.root()).args(["plan", "status"]).assert().success();
        let stdout = std::str::from_utf8(&assert.get_output().stdout).expect("utf8");
        assert!(
            stdout.contains("drained — run /spec:finalize demo"),
            "drained must render the literal stop-conditions string, got: {stdout:?}"
        );
    }

    #[test]
    fn status_is_read_only() {
        // The projection must not advance the plan, write the journal, or
        // touch slice state — `plan next` stays the only in-progress writer.
        let project = Project::init();
        project.seed_plan(A_DONE_B_PENDING);
        let plan_before = fs::read_to_string(project.plan_path()).expect("read plan");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "status"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(
            actual["next-action"], "stop plan-not-approved",
            "seeded plan has no lifecycle stamp"
        );

        let plan_after = fs::read_to_string(project.plan_path()).expect("read plan");
        assert_eq!(plan_before, plan_after, "plan status must not write plan.yaml");
        assert!(
            !project.root().join(".specify").join("journal.jsonl").exists(),
            "plan status must not journal"
        );
    }
}

mod mutate {
    //! `specify plan {add,remove,amend}` CLI tests — the L1.J write-side
    //! commands.

    use super::support::*;

    const EMPTY_PLAN: &str = "\
name: demo
slices: []
";

    const WITH_DESCRIPTION: &str = "\
name: demo
slices:
  - name: foo
    project: default
    status: pending
    description: original
";

    // -- plan add ---------------------------------------------------------

    #[test]
    fn plan_add_appends_pending_entry_json() {
        let project = Project::init();
        project.seed_plan(EMPTY_PLAN);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "add", "foo"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());

        assert_eq!(actual["action"], "create");
        assert_eq!(actual["entry"]["name"], "foo");
        assert_eq!(actual["entry"]["status"], "pending");
        assert_eq!(actual["entry"]["status-reason"], Value::Null);
        assert_eq!(actual["plan"]["name"], "demo");

        // Parse the persisted plan and assert the entry's fields rather than
        // substring-matching the raw YAML.
        let plan = load_plan(&project);
        let entry = plan
            .entries
            .iter()
            .find(|e| e.name == "foo")
            .expect("saved plan must contain entry `foo`");
        assert_eq!(entry.status, Status::Pending, "saved entry must be pending");

        assert_golden("create-foo.json", actual);
    }

    #[test]
    fn plan_add_rejects_duplicate_name_text() {
        let project = Project::init();
        project.seed_plan(EMPTY_PLAN);

        add_pending_entry(&project, "foo");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["plan", "add", "foo"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(1));
        let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8 stderr");
        assert!(
            stderr.contains("already contains a change"),
            "stderr should flag duplicate, got: {stderr:?}"
        );
    }

    #[test]
    fn plan_add_rejects_invalid_name() {
        let project = Project::init();
        project.seed_plan(EMPTY_PLAN);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["plan", "add", "NotKebab"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(1));

        let saved = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
        assert!(!saved.contains("NotKebab"), "invalid name must not land in the plan:\n{saved}");
    }

    // -- plan remove ------------------------------------------------------

    #[test]
    fn plan_remove_drops_pending_entry() {
        let project = Project::init();
        project.seed_plan(
            "\
name: demo
slices:
  - name: a
    project: default
    status: pending
  - name: b
    project: default
    status: pending
",
        );

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "remove", "a"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(actual["action"], "remove");
        assert_eq!(actual["entry"]["name"], "a");

        let saved = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
        assert!(!saved.contains("name: a"), "removed entry must not remain:\n{saved}");
        assert!(saved.contains("name: b"), "other entry must remain:\n{saved}");
    }

    #[test]
    fn plan_remove_refuses_when_depended_on() {
        let project = Project::init();
        project.seed_plan(
            "\
name: demo
slices:
  - name: a
    project: default
    status: pending
  - name: b
    project: default
    status: pending
    depends-on: [a]
",
        );

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["plan", "remove", "a"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8 stderr");
        assert!(
            stderr.contains("plan-remove-entry-referenced"),
            "stderr should name the validation code, got: {stderr:?}"
        );
    }

    // -- plan amend -------------------------------------------------------

    #[test]
    fn plan_amend_replaces_depends_on() {
        let project = Project::init();
        project.seed_plan(
            "\
name: demo
slices:
  - name: a
    project: default
    status: done
  - name: b
    project: default
    status: done
  - name: foo
    project: default
    status: pending
    depends-on: [a]
",
        );

        let assert = specify_cmd()
            .current_dir(project.root())
            .args([
                "--format",
                "json",
                "plan",
                "amend",
                "foo",
                "--depends-on",
                "a",
                "--depends-on",
                "b",
            ])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(actual["action"], "amend");
        assert_eq!(actual["entry"]["name"], "foo");
        let deps = actual["entry"]["depends-on"].as_array().expect("deps array");
        let names: Vec<&str> = deps.iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(names, ["a", "b"]);

        assert_golden("amend-replace-depends-on.json", actual);

        // Assert the persisted `depends-on` as a parsed list, not a pair of
        // raw-YAML substring probes.
        let plan = load_plan(&project);
        let entry =
            plan.entries.iter().find(|e| e.name == "foo").expect("amended entry must be present");
        assert_eq!(
            entry.depends_on,
            vec!["a", "b"],
            "amended depends-on must round-trip to disk in order"
        );
    }

    #[test]
    fn plan_amend_clears_description() {
        let project = Project::init();
        project.seed_plan(WITH_DESCRIPTION);

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "amend", "foo", "--description", ""])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read");
        assert!(
            !saved.contains("description: original"),
            "original description should be gone:\n{saved}"
        );
    }

    #[test]
    fn plan_amend_leaves_field_alone() {
        let project = Project::init();
        project.seed_plan(WITH_DESCRIPTION);

        // --depends-on (clear) but no --description; description must stay.
        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "amend", "foo", "--depends-on"])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read");
        assert!(
            saved.contains("description: original"),
            "description should be preserved:\n{saved}"
        );
    }

    #[test]
    fn plan_amend_on_missing_entry_fails() {
        let project = Project::init();
        project.seed_plan(SINGLE_PENDING);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["plan", "amend", "nope", "--description", "x"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(1));
        let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8");
        assert!(
            stderr.contains("no slice named"),
            "stderr should mention missing change, got: {stderr:?}"
        );
    }
}

mod source_binding {
    //! Per-slice source binding flag shape on `plan add` / `plan amend`.
    //!
    //! The binding flag uses the `<key>=<lead>` wire form, accepting the bare
    //! `<key>` shorthand only as sugar for `{ source, lead: <slice.name> }`
    //! per workflow §`Slice.sources`.

    use super::support::*;

    const W11_PLAN: &str = "\
name: w11
sources:
  intent:
    adapter: intent
    value: \"Demo intent value.\"
  identity-design-notes:
    adapter: documentation
    path: ./docs
slices: []
";

    #[test]
    fn plan_add_structured_sources_round_trips() {
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        specify_cmd()
            .current_dir(project.root())
            .args([
                "--format",
                "json",
                "plan",
                "add",
                "foo",
                "--sources",
                "identity-design-notes=user-registration",
            ])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            saved.contains("source: identity-design-notes")
                && saved.contains("lead: user-registration"),
            "structured form must round-trip to disk:\n{saved}"
        );
    }

    #[test]
    fn plan_add_bare_source_round_trips() {
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        // Slice name `add-search-filter`; bare `--sources intent` is
        // sugar for `{ source: intent, lead: add-search-filter }`.
        specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "add", "add-search-filter", "--sources", "intent"])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        // Bare form must appear on disk as the YAML scalar `intent`,
        // not the structured `{ source, lead }` mapping.
        assert!(
            saved.contains("  - intent"),
            "bare shorthand must round-trip to the unquoted scalar form:\n{saved}"
        );
        assert!(
            !saved.contains("lead: add-search-filter"),
            "lead=slice.name must collapse to bare form:\n{saved}"
        );
    }

    #[test]
    fn plan_add_structured_lead_differs() {
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        specify_cmd()
            .current_dir(project.root())
            .args([
                "--format",
                "json",
                "plan",
                "add",
                "foo",
                "--sources",
                "intent=different-candidate",
            ])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            saved.contains("lead: different-candidate"),
            "structured form must stay structured when lead != slice.name:\n{saved}"
        );
    }

    #[test]
    fn add_rejects_dangling_equals() {
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "add", "foo", "--sources", "intent="])
            .assert()
            .failure();
        let code = assert.get_output().status.code().expect("exit code");
        assert_eq!(code, 2, "malformed --sources must exit 2 (argument error), got {code}");
    }

    #[test]
    fn plan_amend_add_source_appends_binding() {
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        add_entry_with(&project, "foo", &["--sources", "intent"]);

        specify_cmd()
            .current_dir(project.root())
            .args([
                "plan",
                "amend",
                "foo",
                "--add-source",
                "identity-design-notes=user-registration",
            ])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            saved.contains("source: identity-design-notes"),
            "amend --add-source must append the binding:\n{saved}"
        );
    }

    #[test]
    fn plan_amend_remove_source_drops_binding() {
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        add_entry_with(
            &project,
            "foo",
            &["--sources", "intent", "--sources", "identity-design-notes=foo"],
        );

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "amend", "foo", "--remove-source", "intent"])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            !saved.contains("- intent"),
            "amend --remove-source must drop the binding:\n{saved}"
        );
        assert!(
            saved.contains("identity-design-notes"),
            "non-targeted bindings must remain:\n{saved}"
        );
    }

    #[test]
    fn amend_add_source_duplicate_key_rejected() {
        // A slice binds at most one lead per source key — a duplicate would
        // silently overwrite `evidence/<source>.yaml` at refine time.
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        add_entry_with(&project, "foo", &["--sources", "identity-design-notes=user-registration"]);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args([
                "--format",
                "json",
                "plan",
                "amend",
                "foo",
                "--add-source",
                "identity-design-notes=password-reset",
            ])
            .assert()
            .failure();
        let code = assert.get_output().status.code().expect("exit code");
        assert_eq!(code, 2, "duplicate source key must exit 2 (validation), got {code}");
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "duplicate-source-key");

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            !saved.contains("password-reset"),
            "rejected --add-source must leave the plan unwritten:\n{saved}"
        );
    }

    #[test]
    fn amend_sources_duplicate_key_rejected() {
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        add_entry_with(&project, "foo", &["--sources", "intent"]);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args([
                "--format",
                "json",
                "plan",
                "amend",
                "foo",
                "--sources",
                "identity-design-notes=lead-a",
                "--sources",
                "identity-design-notes=lead-b",
            ])
            .assert()
            .failure();
        // The wholesale replacement routes through `Plan::amend`'s
        // validate-and-rollback gate, so the generic amend discriminant
        // carries the duplicate-source-key impact text.
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "plan-amend-validation-failed");
        assert!(
            stderr["message"].as_str().expect("message").contains("more than once"),
            "message must name the duplicate-key violation, got: {stderr}"
        );

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            !saved.contains("lead-a"),
            "rejected --sources must leave the plan unwritten:\n{saved}"
        );
    }

    #[test]
    fn amend_sources_resize_passes() {
        // Same-source re-sizing — replacing the lead bound under an
        // existing key — stays legal; only duplicate keys are rejected.
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        add_entry_with(&project, "foo", &["--sources", "identity-design-notes=user-registration"]);

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "amend", "foo", "--sources", "identity-design-notes=password-reset"])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            saved.contains("lead: password-reset") && !saved.contains("user-registration"),
            "re-sizing must replace the bound lead:\n{saved}"
        );
    }

    #[test]
    fn amend_remove_source_unknown_key_errors() {
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        add_entry_with(&project, "foo", &["--sources", "intent"]);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "amend", "foo", "--remove-source", "no-such-key"])
            .assert()
            .failure();
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "plan-binding-not-found");
    }

    #[test]
    fn amend_divergence_accepted_writes() {
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        add_pending_entry(&project, "foo");

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "amend", "foo", "--divergence", "accepted"])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            saved.contains("divergence: accepted"),
            "amend --divergence accepted must write the field:\n{saved}"
        );
    }

    #[test]
    fn amend_divergence_rejected_writes() {
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        add_pending_entry(&project, "foo");

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "amend", "foo", "--divergence", "rejected"])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            saved.contains("divergence: rejected"),
            "amend --divergence rejected must write the field:\n{saved}"
        );
    }

    #[test]
    fn amend_divergence_likely_writes() {
        // divergence and writer-ownership contract: `--divergence likely` is operator-settable from
        // the CLI; the field is byte-identical to the legacy
        // skill-written `divergence: likely` line.
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        add_pending_entry(&project, "foo");

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "amend", "foo", "--divergence", "likely"])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            saved.contains("divergence: likely"),
            "amend --divergence likely must write the field:\n{saved}"
        );
    }

    #[test]
    fn plan_amend_divergence_none_refused() {
        let project = Project::init();
        project.seed_plan(W11_PLAN);

        add_pending_entry(&project, "foo");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "amend", "foo", "--divergence", "none"])
            .assert()
            .failure();
        let code = assert.get_output().status.code().expect("exit code");
        assert_eq!(code, 2, "implicit --divergence none must exit 2 (argument error)");
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "argument");
    }
}

mod transition {
    //! `specify plan transition` CLI tests: per-entry edges, undo, the
    //! plan-level Gate-1 stamp, and the retired-state rejections.

    use super::support::*;

    const SINGLE_IN_PROGRESS: &str = "\
name: demo
slices:
  - name: foo
    project: default
    status: in-progress
";

    const SINGLE_DONE: &str = "\
name: demo
slices:
  - name: foo
    project: default
    status: done
";

    #[test]
    fn plan_transition_legal_edge_json() {
        // Post-2.0 the only legal per-entry transition is
        // `InProgress -> Done`.
        let project = Project::init();
        project.seed_plan(SINGLE_IN_PROGRESS);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "transition", "foo", "done"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());

        assert_eq!(actual["name"], "foo");
        assert_eq!(actual["current"], "done");
        assert_eq!(actual["previous"], "in-progress");
        assert_eq!(actual["kind"], "entry");

        assert_golden("transition-in-progress-to-done.json", actual);
    }

    #[test]
    fn plan_transition_rejects_illegal_edge() {
        let project = Project::init();
        project.seed_plan(SINGLE_DONE);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["plan", "transition", "foo", "pending"])
            .assert()
            .failure();
        let code = assert.get_output().status.code();
        assert!(
            code == Some(1) || code == Some(2),
            "illegal transition should be rejected (exit 1 or 2), got: {code:?}"
        );
        let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8");
        assert!(
            stderr.to_lowercase().contains("transition")
                || stderr.contains("plan add")
                || stderr.contains("plan next")
                || stderr.contains("argument"),
            "stderr should mention the rejected transition, got: {stderr:?}"
        );
    }

    #[test]
    fn transition_undo_done_to_in_progress() {
        let project = Project::init();
        project.seed_plan(SINGLE_DONE);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "transition", "foo", "--undo"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(actual["kind"], "undo");
        assert_eq!(actual["name"], "foo");
        assert_eq!(actual["previous"], "done");
        assert_eq!(actual["current"], "in-progress");
        assert_eq!(actual["undo"]["from"], "done");
        assert_eq!(actual["undo"]["to"], "in-progress");

        let plan_after = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
        assert!(plan_after.contains("status: in-progress"), "plan.yaml: {plan_after}");

        let journal = fs::read_to_string(project.root().join(".specify").join("journal.jsonl"))
            .expect("read journal.jsonl");
        let last = journal.lines().rfind(|l| !l.is_empty()).expect("journal line");
        assert!(
            last.contains(r#""event":"plan.transition.undone""#),
            "undo must emit plan.transition.undone, got:\n{last}"
        );
        assert!(last.contains(r#""from":"done""#), "from in payload: {last}");
        assert!(last.contains(r#""to":"in-progress""#), "to in payload: {last}");
    }

    #[test]
    fn undo_in_progress_to_pending_refuses() {
        let project = Project::init();
        project.seed_plan(SINGLE_IN_PROGRESS);

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "transition", "foo", "--undo"])
            .assert()
            .success();

        let plan_mid = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
        assert!(plan_mid.contains("status: pending"), "plan.yaml after first undo: {plan_mid}");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["plan", "transition", "foo", "--undo"])
            .assert()
            .failure();
        let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8");
        assert!(
            stderr.contains("pending"),
            "undo-from-pending stderr should mention `pending`, got: {stderr:?}"
        );
    }

    #[test]
    fn transition_plan_level_approved() {
        // workflow §The Plan: `specify plan transition <plan-name>
        // approved` is the operator-stamped Gate 1 transition. The plan
        // name on the wire matches `plan.yaml.name`.
        let project = Project::init();
        project.seed_plan(SINGLE_PENDING);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "transition", "demo", "approved"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(actual["kind"], "plan");
        assert_eq!(actual["name"], "demo");
        assert_eq!(actual["previous"], "pending");
        assert_eq!(actual["current"], "approved");

        assert_golden("transition-plan-approved.json", actual);
    }

    #[test]
    fn transition_rejects_per_entry_in_progress() {
        // Per-entry `in-progress` is owned by `plan next`. `plan transition`
        // must reject the request with an argument-shape error (exit 2).
        let project = Project::init();
        project.seed_plan(SINGLE_PENDING);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["plan", "transition", "foo", "in-progress"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8");
        assert!(
            stderr.contains("plan next"),
            "stderr should point at `plan next`, got: {stderr:?}"
        );
    }

    #[test]
    fn plan_transition_rejects_retired_states() {
        // `blocked`, `failed`, and `skipped` are not valid per-entry states.
        // Each must be rejected with the same argument-shape error.
        let project = Project::init();
        project.seed_plan(SINGLE_PENDING);

        for invalid in ["blocked", "failed", "skipped"] {
            let assert = specify_cmd()
                .current_dir(project.root())
                .args(["plan", "transition", "foo", invalid])
                .assert()
                .failure();
            assert_eq!(
                assert.get_output().status.code(),
                Some(2),
                "invalid state `{invalid}` must yield exit 2"
            );
        }
    }

    // `plan transition <name> failed --reason <text>` is not a valid form —
    // there is no per-entry `failed` state; see
    // `plan_transition_rejects_retired_states` above.

    #[test]
    fn transition_rejects_unknown_reason() {
        // `--reason` is not a valid flag (no per-entry state accepts a
        // reason). Clap surfaces unknown flags as exit 2 with `--reason`
        // named in stderr.
        let project = Project::init();
        project.seed_plan(SINGLE_PENDING);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["plan", "transition", "foo", "in-progress", "--reason", "x"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8");
        assert!(stderr.contains("--reason"), "stderr should mention '--reason', got: {stderr:?}");
    }

    // Re-entry to `pending` retired with the per-entry status purge
    // (the 2.0 collapse removed the per-entry enum to `pending | in-progress | done`).
}

mod create {
    //! `specify plan create` CLI tests, the human-driven replay loop, and
    //! the `--auto-approve` Gate-1 contract.

    use super::support::*;

    // -- human-driven replay (the human-driven plan loop) -----------

    #[test]
    fn plan_human_replay_matches_fixture() {
        let project = Project::init();
        project.seed_plan(
            "\
name: demo
slices:
  - name: user-registration
    project: default
    status: done
",
        );

        specify_cmd()
        .current_dir(project.root())
        .args([
            "plan",
            "add",
            "registration-duplicate-email-crash",
            "--description",
            "Duplicate email submission returns 500 instead of 409. Modifies user-registration.",
        ])
        .assert()
        .success();

        specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();

        specify_cmd()
            .current_dir(project.root())
            .args([
                "plan",
                "amend",
                "registration-duplicate-email-crash",
                "--description",
                "Clarified scope",
            ])
            .assert()
            .success();

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "transition", "registration-duplicate-email-crash", "done"])
            .assert()
            .success();

        let actual = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
        let fixture_path = plan_fixtures().join("human-replay-final.yaml");

        if std::env::var_os("REGENERATE_GOLDENS").is_some() {
            fs::create_dir_all(plan_fixtures()).expect("mkdir plan fixtures");
            fs::write(&fixture_path, &actual).expect("write fixture");
            return;
        }

        let expected = fs::read_to_string(&fixture_path).unwrap_or_else(|err| {
            panic!(
                "fixture {} missing ({err}); regenerate via \
                 REGENERATE_GOLDENS=1 cargo nextest run --test plan",
                fixture_path.display()
            )
        });

        assert_eq!(
            actual,
            expected,
            "plan.yaml after replay diverged from fixture {}\n--- actual ---\n{actual}\n--- expected ---\n{expected}",
            fixture_path.display()
        );
    }

    #[test]
    fn create_scaffolds_matches_golden() {
        let project = Project::init();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "create", "my-change"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());

        assert_eq!(actual["name"], "my-change");
        let plan_path = actual["plan"].as_str().expect("plan string");
        assert!(
            plan_path.ends_with("/plan.yaml"),
            "plan should end with /plan.yaml at the repo root, got: {plan_path}"
        );

        assert!(project.plan_path().exists(), "plan.yaml should be created");
        assert!(!project.root().join("change.md").exists(), "plan create must not write change.md");

        assert_golden("plan-create.json", actual);
    }

    #[test]
    fn create_refuses_overwrite() {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "create", "first"])
            .assert()
            .success();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "create", "second"])
            .assert()
            .failure();
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "already-exists");
    }

    #[test]
    fn plan_create_then_validate_passes_clean() {
        let project = Project::init();

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "create", "fresh"])
            .assert()
            .success();

        let assert =
            specify_cmd().current_dir(project.root()).args(["plan", "validate"]).assert().success();
        assert_eq!(assert.get_output().status.code(), Some(0));
        let stdout = std::str::from_utf8(&assert.get_output().stdout).expect("utf8");
        assert!(
            !stdout.contains("ERROR"),
            "freshly-scaffolded plan must pass `specify plan validate` with no errors, got:\n{stdout}"
        );
    }

    // -- plan create --intent (elicitation-to-argv sugar, RFC-61 Milestone E) ----------------

    #[test]
    fn create_intent_desugars_to_source_binding() {
        // `--intent <string>` is pure sugar for
        // `--source intent=intent:value:<string>`: the two invocations
        // must persist byte-identical plan.yaml files.
        let sugared = Project::init();
        specify_cmd()
            .current_dir(sugared.root())
            .args(["plan", "create", "demo", "--intent", "Fix the greeting typo."])
            .assert()
            .success();

        let explicit = Project::init();
        specify_cmd()
            .current_dir(explicit.root())
            .args([
                "plan",
                "create",
                "demo",
                "--source",
                "intent=intent:value:Fix the greeting typo.",
            ])
            .assert()
            .success();

        let sugared_yaml = fs::read_to_string(sugared.plan_path()).expect("sugared plan.yaml");
        let explicit_yaml = fs::read_to_string(explicit.plan_path()).expect("explicit plan.yaml");
        assert_eq!(
            sugared_yaml, explicit_yaml,
            "--intent must desugar byte-identically to the explicit intent source binding"
        );
        assert!(sugared_yaml.contains("adapter: intent"), "{sugared_yaml}");
        assert!(sugared_yaml.contains("Fix the greeting typo."), "{sugared_yaml}");
    }

    #[test]
    fn create_intent_conflicts_with_source() {
        // Combining `--intent` with an explicit `--source intent=...`
        // binding trips the existing duplicate-key gate — no bespoke
        // conflict rule.
        let project = Project::init();
        let assert = specify_cmd()
            .current_dir(project.root())
            .args([
                "--format",
                "json",
                "plan",
                "create",
                "demo",
                "--intent",
                "One brief.",
                "--source",
                "intent=intent:value:Another brief.",
            ])
            .assert()
            .failure();
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "plan-source-duplicate-key");
        assert!(!project.plan_path().exists(), "a refused create must not write plan.yaml");
    }

    // -- plan execute (guest-owned verb: triage routes it to the guest leg) -------------------

    #[test]
    fn plan_execute_routes_to_guest_leg() {
        // `plan execute` lives in the shared grammar but runs only in
        // the workflow guest; the triage main routes it to the
        // composed-deployment leg instead of the native handler table
        // (DECISIONS.md §"One `specify` binary"). With no `cursor-agent`
        // on PATH the spawned `specify-host` fails deterministically at
        // backend connect — its stderr names the missing agent and its
        // exit 1 passes through, not the old native `argument` refusal.
        crate::common::ensure_host_binary();
        let empty_path = tempdir().expect("empty PATH dir");
        let project = Project::init();
        let assert = specify_cmd()
            .current_dir(project.root())
            .env("PATH", empty_path.path())
            .env_remove("RUST_LOG")
            .args(["--format", "json", "plan", "execute"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
        assert!(stderr.contains("cursor-agent"), "the drive must reach the host spawn:\n{stderr}");
    }

    // -- plan author (guest-owned verb: triage routes it to the guest leg) --------------------

    #[test]
    fn plan_author_routes_to_guest_leg() {
        // `plan author` (the collapsed /spec:plan flow, RFC-61 S1) —
        // the same triage posture as `plan execute`.
        crate::common::ensure_host_binary();
        let empty_path = tempdir().expect("empty PATH dir");
        let project = Project::init();
        let assert = specify_cmd()
            .current_dir(project.root())
            .env("PATH", empty_path.path())
            .env_remove("RUST_LOG")
            .args(["--format", "json", "plan", "author", "fresh", "--intent", "Fix the typo."])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
        assert!(stderr.contains("cursor-agent"), "the drive must reach the host spawn:\n{stderr}");
        assert!(!project.plan_path().exists(), "a failed author must not write plan.yaml");
    }

    // -- plan create --auto-approve (auto-approve Gate-1 contract) ---------------------------

    #[test]
    fn create_auto_approve_stamps() {
        // auto-approve Gate-1 contract: `--auto-approve` is the operator's Gate-1 consent at
        // create time. The on-disk plan carries `lifecycle: approved`
        // directly (single atomic write — no transient `pending`
        // observable to readers) and the journal carries exactly one
        // `plan.transition.approved` event matching the post-create stamp.
        let project = Project::init();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "create", "fresh", "--auto-approve"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(actual["name"], "fresh");
        assert_eq!(actual["lifecycle"], "approved");

        let on_disk = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
        assert!(
            on_disk.contains("lifecycle: approved"),
            "plan.yaml must carry `lifecycle: approved` after --auto-approve, got:\n{on_disk}"
        );
        assert!(
            !on_disk.contains("lifecycle: pending"),
            "no transient `lifecycle: pending` must remain on disk, got:\n{on_disk}"
        );

        let journal = project.root().join(".specify").join("journal.jsonl");
        let raw = fs::read_to_string(&journal).expect("read journal.jsonl");
        let lines: Vec<&str> = raw.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(
            lines.len(),
            1,
            "exactly one journal event (plan.transition.approved) per --auto-approve create, got:\n{raw}"
        );
        assert!(
            lines[0].contains(r#""event":"plan.transition.approved""#),
            "first (and only) line must be plan.transition.approved, got:\n{}",
            lines[0]
        );
        assert!(
            lines[0].contains(r#""plan-name":"fresh""#),
            "plan-name must serialise kebab-case, got:\n{}",
            lines[0]
        );
        assert!(
            lines[0].contains(r#""actor":"operator""#),
            "--auto-approve is operator consent, so the stamp records actor: operator, got:\n{}",
            lines[0]
        );
    }

    #[test]
    fn plan_create_auto_approve_idempotent() {
        // auto-approve Gate-1 contract: running `specify plan transition <name> approved`
        // after a successful `--auto-approve` create must be a no-op —
        // exit 0, no second `plan.transition.approved` event, plan.yaml
        // unchanged.
        let project = Project::init();

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "create", "fresh", "--auto-approve"])
            .assert()
            .success();
        let journal = project.root().join(".specify").join("journal.jsonl");
        let before = fs::read_to_string(&journal).expect("read journal.jsonl");
        let before_lines = before.lines().filter(|l| !l.is_empty()).count();
        let plan_before = fs::read_to_string(project.plan_path()).expect("read plan.yaml");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "transition", "fresh", "approved"])
            .assert()
            .success();
        let body = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(body["kind"], "plan");
        assert_eq!(
            body["previous"], "approved",
            "previous lifecycle must already be approved (no-op), got:\n{body}"
        );
        assert_eq!(body["current"], "approved");

        let plan_after = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
        assert_eq!(
            plan_before, plan_after,
            "plan.yaml must not change under the idempotent no-op transition"
        );
        let after = fs::read_to_string(&journal).expect("read journal.jsonl");
        let after_lines = after.lines().filter(|l| !l.is_empty()).count();
        assert_eq!(
            before_lines, after_lines,
            "explicit `transition approved` after --auto-approve must not append a second event"
        );
    }

    #[test]
    fn plan_create_auto_approve_invalid_name() {
        // auto-approve Gate-1 contract: `--auto-approve` does NOT bypass validation. An
        // invalid (non-kebab) name refuses the create with the same
        // exit code and envelope as the post-create path; no `plan.yaml`
        // lands on disk and the journal stays untouched.
        let project = Project::init();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "create", "Bad_Name", "--auto-approve"])
            .assert()
            .failure();
        let code = assert.get_output().status.code().expect("exit code");
        assert_eq!(code, 1, "kebab-case violation surfaces via Error::Diag (exit 1)");
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "change-name-not-kebab");

        assert!(
            !project.plan_path().exists(),
            "plan.yaml must not be written when --auto-approve fails validation"
        );
        let journal = project.root().join(".specify").join("journal.jsonl");
        assert!(
            !journal.exists(),
            "journal must stay empty when --auto-approve validation fails, found: {}",
            journal.display()
        );
    }

    #[test]
    fn create_auto_approve_no_partial_events() {
        // auto-approve Gate-1 contract: validation failure under --auto-approve must not
        // surface a partial-state event sequence — no orphan
        // `plan.amend.authority-override` without the matching
        // `plan.transition.approved`, no half-written plan.yaml. An
        // unknown `--authority-override` slice (the cheapest validation
        // gate to trip on a fresh plan) must refuse the create and
        // leave the journal untouched.
        let project = Project::init();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args([
                "plan",
                "create",
                "fresh",
                "--auto-approve",
                "--authority-override",
                "ghost-slice",
                "criterion=runtime",
            ])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));

        assert!(
            !project.plan_path().exists(),
            "plan.yaml must not be written when --auto-approve + --authority-override fails"
        );
        let journal = project.root().join(".specify").join("journal.jsonl");
        assert!(
            !journal.exists(),
            "journal must stay empty on validation failure, found: {}",
            journal.display()
        );
    }

    #[test]
    fn create_auto_approve_then_validate_passes() {
        // The empty-scaffold + `--auto-approve` combination must still
        // validate cleanly — `--auto-approve` is a Gate-1 consent flag,
        // not a validation bypass, but it also must not introduce any
        // new validation drift on the empty-scaffold path.
        let project = Project::init();

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "create", "fresh", "--auto-approve"])
            .assert()
            .success();

        let assert =
            specify_cmd().current_dir(project.root()).args(["plan", "validate"]).assert().success();
        assert_eq!(assert.get_output().status.code(), Some(0));
    }
}

mod archive {
    //! `specify plan archive` CLI tests, including the working-directory
    //! co-move (L1.K / L3.B).

    use super::support::*;

    // -- plan archive (L1.K) ----------------------------------------------
    //
    // Determinism: the archive verb stamps its filename from
    // `Timestamp::now()` read *inside* the CLI subprocess, and the CLI
    // exposes no clock-injection seam (`Ctx::now()` hardcodes
    // `Timestamp::now()`; `plan archive` passes `Timestamp::now()` straight
    // through). So tests must not reconstruct the stamp from their own
    // clock — a midnight roll between the two reads would desync them.
    // Discovery assertions match the produced `<name>-YYYYMMDD` shape with
    // a regex; the two collision tests, which must pre-create the exact
    // destination, seed the whole `date_window()` instead.

    /// UTC `YYYYMMDD` stamps for yesterday / today / tomorrow. The CLI
    /// reads its clock a beat after the test reads its own, so its stamp is
    /// always within this window; seeding all three guarantees a collision
    /// regardless of a midnight roll.
    fn date_window() -> Vec<String> {
        let day = jiff::SignedDuration::from_hours(24);
        let now = jiff::Timestamp::now();
        [now.checked_sub(day).expect("now - 24h"), now, now.checked_add(day).expect("now + 24h")]
            .iter()
            .map(|ts| ts.strftime("%Y%m%d").to_string())
            .collect()
    }

    /// Entry names directly under `.specify/archive/plans` (empty when the
    /// dir is absent).
    fn archived_entries(project: &Project) -> Vec<String> {
        fs::read_dir(archive_dir(project))
            .map(|rd| {
                rd.filter_map(|e| e.ok().and_then(|e| e.file_name().into_string().ok())).collect()
            })
            .unwrap_or_default()
    }

    /// Locate the archived plan file `<name>-YYYYMMDD.yaml`, if the verb
    /// wrote one — matched by shape rather than a clock-derived literal.
    fn archived_plan_file(project: &Project, name: &str) -> Option<PathBuf> {
        let re = regex::Regex::new(&format!(r"^{}-\d{{8}}\.yaml$", regex::escape(name)))
            .expect("regex compiles");
        archived_entries(project)
            .into_iter()
            .find(|f| re.is_match(f))
            .map(|f| archive_dir(project).join(f))
    }

    /// Locate the co-moved archive directory `<name>-YYYYMMDD`, if any.
    fn archived_plan_dir(project: &Project, name: &str) -> Option<PathBuf> {
        let re = regex::Regex::new(&format!(r"^{}-\d{{8}}$", regex::escape(name)))
            .expect("regex compiles");
        archived_entries(project)
            .into_iter()
            .find(|f| re.is_match(f))
            .map(|f| archive_dir(project).join(f))
    }

    /// Replace any `-YYYYMMDD` date stamp in JSON strings with a stable
    /// placeholder so the archive-success golden is date-insensitive.
    fn strip_date_stamps(value: &mut Value) {
        fn visit(re: &regex::Regex, v: &mut Value) {
            match v {
                Value::String(s) if re.is_match(s) => {
                    *s = re.replace_all(s, "-<YYYYMMDD>").into_owned();
                }
                Value::Array(items) => {
                    for item in items {
                        visit(re, item);
                    }
                }
                Value::Object(map) => {
                    for (_k, v) in map.iter_mut() {
                        visit(re, v);
                    }
                }
                _ => {}
            }
        }
        let re = regex::Regex::new(r"-\d{8}\b").expect("regex compiles");
        visit(&re, value);
    }

    fn archive_dir(project: &Project) -> PathBuf {
        project.root().join(".specify/archive/plans")
    }

    #[test]
    fn plan_archive_happy_path_json() {
        let project = Project::init();
        project.seed_plan(ALL_DONE);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "archive"])
            .assert()
            .success();
        let mut actual = parse_stdout(&assert.get_output().stdout, project.root());

        assert_eq!(actual["plan"]["name"], "demo");
        assert!(
            actual["archived"].as_str().unwrap_or_default().contains("demo-"),
            "archived path should contain the plan name, got: {}",
            actual["archived"]
        );

        // Filesystem effects of the move, asserted here rather than in a
        // separate text-format twin.
        assert!(!project.plan_path().exists(), "original plan.yaml must be gone");
        assert!(
            archived_plan_file(&project, "demo").is_some(),
            "archived plan file not found under {}",
            archive_dir(&project).display()
        );

        strip_date_stamps(&mut actual);
        assert_golden("archive-success.json", actual);
    }

    #[test]
    fn plan_archive_refuses_without_force() {
        let project = Project::init();
        project.seed_plan(A_DONE_B_PENDING);

        let assert =
            specify_cmd().current_dir(project.root()).args(["plan", "archive"]).assert().failure();
        assert_eq!(assert.get_output().status.code(), Some(1));
        let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8 stderr");
        assert!(
            stderr.contains('b'),
            "stderr should mention the pending entry name 'b', got: {stderr:?}"
        );
        assert!(stderr.contains("--force"), "stderr should suggest --force, got: {stderr:?}");

        assert!(project.plan_path().exists(), "plan.yaml must still exist");
        assert!(
            archived_plan_file(&project, "demo").is_none(),
            "no archive file should be written on refusal"
        );
    }

    #[test]
    fn plan_archive_refuses_json_lists_entries() {
        let project = Project::init();
        project.seed_plan(A_DONE_B_PENDING);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "archive"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(1));

        // The typed failure envelope is written to stderr.
        let actual = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(actual["error"], "plan-has-outstanding-work");
        assert_eq!(actual["exit-code"], 1);
        let message = actual["message"].as_str().expect("message string");
        assert!(message.contains('b'), "message should mention the pending entry 'b': {message}");

        assert_golden("archive-outstanding-work.json", actual);
    }

    #[test]
    fn plan_archive_with_force_succeeds() {
        let project = Project::init();
        project.seed_plan(A_DONE_B_PENDING);

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "archive", "--force"])
            .assert()
            .success();

        let archived = archived_plan_file(&project, "demo")
            .expect("archived plan file must exist after --force");
        let contents = fs::read_to_string(&archived).expect("read archived yaml");
        assert!(
            contents.contains("name: b"),
            "archived yaml should preserve pending entry 'b':\n{contents}"
        );
        assert!(
            contents.contains("status: pending"),
            "archived yaml should preserve pending status verbatim:\n{contents}"
        );
    }

    #[test]
    fn archive_filename_kebab_plus_date() {
        let project = Project::init();
        project.seed_plan(
            "\
name: my-change
slices: []
",
        );

        specify_cmd().current_dir(project.root()).args(["plan", "archive"]).assert().success();

        let re = regex::Regex::new(r"^my-change-\d{8}\.yaml$").expect("regex compiles");
        let entries: Vec<String> = fs::read_dir(archive_dir(&project))
            .expect("read archive dir")
            .filter_map(|e| e.ok().and_then(|e| e.file_name().into_string().ok()))
            .collect();
        assert_eq!(entries.len(), 1, "expected exactly one archive file, got: {entries:?}");
        assert!(
            re.is_match(&entries[0]),
            "archive filename {} should match `my-change-<YYYYMMDD>.yaml`",
            entries[0]
        );
    }

    #[test]
    fn plan_archive_refuses_when_dest_exists() {
        let project = Project::init();
        project.seed_plan(ALL_DONE);

        let dest_dir = archive_dir(&project);
        fs::create_dir_all(&dest_dir).expect("mkdir archive dir");
        // Seed a collision file for every stamp the CLI's clock might pick
        // (see `date_window`) so the dest-exists guard fires deterministically.
        let seeded: Vec<PathBuf> = date_window()
            .iter()
            .map(|d| {
                let dest = dest_dir.join(format!("demo-{d}.yaml"));
                fs::write(&dest, "prior: content\n").expect("seed prior archive");
                dest
            })
            .collect();

        let assert =
            specify_cmd().current_dir(project.root()).args(["plan", "archive"]).assert().failure();
        assert_eq!(assert.get_output().status.code(), Some(1));
        let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8 stderr");
        assert!(
            stderr.contains("already exists"),
            "stderr should mention 'already exists', got: {stderr:?}"
        );

        assert!(project.plan_path().exists(), "original plan.yaml must be untouched");
        for dest in &seeded {
            assert_eq!(
                fs::read_to_string(dest).expect("read prior archive"),
                "prior: content\n",
                "pre-existing archive destination must not be overwritten"
            );
        }
    }

    #[test]
    fn plan_archive_missing_file_errors() {
        let project = Project::init();
        // Deliberately do NOT seed plan.yaml.

        let assert =
            specify_cmd().current_dir(project.root()).args(["plan", "archive"]).assert().failure();
        assert_eq!(assert.get_output().status.code(), Some(1));
        let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8 stderr");
        assert!(
            stderr.contains("plan.yaml not found at"),
            "stderr should mention 'plan.yaml not found at', got: {stderr:?}"
        );
    }

    // -- plan archive co-move of working directory (L3.B) ---------------

    /// Seed `.specify/plans/<name>/` with the given files and return
    /// the directory path.
    fn seed_working_dir(project: &Project, plan_name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let dir = project.root().join(".specify/plans").join(plan_name);
        fs::create_dir_all(&dir).expect("mkdir plans working dir");
        for (name, bytes) in files {
            fs::write(dir.join(name), bytes).expect("seed working file");
        }
        dir
    }

    #[test]
    fn plan_archive_co_moves_working_dir() {
        let project = Project::init();
        project.seed_plan(ALL_DONE);
        let working_dir = seed_working_dir(
            &project,
            "demo",
            &[("discovery.md", b"# discovery\n"), ("proposal.md", b"# proposal\n")],
        );

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "archive"])
            .assert()
            .success();
        let mut actual = parse_stdout(&assert.get_output().stdout, project.root());

        assert_eq!(actual["plan"]["name"], "demo");
        assert!(
            actual["archived"].as_str().unwrap_or_default().contains("demo-"),
            "archived path should contain the plan name"
        );
        assert!(
            actual["archived-plans-dir"].as_str().unwrap_or_default().contains("demo-"),
            "archived-plans-dir should contain the plan name, got: {}",
            actual["archived-plans-dir"]
        );

        assert!(!working_dir.exists(), ".specify/plans/demo/ must be gone after archive");
        let archived_dir =
            archived_plan_dir(&project, "demo").expect("co-moved archive directory must exist");
        assert!(archived_dir.is_dir(), "co-moved dir missing at {}", archived_dir.display());
        assert_eq!(
            fs::read_to_string(archived_dir.join("discovery.md")).expect("read"),
            "# discovery\n"
        );
        assert_eq!(
            fs::read_to_string(archived_dir.join("proposal.md")).expect("read"),
            "# proposal\n"
        );

        strip_date_stamps(&mut actual);
        assert_golden("archive-success-with-working-dir.json", actual);
    }

    #[test]
    fn plan_archive_no_working_dir_json() {
        let project = Project::init();
        project.seed_plan(ALL_DONE);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "archive"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());

        assert_eq!(
            actual["archived-plans-dir"],
            Value::Null,
            "no working dir must surface archived-plans-dir: null, got: {}",
            actual["archived-plans-dir"]
        );
    }

    #[test]
    fn plan_archive_co_move_collision_halts() {
        let project = Project::init();
        project.seed_plan(ALL_DONE);
        let working_dir = seed_working_dir(&project, "demo", &[("notes.md", b"# notes\n")]);

        // Pre-create the co-move destination dir for the whole date window so
        // the working-dir preflight collides regardless of a midnight roll;
        // the plan.yaml archive destination stays clear, isolating the
        // working-dir preflight specifically.
        let seeded_dirs: Vec<PathBuf> = date_window()
            .iter()
            .map(|d| {
                let dir = archive_dir(&project).join(format!("demo-{d}"));
                fs::create_dir_all(&dir).expect("seed collision dir");
                dir
            })
            .collect();

        let assert =
            specify_cmd().current_dir(project.root()).args(["plan", "archive"]).assert().failure();
        assert_eq!(assert.get_output().status.code(), Some(1));
        let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8 stderr");
        assert!(
            stderr.contains("already exists"),
            "stderr should name 'already exists', got: {stderr:?}"
        );

        // Preflight contract: plan.yaml must be untouched on collision.
        assert!(
            project.plan_path().exists(),
            "plan.yaml MUST be untouched when working-dir preflight fails"
        );
        assert!(working_dir.is_dir(), "source working dir must be untouched on collision");
        assert!(
            archived_plan_file(&project, "demo").is_none(),
            "plan.yaml must not have been archived on collision"
        );
        for dir in &seeded_dirs {
            assert!(
                dir.is_dir() && fs::read_dir(dir).expect("read").next().is_none(),
                "pre-existing collision dir must remain empty"
            );
        }
    }
}

mod authority {
    //! `plan {create,add,amend} --authority-override` CLI tests.

    use super::support::*;

    // -- plan {create,add,amend} --authority-override (per-slice authority override) --------

    const AUTHORITY_OVERRIDE_PLAN: &str = "\
name: identity-revamp
sources:
  legacy:
    adapter: typescript
    path: ./legacy-monolith
  runtime:
    adapter: captures
    path: ./captures/replays
slices:
  - name: identity-user-registration
    project: default
    status: pending
    sources:
      - source: legacy
        lead: user-registration
      - source: runtime
        lead: user-registration
";

    fn read_journal_lines(project: &Project) -> Vec<String> {
        let path = project.root().join(".specify").join("journal.jsonl");
        if !path.exists() {
            return Vec::new();
        }
        fs::read_to_string(&path)
            .expect("read journal")
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn amend_authority_override_round_trips() {
        // per-slice authority override happy path: set an override via `amend`, re-read
        // `plan.yaml` and confirm the field landed under the named
        // slice; `slice validate` accepts it because `runtime` is in
        // the slice's `sources[]`.
        let project = Project::init();
        project.seed_plan(AUTHORITY_OVERRIDE_PLAN);

        specify_cmd()
            .current_dir(project.root())
            .args([
                "plan",
                "amend",
                "identity-user-registration",
                "--authority-override",
                "identity-user-registration",
                "requirement=runtime",
            ])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            saved.contains("authority-override:"),
            "plan.yaml must contain authority-override block, got:\n{saved}"
        );
        assert!(
            saved.contains("requirement: runtime"),
            "plan.yaml must record requirement: runtime, got:\n{saved}"
        );

        // Plan-level validate passes — orphan check only fires for bad keys.
        specify_cmd().current_dir(project.root()).args(["plan", "validate"]).assert().success();

        // Journal carries exactly one PlanAmendAuthorityOverride event.
        let lines = read_journal_lines(&project);
        assert_eq!(lines.len(), 1, "expected one journal event, got:\n{lines:?}");
        let line = &lines[0];
        assert!(line.contains(r#""event":"plan.amend.authority-override""#));
        assert!(line.contains(r#""action":"set""#));
        assert!(line.contains(r#""claim-kind":"requirement""#));
        assert!(line.contains(r#""source":"runtime""#));
        assert!(line.contains(r#""slice-name":"identity-user-registration""#));
    }

    #[test]
    fn plan_amend_override_orphan_refused() {
        // per-slice authority override gate: refuse the `specify plan amend` write when
        // the authority-override value names a source key not present
        // in the slice's `sources[]` list (`phantom`). The orphan
        // check runs in `Plan::validate` (folded in by Change 2.3),
        // which `mutate_authority_overrides` re-runs after the
        // override mutations to catch the case where a brand-new
        // entry would introduce drift.
        let project = Project::init();
        project.seed_plan(AUTHORITY_OVERRIDE_PLAN);
        let before = fs::read_to_string(project.plan_path()).expect("read plan");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args([
                "--format",
                "json",
                "plan",
                "amend",
                "identity-user-registration",
                "--authority-override",
                "identity-user-registration",
                "requirement=phantom",
            ])
            .assert()
            .failure();
        let code = assert.get_output().status.code().expect("exit code");
        assert_eq!(code, 2, "orphan source must exit 2 (validation_failed)");
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "slice-authority-override-orphan-source");

        let after = fs::read_to_string(project.plan_path()).expect("read plan");
        assert_eq!(before, after, "plan.yaml must not change on the refused write");
        assert!(
            read_journal_lines(&project).is_empty(),
            "journal must stay empty on the refused write"
        );
    }

    #[test]
    fn slice_validate_authority_override_orphan() {
        // per-slice authority override — `specify slice validate` is the per-slice gate
        // that mirrors the plan-level check; it runs before refine
        // synthesises any artifacts so a bad override is caught
        // before downstream writes. Hand-edit `plan.yaml` to seed an
        // orphan entry (the only legal path is via the CLI, which
        // refuses, so we splice the file to exercise the gate without
        // bypassing the JSON-schema enforcement).
        let project = Project::init();
        project.seed_plan(AUTHORITY_OVERRIDE_PLAN);
        let original = fs::read_to_string(project.plan_path()).expect("read plan");
        // Splice the orphan override into the first slice. Anchor on
        // the `status: pending` line so the YAML structure stays
        // wellformed regardless of source-binding ordering.
        let needle = "    status: pending\n    sources:";
        let replacement = "    status: pending\n    authority-override:\n      requirement: phantom\n    sources:";
        let patched = original.replacen(needle, replacement, 1);
        assert_ne!(patched, original, "splice precondition: needle present in plan.yaml");
        fs::write(project.plan_path(), patched.as_bytes()).expect("write patched plan");

        // Create the slice dir so `slice validate` runs to the gate
        // (other artifacts absent → no spec/evidence findings).
        let slices_dir =
            project.root().join(".specify").join("slices").join("identity-user-registration");
        fs::create_dir_all(&slices_dir).expect("mkdir slice");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "identity-user-registration"])
            .assert()
            .failure();
        let code = assert.get_output().status.code().expect("exit code");
        assert_eq!(code, 2, "slice validate orphan must exit 2 (validation_failed)");
        // `slice validate` renders the DiagnosticReport on stdout and fails
        // payload-free on stderr; the orphan finding lives on the report.
        let report = parse_stdout(&assert.get_output().stdout, project.root());
        let findings = report["findings"].as_array().expect("findings array");
        assert!(
            findings.iter().any(|r| r["rule-id"] == "slice-authority-override-orphan-source"),
            "expected orphan finding from slice validate: {findings:#?}"
        );
    }

    #[test]
    fn amend_clear_override_removes_one() {
        // per-slice authority override: `--clear-authority-override <slice> <kind>` peels
        // off a single entry; the rest of the map survives. Journal
        // records the Clear without any spurious Set events for the
        // surviving entries.
        let project = Project::init();
        project.seed_plan(AUTHORITY_OVERRIDE_PLAN);

        specify_cmd()
            .current_dir(project.root())
            .args([
                "plan",
                "amend",
                "identity-user-registration",
                "--authority-override",
                "identity-user-registration",
                "requirement=runtime",
                "--authority-override",
                "identity-user-registration",
                "criterion=legacy",
            ])
            .assert()
            .success();

        // Wipe the journal so we observe only the second amend's events.
        fs::write(project.root().join(".specify").join("journal.jsonl"), "")
            .expect("clear journal");

        specify_cmd()
            .current_dir(project.root())
            .args([
                "plan",
                "amend",
                "identity-user-registration",
                "--clear-authority-override",
                "identity-user-registration",
                "requirement",
            ])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            !saved.contains("requirement: runtime"),
            "requirement entry must be cleared, got:\n{saved}"
        );
        assert!(
            saved.contains("criterion: legacy"),
            "criterion entry must survive the targeted clear, got:\n{saved}"
        );

        let lines = read_journal_lines(&project);
        assert_eq!(lines.len(), 1, "expected one Clear event, got:\n{lines:?}");
        let line = &lines[0];
        assert!(line.contains(r#""action":"clear""#));
        assert!(line.contains(r#""claim-kind":"requirement""#));
    }

    #[test]
    fn plan_amend_clear_overrides_wipes_map() {
        // per-slice authority override: `--clear-authority-overrides <slice>` wipes the
        // entire `authority-override` map for that slice and emits one
        // Clear event per kind that was present before the wipe.
        let project = Project::init();
        project.seed_plan(AUTHORITY_OVERRIDE_PLAN);

        specify_cmd()
            .current_dir(project.root())
            .args([
                "plan",
                "amend",
                "identity-user-registration",
                "--authority-override",
                "identity-user-registration",
                "requirement=runtime",
                "--authority-override",
                "identity-user-registration",
                "criterion=legacy",
            ])
            .assert()
            .success();
        fs::write(project.root().join(".specify").join("journal.jsonl"), "")
            .expect("clear journal");

        specify_cmd()
            .current_dir(project.root())
            .args([
                "plan",
                "amend",
                "identity-user-registration",
                "--clear-authority-overrides",
                "identity-user-registration",
            ])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            !saved.contains("authority-override:"),
            "authority-override map must elide once empty, got:\n{saved}"
        );

        let lines = read_journal_lines(&project);
        assert_eq!(lines.len(), 2, "expected two per-kind Clear events, got:\n{lines:?}");
        let combined = lines.join("\n");
        assert!(combined.contains(r#""claim-kind":"requirement""#));
        assert!(combined.contains(r#""claim-kind":"criterion""#));
        assert!(
            lines.iter().all(|l| l.contains(r#""action":"clear""#)),
            "every emitted event must carry action: clear, got:\n{combined}"
        );
    }

    #[test]
    fn amend_authority_override_set_then_clear() {
        // per-slice authority override deterministic-order rule: a same-invocation set +
        // clear pair on the same `(slice, kind)` resolves to the
        // cleared state; the journal records the Clear (not the Set).
        let project = Project::init();
        project.seed_plan(AUTHORITY_OVERRIDE_PLAN);

        specify_cmd()
            .current_dir(project.root())
            .args([
                "plan",
                "amend",
                "identity-user-registration",
                "--authority-override",
                "identity-user-registration",
                "requirement=runtime",
                "--clear-authority-override",
                "identity-user-registration",
                "requirement",
            ])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(
            !saved.contains("requirement: runtime"),
            "set+clear on same kind must resolve to cleared, got:\n{saved}"
        );
        let lines = read_journal_lines(&project);
        assert_eq!(lines.len(), 1, "expected one Clear event (set was elided), got:\n{lines:?}");
        assert!(
            lines[0].contains(r#""action":"clear""#),
            "the surviving event must be a clear, got:\n{}",
            lines[0]
        );
    }

    #[test]
    fn add_authority_override_seeds_map() {
        // per-slice authority override add path: `plan add --authority-override
        // <kind>=<key>` pre-seeds the override map at create time. Each
        // entry fires one PlanAmendAuthorityOverride / `set` event.
        let project = Project::init();
        project.seed_plan(
            "name: identity-revamp\n\
        sources:\n\
        \x20\x20legacy:\n\
        \x20\x20\x20\x20adapter: typescript\n\
        \x20\x20\x20\x20path: ./legacy\n\
        \x20\x20runtime:\n\
        \x20\x20\x20\x20adapter: captures\n\
        \x20\x20\x20\x20path: ./captures/replays\n\
        slices: []\n",
        );

        specify_cmd()
            .current_dir(project.root())
            .args([
                "plan",
                "add",
                "identity-user-registration",
                "--sources",
                "legacy=user-registration",
                "--sources",
                "runtime=user-registration",
                "--authority-override",
                "requirement=runtime",
                "--authority-override",
                "criterion=legacy",
            ])
            .assert()
            .success();

        let saved = fs::read_to_string(project.plan_path()).expect("read plan");
        assert!(saved.contains("authority-override:"));
        assert!(saved.contains("requirement: runtime"));
        assert!(saved.contains("criterion: legacy"));

        let lines = read_journal_lines(&project);
        assert_eq!(lines.len(), 2, "one event per seeded kind, got:\n{lines:?}");
        for line in &lines {
            assert!(line.contains(r#""action":"set""#));
            assert!(line.contains(r#""slice-name":"identity-user-registration""#));
        }
    }

    #[test]
    fn amend_override_unknown_slice_refused() {
        // per-slice authority override: unknown `--authority-override <slice>` must
        // refuse at exit 2 before any plan.yaml write happens.
        let project = Project::init();
        project.seed_plan(AUTHORITY_OVERRIDE_PLAN);
        let before = fs::read_to_string(project.plan_path()).expect("read plan");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args([
                "--format",
                "json",
                "plan",
                "amend",
                "identity-user-registration",
                "--authority-override",
                "ghost-slice",
                "requirement=runtime",
            ])
            .assert()
            .failure();
        let code = assert.get_output().status.code().expect("exit code");
        assert_eq!(code, 2, "unknown slice must exit 2 (validation_failed)");

        let after = fs::read_to_string(project.plan_path()).expect("read plan");
        assert_eq!(before, after, "plan.yaml must be unchanged on refusal");
        assert!(read_journal_lines(&project).is_empty(), "no journal events on the refused write");
    }

    #[test]
    fn plan_amend_override_bad_kind_refused() {
        // per-slice authority override: `<kind>` is validated against the closed
        // ClaimKind enum at the CLI boundary — clap surfaces a usage
        // diagnostic (exit 2) before any plan mutation runs.
        let project = Project::init();
        project.seed_plan(AUTHORITY_OVERRIDE_PLAN);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args([
                "plan",
                "amend",
                "identity-user-registration",
                "--authority-override",
                "identity-user-registration",
                "bogus-kind=runtime",
            ])
            .assert()
            .failure();
        let code = assert.get_output().status.code().expect("exit code");
        assert_eq!(code, 2, "invalid kind must exit 2");
        // The kind enum is enforced inside our argument parser (not by
        // clap's value_parser), so the error surfaces as a plain
        // `Error::Argument` whose stderr is human text rather than
        // JSON. We assert the exit code and the human message body.
        let stderr_str = String::from_utf8_lossy(&assert.get_output().stderr);
        assert!(
            stderr_str.contains("bogus-kind"),
            "expected the bad kind name to appear in stderr, got:\n{stderr_str}"
        );
    }
}
