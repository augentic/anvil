//! Integration tests for `specify registry` (registry add/remove).
//!
//! Covers `registry add`, `registry remove`, the workspace `registry
//! validate` invariants surfaced after a fresh `init --workspace`, and the
//! schema-level `registry {show,validate}` matrix (registry show/validate) on
//! single- and multi-project layouts.

use std::fs;

use serde_json::Value;
use tempfile::tempdir;

mod common;
use common::{Project, init_workspace, omnia_schema_dir, parse_stderr, parse_stdout, specify_cmd};

#[test]
fn workspace_validate_succeeds_empty() {
    let tmp = tempdir().unwrap();
    specify_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .args(["--name", "platform-workspace", "--workspace"])
        .assert()
        .success();

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "registry", "validate"])
        .assert()
        .success();
    let value: Value = serde_json::from_slice(&assert.get_output().stdout).expect("json");
    assert!(value["error"].is_null(), "success envelope must omit error: {value}");
}

#[test]
fn init_workspace_validate_rejects_dot_url() {
    let tmp = tempdir().unwrap();
    specify_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .args(["--name", "platform-workspace", "--workspace"])
        .assert()
        .success();

    // Stomp the registry to pretend the operator hand-edited a `url: .`
    // entry. Hub-mode validation must surface the
    // `workspace-cannot-be-project` diagnostic.
    fs::write(
        tmp.path().join("registry.yaml"),
        "version: 1\n\
         projects:\n\
         \x20\x20- name: platform\n\
         \x20\x20\x20\x20url: .\n\
         \x20\x20\x20\x20adapter: invalid-adapter\n",
    )
    .unwrap();

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "registry", "validate"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let value: Value = serde_json::from_slice(&assert.get_output().stderr).expect("json");
    assert_eq!(
        value["error"], "workspace-cannot-be-project",
        "error must carry the stable diagnostic code, got: {value}"
    );
    let msg = value["message"].as_str().expect("message string");
    assert!(msg.contains("registry.yaml"), "message must scope the file: {msg}");
}

// ---- specify registry {add, remove} (registry add/remove) ----

#[test]
fn add_round_trips_through_show() {
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");

    // Hub registries reject `url: .` (workspace-cannot-be-project) but accept
    // remote-url entries — the canonical multi-repo shape.
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args([
            "--format",
            "json",
            "registry",
            "add",
            "alpha",
            "--url",
            "git@github.com:augentic/alpha.git",
            "--adapter",
            "omnia@1.0.0",
            "--description",
            "Alpha service",
        ])
        .assert()
        .success();
    let value: Value = serde_json::from_slice(&assert.get_output().stdout).expect("json");
    assert!(value["error"].is_null(), "success envelope must omit error: {value}");
    assert_eq!(value["added"]["name"], "alpha");
    assert_eq!(value["added"]["url"], "git@github.com:augentic/alpha.git");
    assert_eq!(value["added"]["adapter"], "omnia@1.0.0");
    assert_eq!(value["added"]["description"], "Alpha service");
    assert_eq!(value["registry"]["projects"].as_array().unwrap().len(), 1);

    let loaded = fs::read_to_string(tmp.path().join("registry.yaml")).expect("read registry");
    let value: Value = serde_saphyr::from_str(&loaded).expect("registry yaml");
    let projects = value["projects"].as_array().expect("projects array");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["name"], "alpha");
}

#[test]
fn add_succeeds_without_adapter() {
    // `--adapter` is an optional greenfield seed. Omitting it is
    // legal — the project's own `project.yaml` authors its target adapter.
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args([
            "--format",
            "json",
            "registry",
            "add",
            "alpha",
            "--url",
            "git@github.com:augentic/alpha.git",
        ])
        .assert()
        .success();
    let value: Value = serde_json::from_slice(&assert.get_output().stdout).expect("json");
    assert!(value["error"].is_null(), "success envelope must omit error: {value}");
    assert_eq!(value["added"]["name"], "alpha");

    let loaded = fs::read_to_string(tmp.path().join("registry.yaml")).expect("read registry");
    assert!(!loaded.contains("adapter:"), "omitted seed must not write an adapter key: {loaded}");
}

#[test]
fn add_rejects_dot_url_in_workspace_mode() {
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args([
            "--format",
            "json",
            "registry",
            "add",
            "self",
            "--url",
            ".",
            "--adapter",
            "omnia@1.0.0",
        ])
        .assert()
        .failure();
    let value: Value = serde_json::from_slice(&assert.get_output().stderr).expect("json");
    assert_eq!(value["error"], "workspace-cannot-be-project");
    let msg = value["message"].as_str().expect("message");
    assert!(
        msg.contains("workspace-cannot-be-project"),
        "diagnostic must carry the stable code, got: {msg}"
    );
}

