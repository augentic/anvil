//! `specify plan validate` CLI tests: base shape rules, the
//! registry-shape hook, the planning-path smoke, and the surviving
//! health diagnostics (`cycle-in-depends-on`, `orphan-source`,
//! `stale-workspace-clone`, `topology-cache-stale`).

use crate::support::*;

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
    let matching: Vec<&Value> =
        findings.iter().filter(|r| r["rule-id"] == "missing-slice-dir-for-in-progress").collect();
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
        findings.iter().any(|r| r["rule-id"] == "duplicate-name" && r["severity"] == "important"),
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
        .arg(omnia_schema_dir())
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
    assert!(init.status.success(), "git init failed: {}", String::from_utf8_lossy(&init.stderr));
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
    copy_dir(
        &omnia_schema_dir(),
        &expected_cache_dir(&tmp.path().join("workspace/alpha")).join("manifests/targets/omnia"),
    );

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
    let value: Value =
        serde_json::from_str(&String::from_utf8(assert.get_output().stdout.clone()).expect("utf8"))
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
    let cycle_path = cycle["evidence"]["data"]["cycle"].as_array().expect("cycle path is array");
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

// ---- RFC-46 bootstrap app-icon gate (plan validate) --------------------

fn vectis_stub_dir() -> PathBuf {
    repo_root().join("tests/fixtures/adapters/targets/vectis-stub")
}

fn init_vectis_greenfield(platforms: &str) -> TempDir {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let adapter = root.join("adapters/targets/vectis");
    copy_dir(&vectis_stub_dir(), &adapter);
    let adapter_yaml =
        fs::read_to_string(adapter.join("adapter.yaml")).expect("read vectis-stub adapter.yaml");
    fs::write(
        adapter.join("adapter.yaml"),
        adapter_yaml.replace("name: vectis-stub", "name: vectis"),
    )
    .expect("patch adapter name to vectis");

    specify_cmd()
        .current_dir(root)
        .args(["init"])
        .arg(&adapter)
        .args(["--name", "platform-app", "--platforms", platforms])
        .assert()
        .success();

    fs::write(
        root.join("plan.yaml"),
        "name: platform-app\nsources:\n  intent:\n    adapter: intent\n    value: \"Add a feature.\"\nslices:\n  - name: add-feature\n    status: pending\n",
    )
    .expect("write plan.yaml");
    tmp
}

#[test]
fn app_icon_missing_greenfield() {
    let tmp = init_vectis_greenfield("core,ios,android");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "plan", "validate"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));

    let value = parse_stdout(&assert.get_output().stdout, tmp.path());
    let findings = value["findings"].as_array().expect("findings array");
    let hits: Vec<&Value> =
        findings.iter().filter(|r| r["rule-id"] == "plan-bootstrap-app-icon-missing").collect();
    assert_eq!(hits.len(), 2, "ios and android must each fail: {findings:#?}");
    assert!(hits.iter().all(|r| r["severity"] == "important"));
}

#[test]
fn app_icon_valid_assets_passes() {
    let tmp = init_vectis_greenfield("core,ios,android");
    let design = tmp.path().join("design-system");
    fs::create_dir_all(design.join("assets")).expect("mkdir assets");
    fs::write(
        design.join("assets.yaml"),
        "version: 1\n\
         app-icon: app-icon\n\
         assets:\n\
         \x20\x20app-icon:\n\
         \x20\x20\x20\x20kind: vector\n\
         \x20\x20\x20\x20role: app-icon\n\
         \x20\x20\x20\x20source: assets/app-icon.svg\n",
    )
    .expect("write assets.yaml");
    fs::write(design.join("assets/app-icon.svg"), "<svg/>").expect("write svg");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "plan", "validate"])
        .assert()
        .success();
    let value = parse_stdout(&assert.get_output().stdout, tmp.path());
    let findings = value["findings"].as_array().expect("findings array");
    assert!(
        !findings.iter().any(|r| r["rule-id"] == "plan-bootstrap-app-icon-missing"),
        "valid path A master must not emit app-icon gate: {findings:#?}"
    );
}

#[test]
fn app_icon_pinned_exports_passes() {
    let tmp = init_vectis_greenfield("core,ios,android");
    let design = tmp.path().join("design-system");
    fs::create_dir_all(design.join("assets/exports/ios/app-icon/AppIcon.appiconset"))
        .expect("mkdir ios export");
    fs::create_dir_all(design.join("assets/exports/android/app-icon"))
        .expect("mkdir android export");
    fs::write(
        design.join("assets.yaml"),
        "version: 1\n\
         app-icon: app-icon\n\
         assets:\n\
         \x20\x20app-icon:\n\
         \x20\x20\x20\x20kind: vector\n\
         \x20\x20\x20\x20role: app-icon\n\
         \x20\x20\x20\x20sources:\n\
         \x20\x20\x20\x20\x20\x20ios: assets/exports/ios/app-icon/AppIcon.appiconset\n\
         \x20\x20\x20\x20\x20\x20android: assets/exports/android/app-icon\n",
    )
    .expect("write assets.yaml");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "plan", "validate"])
        .assert()
        .success();
    let value = parse_stdout(&assert.get_output().stdout, tmp.path());
    let findings = value["findings"].as_array().expect("findings array");
    assert!(
        !findings.iter().any(|r| r["rule-id"] == "plan-bootstrap-app-icon-missing"),
        "path B pinned exports must satisfy §6.2 without a `source:` master: {findings:#?}"
    );
}

#[test]
fn app_icon_shell_resident_passes() {
    let tmp = init_vectis_greenfield("core,ios,android");
    let appiconset = tmp.path().join("iOS/Demo/Resources/Assets.xcassets/AppIcon.appiconset");
    fs::create_dir_all(&appiconset).expect("mkdir appiconset");
    fs::write(
        appiconset.join("Contents.json"),
        r#"{"images":[{"filename":"AppIcon.png","idiom":"universal"}]}"#,
    )
    .expect("write contents");
    fs::write(appiconset.join("AppIcon.png"), minimal_app_icon_png()).expect("write png");

    let res_dir = tmp.path().join("Android/app/src/main/res");
    fs::create_dir_all(res_dir.join("mipmap-anydpi-v26")).expect("mkdir v26");
    fs::write(res_dir.join("mipmap-anydpi-v26/ic_launcher.xml"), "<adaptive-icon/>")
        .expect("write launcher xml");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "plan", "validate"])
        .assert()
        .success();
    let value = parse_stdout(&assert.get_output().stdout, tmp.path());
    let findings = value["findings"].as_array().expect("findings array");
    assert!(
        !findings.iter().any(|r| r["rule-id"] == "plan-bootstrap-app-icon-missing"),
        "shell-resident escape hatch must satisfy §6.3 without assets.yaml: {findings:#?}"
    );
}

fn minimal_app_icon_png() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}