#[test]
fn add_rejects_non_kebab() {
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args([
            "--format",
            "json",
            "registry",
            "add",
            "BadName",
            "--url",
            "git@github.com:org/bad.git",
            "--adapter",
            "omnia@1.0.0",
        ])
        .assert()
        .failure();
    let value: Value = serde_json::from_slice(&assert.get_output().stderr).expect("json");
    assert_eq!(value["error"], "registry-add-name-not-kebab");
    let msg = value["message"].as_str().expect("message");
    assert!(msg.contains("kebab-case"), "diagnostic must mention kebab-case, got: {msg}");
    assert!(msg.contains("BadName"), "diagnostic must echo the bad name, got: {msg}");
}

#[test]
fn remove_succeeds_and_round_trips() {
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");

    // Seed two entries — both descriptions present so the registry is
    // multi-repo–valid.
    for (name, url) in
        [("alpha", "git@github.com:org/alpha.git"), ("beta", "git@github.com:org/beta.git")]
    {
        specify_cmd()
            .current_dir(tmp.path())
            .args([
                "--format",
                "json",
                "registry",
                "add",
                name,
                "--url",
                url,
                "--adapter",
                "omnia@1.0.0",
                "--description",
                &format!("{name} service"),
            ])
            .assert()
            .success();
    }

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "registry", "remove", "beta"])
        .assert()
        .success();
    let value: Value = serde_json::from_slice(&assert.get_output().stdout).expect("json");
    assert!(value["error"].is_null(), "success envelope must omit error: {value}");
    assert_eq!(value["removed"], "beta");
    assert!(
        value["warnings"].as_array().expect("warnings array").is_empty(),
        "no plan.yaml present, warnings must be empty: {value}"
    );
    let projects = value["registry"]["projects"].as_array().expect("projects array");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["name"], "alpha");
}

#[test]
fn remove_warns_on_plan_ref() {
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");

    for (name, url) in
        [("alpha", "git@github.com:org/alpha.git"), ("beta", "git@github.com:org/beta.git")]
    {
        specify_cmd()
            .current_dir(tmp.path())
            .args([
                "--format",
                "json",
                "registry",
                "add",
                name,
                "--url",
                url,
                "--adapter",
                "omnia@1.0.0",
                "--description",
                &format!("{name} service"),
            ])
            .assert()
            .success();
    }

    // Author a plan with one entry pointing at alpha. The merged
    // `specify plan create` scaffolds plan.yaml (change.md scaffold moved to /spec:plan).
    specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "plan", "create", "demo"])
        .assert()
        .success();
    specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "plan", "add", "alpha-feature", "--project", "alpha"])
        .assert()
        .success();

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "registry", "remove", "alpha"])
        .assert()
        .success();
    let value: Value = serde_json::from_slice(&assert.get_output().stdout).expect("json");
    assert!(value["error"].is_null(), "success envelope must omit error: {value}");
    assert_eq!(value["removed"], "alpha");
    let warnings = value["warnings"].as_array().expect("warnings array");
    assert_eq!(warnings.len(), 1, "expected a single warning, got: {value}");
    let warning = warnings[0].as_str().expect("warning string");
    assert!(warning.contains("alpha-feature"), "warning must name the entry, got: {warning}");
    assert!(warning.contains("plan amend"), "warning must hint at remediation, got: {warning}");
}

#[test]
fn remove_unknown_project_errors() {
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "registry", "remove", "ghost"])
        .assert()
        .failure();
    let value: Value = serde_json::from_slice(&assert.get_output().stderr).expect("json");
    assert_eq!(value["error"], "registry-remove-not-found");
    let msg = value["message"].as_str().expect("message");
    assert!(msg.contains("not found"), "msg: {msg}");
    assert!(msg.contains("ghost"), "msg: {msg}");
}

#[test]
fn remove_refuses_when_absent() {
    let tmp = tempdir().unwrap();
    // Plain init (no workspace) — single-repo project has no registry.yaml
    // by default.
    specify_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .arg(omnia_schema_dir())
        .args(["--name", "demo"])
        .assert()
        .success();
    assert!(!tmp.path().join("registry.yaml").exists());

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "registry", "remove", "alpha"])
        .assert()
        .failure();
    let value: Value = serde_json::from_slice(&assert.get_output().stderr).expect("json");
    assert_eq!(value["error"], "registry-remove-no-registry");
    let msg = value["message"].as_str().expect("message");
    assert!(msg.contains("no registry declared"), "msg: {msg}");
}

// ---- Registry (registry validation) ----

#[test]
fn load_from_tempdir() {
    use specify_workflow::registry::Registry;

    let project = Project::init();
    let registry_path = project.root().join("registry.yaml");
    fs::write(
        &registry_path,
        "version: 1\n\
             projects:\n\
             \x20\x20- name: traffic\n\
             \x20\x20\x20\x20url: .\n\
             \x20\x20\x20\x20adapter: omnia@1.0.0\n",
    )
    .expect("write registry.yaml");

    let loaded =
        Registry::load(project.root()).expect("registry parses").expect("registry present");
    assert_eq!(loaded.version, 1);
    assert_eq!(loaded.projects.len(), 1);
    assert_eq!(loaded.projects[0].name, "traffic");
    assert_eq!(loaded.projects[0].url, ".");
    assert_eq!(loaded.projects[0].adapter.as_deref(), Some("omnia@1.0.0"));
    assert!(loaded.is_single_repo());
}

// ---- Registry CLI verbs (registry CLI verbs) ----
//
// `specify registry validate` isolates the same shape check the C12 hook drives through
// `specify plan validate`. The tests below cover the full
// matrix: absent / well-formed / malformed × text / json.

const REGISTRY_SINGLE: &str = "\
version: 1
projects:
  - name: traffic
    url: .
    adapter: omnia@1.0.0
";

const REGISTRY_THREE: &str = "\
version: 1
projects:
  - name: monolith
    url: .
    adapter: omnia@1.0.0
    description: Core monolith service
  - name: orders
    url: ../orders
    adapter: omnia@1.0.0
    description: Order management service
  - name: payments
    url: git@github.com:org/payments.git
    adapter: omnia@1.0.0
    description: Payment processing service
";

fn write_registry(project: &Project, body: &str) {
    fs::write(project.root().join("registry.yaml"), body).expect("write registry");
}

#[test]
fn validate_absent() {
    let project = Project::init();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "registry", "validate"])
        .assert()
        .success();
    assert_eq!(assert.get_output().status.code(), Some(0));
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["registry"], Value::Null);
    assert!(actual["error"].is_null(), "success envelope must omit error: {actual}");

    let text =
        specify_cmd().current_dir(project.root()).args(["registry", "validate"]).assert().success();
    let stdout = std::str::from_utf8(&text.get_output().stdout).expect("utf8");
    assert!(
        stdout.contains("no registry declared"),
        "text validate should say 'no registry declared', got: {stdout:?}"
    );
}

#[test]
fn validate_well_formed() {
    let project = Project::init();
    write_registry(&project, REGISTRY_SINGLE);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "registry", "validate"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert!(actual["error"].is_null(), "success envelope must omit error: {actual}");
    let registry = actual["registry"].as_object().expect("registry object");
    assert_eq!(registry["version"], 1);
}

#[test]
fn validate_multi_project() {
    let project = Project::init();
    write_registry(&project, REGISTRY_THREE);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "registry", "validate"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert!(actual["error"].is_null(), "success envelope must omit error: {actual}");
    let projects = actual["registry"]["projects"].as_array().expect("projects array");
    assert_eq!(projects.len(), 3);
}

#[test]
fn validate_malformed_version() {
    let project = Project::init();
    write_registry(&project, "version: 2\nprojects: []\n");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "registry", "validate"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    // Failure envelopes are written to stderr.
    let actual = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(actual["error"], "registry-version-unsupported");
    let msg = actual["message"].as_str().expect("message string");
    assert!(msg.contains("version"), "message should mention version, got: {msg}");
    assert!(msg.contains("registry.yaml"), "message should mention registry.yaml, got: {msg}");
}

#[test]
fn validate_duplicate_name() {
    let project = Project::init();
    write_registry(
        &project,
        "\
version: 1
projects:
  - name: dup
    url: .
    adapter: omnia@1.0.0
  - name: dup
    url: ../other
    adapter: omnia@1.0.0
",
    );

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "registry", "validate"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    // Failure envelopes are written to stderr.
    let actual = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(actual["error"], "registry-project-name-duplicate");
    let msg = actual["message"].as_str().expect("message string");
    assert!(msg.contains("duplicate"), "message should mention duplicate, got: {msg}");
}

#[test]
fn validate_non_kebab() {
    let project = Project::init();
    write_registry(
        &project,
        "\
version: 1
projects:
  - name: NotKebab
    url: .
    adapter: omnia@1.0.0
",
    );

    let assert =
        specify_cmd().current_dir(project.root()).args(["registry", "validate"]).assert().failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
}

#[test]
fn validate_unknown_key() {
    let project = Project::init();
    write_registry(&project, "version: 1\nversions: 2\nprojects: []\n");

    let assert =
        specify_cmd().current_dir(project.root()).args(["registry", "validate"]).assert().failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
}

/// Plan "Done when" criterion: on a scaffolded project with no
/// registry, `specify registry validate` exits 0.
#[test]
fn validate_on_bare_repo() {
    let project = Project::init();
    assert!(!project.root().join("registry.yaml").exists(), "bare repo must not have a registry");
    specify_cmd().current_dir(project.root()).args(["registry", "validate"]).assert().success();
}
