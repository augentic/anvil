//! Integration tests for `specify init` (adapter and `--workspace` modes).
//!
//! Covers the on-disk shape produced by `init`, the JSON envelope, and
//! the clap-level invariants around the positional `<adapter>`
//! argument and the `--workspace` flag.

use std::fs;
use std::path::{Path, PathBuf};

use specify_workflow::config::ProjectConfig;
use tempfile::tempdir;

use crate::common::{copy_dir, omnia_schema_dir, repo_root, snapshot_tree, specify_cmd};

/// In-repo vectis stub target adapter that declares
/// `platforms: { required: true, allowed: [core, ios, android, web, desktop] }`.
fn vectis_stub_dir() -> PathBuf {
    repo_root().join("tests/fixtures/adapters/targets/vectis-stub")
}

#[test]
fn init_text_format_succeeds() {
    let tmp = tempdir().unwrap();
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .arg(omnia_schema_dir())
        .args(["--name", "demo"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    assert!(stdout.contains("Initialized"));
    assert!(stdout.contains("omnia"));
    assert!(stdout.contains(".specify/project.yaml"));

    let config_path = tmp.path().join(".specify/project.yaml");
    assert!(config_path.is_file(), "project.yaml must exist");
}

#[test]
fn init_json_format_has_stable_shape() {
    let tmp = tempdir().unwrap();
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "init"])
        .arg(omnia_schema_dir())
        .args(["--name", "demo"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is JSON");

    assert_eq!(value["adapter-name"], "omnia");
    assert!(value["config-path"].is_string());
    let config_path = value["config-path"].as_str().unwrap();
    // Canonicalized tmp path so substring match handles macOS
    // /private/var symlinks gracefully.
    let canonical_tmp = fs::canonicalize(tmp.path()).expect("canonicalize tmp");
    assert!(
        config_path.starts_with(canonical_tmp.to_string_lossy().as_ref()),
        "config_path {config_path} should start with {}",
        canonical_tmp.display()
    );
    assert!(value["specify-version"].is_string());
    assert!(value["scaffolded-rule-keys"].is_array());
}

#[test]
#[ignore = "networked GitHub fetch smoke test"]
fn init_github_directory_uri_succeeds() {
    let tmp = tempdir().unwrap();
    specify_cmd()
        .current_dir(tmp.path())
        .args([
            "init",
            "https://github.com/augentic/specify/adapters/targets/omnia",
            "--name",
            "demo",
        ])
        .assert()
        .success();
}

#[test]
#[ignore = "networked GitHub fetch smoke test"]
fn init_shorthand_resolves_via_github() {
    // `specify init omnia@1.0.0` resolves the first-party shorthand to the
    // published adapter on GitHub (sparse checkout). Networked.
    let tmp = tempdir().unwrap();
    specify_cmd()
        .current_dir(tmp.path())
        .args(["init", "omnia@1.0.0", "--name", "demo"])
        .assert()
        .success();
}

// ---- `specify init` adapter/workspace invariant: positional <adapter> + --workspace mutual exclusion ----

#[test]
fn init_writes_adapter_field_for_url_arg() {
    // Acceptance (a): `specify init <url>` writes `adapter: <url>`
    // and no `schema:` field; `workspace:` either absent or false.
    let tmp = tempdir().unwrap();
    specify_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .arg(omnia_schema_dir())
        .args(["--name", "demo"])
        .assert()
        .success();

    let project_yaml =
        fs::read_to_string(tmp.path().join(".specify/project.yaml")).expect("read project.yaml");
    assert!(
        project_yaml.contains("adapter:"),
        "project.yaml must carry `adapter:` after regular init, got:\n{project_yaml}"
    );
    assert!(
        !project_yaml.lines().any(|line| line.trim_start().starts_with("schema:")),
        "project.yaml must NOT carry the legacy `schema:` field, got:\n{project_yaml}"
    );
    // workspace: absent (or false) means the value is implicit; just check no
    // `workspace: true` line.
    assert!(
        !project_yaml.lines().any(|l| l.trim_start().starts_with("workspace: true")),
        "regular init must not write `workspace: true`, got:\n{project_yaml}"
    );

    // Regular init writes only `project.yaml` and the `.specify/`
    // skeleton at the project root. Platform-component artefacts at the
    // repo root are operator-managed.
    for absent in ["registry.yaml", "plan.yaml", "change.md"] {
        assert!(
            !tmp.path().join(absent).exists(),
            "regular init must not pre-touch `{absent}` at the repo root"
        );
    }
}

// ---- `specify init --platforms` (RFC: project platform set) ----

#[test]
fn init_platforms_persists_declared_set() {
    // Happy path: a target that requires platforms accepts a valid
    // `--platforms core,ios,android` set and persists it verbatim into
    // `project.yaml.platforms`. Init does not scaffold shell trees — the
    // declared set is the contract the later bootstrap-slice flow reads.
    let tmp = tempdir().unwrap();
    let adapter = tmp.path().join("adapters/targets/vectis-stub");
    copy_dir(&vectis_stub_dir(), &adapter);

    specify_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .arg(&adapter)
        .args(["--name", "platform-app", "--platforms", "core,ios,android"])
        .assert()
        .success();

    let cfg = ProjectConfig::load(tmp.path()).expect("reload project.yaml");
    let declared: Vec<String> = cfg.platforms.iter().map(ToString::to_string).collect();
    assert_eq!(
        declared,
        vec!["core", "ios", "android"],
        "init must persist the declared --platforms set verbatim"
    );
}

#[test]
fn init_platforms_not_allowed_errors() {
    // Error path: a platform outside the target's `allowed` set aborts
    // with the `project-platforms-not-allowed` validation discriminant
    // (exit 2) and never scaffolds the project.
    let tmp = tempdir().unwrap();
    let adapter = tmp.path().join("adapters/targets/adapter-limited");
    fs::create_dir_all(adapter.join("briefs")).unwrap();
    fs::write(
        adapter.join("adapter.yaml"),
        "name: adapter-limited\nversion: 1.0.0\naxis: target\nexecution: agent\nbriefs:\n  shape: briefs/shape.md\n  build: briefs/build.md\n  merge: briefs/merge.md\ndescription: Stub adapter that only allows core + ios\nplatforms:\n  required: true\n  allowed: [core, ios]\n  default: [core, ios]\n",
    )
    .unwrap();
    for brief in ["shape.md", "build.md", "merge.md"] {
        fs::write(adapter.join("briefs").join(brief), "# Stub\n").unwrap();
    }

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "init"])
        .arg(&adapter)
        .args(["--name", "demo", "--platforms", "core,ios,android"])
        .assert()
        .failure();

    assert_eq!(
        assert.get_output().status.code(),
        Some(2),
        "a disallowed platform maps to the validation exit code"
    );
    let envelope: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stderr).expect("stderr is the JSON envelope");
    assert_eq!(envelope["error"], "project-platforms-not-allowed");
    assert_eq!(envelope["exit-code"], 2);
    assert!(
        !tmp.path().join(".specify/project.yaml").exists(),
        "a rejected init must not scaffold the project"
    );
}

// ---- `specify init` AGENTS.md context fences + context.lock ----

#[test]
fn init_writes_agents_fences_and_lock() {
    // A greenfield init both renders the fenced `AGENTS.md` context
    // block and writes the `.specify/context.lock` fingerprint sidecar
    // the re-generation flow diffs against. `tests/init_shapes.rs`
    // covers the `.specify/` skeleton dirs but neither of these two
    // generated artifacts.
    let tmp = tempdir().unwrap();
    specify_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .arg(omnia_schema_dir())
        .args(["--name", "fenced-proj"])
        .assert()
        .success();

    let agents =
        fs::read_to_string(tmp.path().join("AGENTS.md")).expect("AGENTS.md must be written");
    assert!(
        agents.contains("<!-- specify:context begin")
            && agents.contains("<!-- specify:context end -->"),
        "AGENTS.md must carry both Specify context-fence markers, got:\n{agents}"
    );

    let lock_path = tmp.path().join(".specify/context.lock");
    assert!(lock_path.is_file(), ".specify/context.lock must be written on greenfield init");
    let lock: serde_json::Value =
        serde_saphyr::from_str(&fs::read_to_string(&lock_path).expect("read context.lock"))
            .expect("context.lock parses as YAML");
    assert_eq!(lock["version"], 1, "context.lock must pin the v1 schema marker");
    assert!(
        lock["fingerprint"].as_str().is_some(),
        "context.lock must carry an aggregate fingerprint, got:\n{lock}"
    );
}

#[test]
fn reinit_skips_agents_keeps_existing() {
    // The init-time AGENTS.md generator runs only when the file is
    // absent (always a `create`). A re-entry `--upgrade` over a project
    // that already carries an AGENTS.md must skip regeneration — never
    // clobbering operator edits — and surface the skip in the envelope.
    // The greenfield smoke above pins the `create` artifacts; this pins
    // the envelope `context-*` fields and the idempotent skip path.
    let tmp = tempdir().unwrap();

    let created = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "init"])
        .arg(omnia_schema_dir())
        .args(["--name", "fenced-proj"])
        .assert()
        .success();
    let created_env: serde_json::Value =
        serde_json::from_slice(&created.get_output().stdout).expect("init stdout is JSON");
    assert_eq!(created_env["context-generated"], true, "greenfield init generates AGENTS.md");
    assert_eq!(created_env["context-skipped"], false);

    // Operator appends content; a faithful re-init must not lose it.
    let agents_path = tmp.path().join("AGENTS.md");
    let mut edited = fs::read_to_string(&agents_path).expect("read AGENTS.md");
    edited.push_str("\n<!-- operator note: keep me -->\n");
    fs::write(&agents_path, &edited).expect("append operator note");

    let upgraded = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "init", "--upgrade"])
        .assert()
        .success();
    let upgraded_env: serde_json::Value =
        serde_json::from_slice(&upgraded.get_output().stdout).expect("upgrade stdout is JSON");
    assert_eq!(upgraded_env["context-generated"], false, "re-init must not regenerate AGENTS.md");
    assert_eq!(upgraded_env["context-skipped"], true);
    assert_eq!(upgraded_env["context-skip-reason"], "existing-agents-md");

    let after = fs::read_to_string(&agents_path).expect("read AGENTS.md after upgrade");
    assert!(
        after.contains("<!-- operator note: keep me -->"),
        "re-init must preserve operator content, got:\n{after}"
    );
    assert!(
        after.contains("<!-- specify:context begin")
            && after.contains("<!-- specify:context end -->"),
        "re-init must leave the context fences intact, got:\n{after}"
    );
}

#[test]
fn init_with_no_args_errors() {
    // Acceptance (c): `specify init` (no positional, no `--workspace`) must
    // exit `2` (clap's parse-error slot) with clap's standard
    // "required arguments were not provided" diagnostic. The historical
    // post-parse `init-requires-adapter-or-workspace` diagnostic was lifted
    // into the clap surface (`required_unless_present = "workspace"`).
    let tmp = tempdir().unwrap();
    let assert = specify_cmd().current_dir(tmp.path()).args(["init"]).assert().failure();
    assert_eq!(assert.get_output().status.code(), Some(2), "clap parse errors map to exit code 2");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("utf8");
    assert!(
        stderr.contains("required arguments were not provided") && stderr.contains("ADAPTER"),
        "diagnostic must surface clap's required-arg parse error, got stderr:\n{stderr}"
    );
    assert!(
        !tmp.path().join(".specify").exists(),
        "no .specify must be scaffolded on parse failure"
    );
}

#[test]
fn init_with_adapter_and_workspace_errors() {
    // Acceptance (d): `specify init <url> --workspace` must exit `2` with
    // clap's "the argument cannot be used with" diagnostic. Same
    // motivation as `init_with_no_args_errors`: the invariant lives in
    // clap (`conflicts_with = "workspace"`), not a post-parse diagnostic.
    let tmp = tempdir().unwrap();
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .arg(omnia_schema_dir())
        .arg("--workspace")
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2), "clap parse errors map to exit code 2");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("utf8");
    assert!(
        stderr.contains("cannot be used with") && stderr.contains("--workspace"),
        "diagnostic must mention the conflicts_with rule, got stderr:\n{stderr}"
    );
}

// ---- specify init --workspace (registry workspace topology) ----

#[test]
fn workspace_writes_canonical_shape() {
    let tmp = tempdir().unwrap();
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "init"])
        .args(["--name", "platform-workspace", "--workspace"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is JSON");
    assert_eq!(
        value["adapter-name"], "workspace",
        "JSON response must surface adapter-name: \"workspace\", got: {value}"
    );
    assert_eq!(value["workspace-synced"], true);
    assert_eq!(value["workspace-sync-message"], "workspace sync complete");
    assert!(
        value["scaffolded-rule-keys"].as_array().expect("array").is_empty(),
        "workspace init must not scaffold rule keys, got: {}",
        value["scaffolded-rule-keys"]
    );

    // Workspace init scaffolds `project.yaml` (under `.specify/`) plus
    // `registry.yaml` at the repo root, and nothing else. `registry.yaml`
    // survives because bootstrapping a workspace is bootstrapping its registry;
    // `change.md` and `plan.yaml` stay operator-managed.
    assert!(tmp.path().join(".specify/project.yaml").is_file());
    assert!(tmp.path().join("registry.yaml").is_file());
    for absent in ["plan.yaml", "change.md"] {
        assert!(
            !tmp.path().join(absent).exists(),
            "workspace init must not pre-touch `{absent}` at the repo root"
        );
    }
    // Phase-pipeline directories MUST NOT be present.
    assert!(!tmp.path().join(".specify/slices").exists());
    assert!(!tmp.path().join(".specify/specs").exists());
    assert!(!tmp.path().join(".specify/cache").exists());

    // project.yaml shape: `workspace: true` only, no `adapter:` field, and
    // no stale `schema:` sentinel.
    let project_yaml =
        fs::read_to_string(tmp.path().join(".specify/project.yaml")).expect("read project.yaml");
    assert!(
        !project_yaml.lines().any(|l| l.trim_start().starts_with("schema:")),
        "workspace project.yaml must omit the stale `schema:` field:\n{project_yaml}"
    );
    assert!(
        !project_yaml.lines().any(|l| l.trim_start().starts_with("adapter:")),
        "workspace project.yaml must omit the `adapter:` field:\n{project_yaml}"
    );
    assert!(
        project_yaml.contains("workspace: true"),
        "project.yaml must carry `workspace: true`:\n{project_yaml}"
    );

    // registry.yaml shape — version: 1, projects: [].
    let registry_yaml =
        fs::read_to_string(tmp.path().join("registry.yaml")).expect("read registry.yaml");
    assert!(
        registry_yaml.contains("version: 1"),
        "registry.yaml missing version:\n{registry_yaml}"
    );
    let registry: serde_json::Value =
        serde_yaml_to_json(&registry_yaml).expect("registry.yaml parses");
    assert_eq!(registry["version"], 1);
    assert!(
        registry["projects"].as_array().is_some_and(Vec::is_empty),
        "registry.yaml `projects` must be an empty list, got: {registry}"
    );

    // `change.md` is not scaffolded by workspace init; it appears only after
    // the operator runs `/spec:plan <name>` (or `specify plan create <name>`).
}

#[test]
fn init_workspace_refuses_when_present() {
    let tmp = tempdir().unwrap();
    // Pre-create `.specify/` with arbitrary content.
    fs::create_dir_all(tmp.path().join(".specify")).unwrap();
    fs::write(tmp.path().join(".specify/project.yaml"), "name: existing\nadapter: omnia\n")
        .unwrap();

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .args(["--name", "platform-workspace", "--workspace"])
        .assert()
        .failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("utf8");
    assert!(
        stderr.contains("refusing to scaffold"),
        "stderr should explain the refusal, got: {stderr:?}"
    );

    let on_disk = fs::read_to_string(tmp.path().join(".specify/project.yaml")).unwrap();
    assert_eq!(on_disk, "name: existing\nadapter: omnia\n");
}

// ---- `specify init --upgrade` (re-entry version bump) ----

/// This binary's version — the target every `--upgrade` bumps toward.
const BINARY_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Populate a brownfield regular project: an older pin (`0.1.0`; the
/// binary is a later `0.x`), a bare `adapter:`, a spread of operator
/// artifacts, and a sentinel `AGENTS.md`.
fn seed_brownfield_regular(root: &Path) {
    let specify = root.join(".specify");
    fs::create_dir_all(specify.join("slices/my-slice")).unwrap();
    fs::create_dir_all(specify.join("specs")).unwrap();
    fs::create_dir_all(specify.join("archive")).unwrap();
    fs::create_dir_all(specify.join("design-system")).unwrap();
    fs::write(
        specify.join("project.yaml"),
        "name: brownfield\ndescription: existing project\nadapter: omnia\nspecify_version: 0.1.0\nrules:\n  specs: specs.md\n",
    )
    .unwrap();
    fs::write(specify.join("slices/my-slice/spec.md"), "# operator slice\n").unwrap();
    fs::write(specify.join("specs/baseline.md"), "# baseline spec\n").unwrap();
    fs::write(specify.join("archive/old.md"), "# archived\n").unwrap();
    fs::write(
        specify.join("design-system/components.yaml"),
        "components:\n  - id: button\n    status: confirmed\n",
    )
    .unwrap();
    fs::write(root.join("AGENTS.md"), "# Sentinel AGENTS.md — operator authored\n").unwrap();
}

#[test]
fn upgrade_bumps_version_keeps_artifacts() {
    let tmp = tempdir().unwrap();
    seed_brownfield_regular(tmp.path());

    let before = snapshot_tree(tmp.path());
    let before_cfg: ProjectConfig = serde_saphyr::from_str(
        std::str::from_utf8(&before[Path::new(".specify/project.yaml")]).unwrap(),
    )
    .expect("parse before");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "init", "--upgrade"])
        .assert()
        .success();
    let value: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("json");
    assert_eq!(value["specify-version"], BINARY_VERSION);
    assert_eq!(value["specify-version-changed"], true);
    assert_eq!(value["adapter-name"], "omnia");

    let after = snapshot_tree(tmp.path());

    // Every path other than project.yaml is byte-identical, and the
    // path set is unchanged (nothing added, nothing removed).
    let project_yaml = PathBuf::from(".specify/project.yaml");
    let before_keys: Vec<_> = before.keys().filter(|k| **k != project_yaml).collect();
    let after_keys: Vec<_> = after.keys().filter(|k| **k != project_yaml).collect();
    assert_eq!(before_keys, after_keys, "upgrade must not add or remove files");
    for key in before_keys {
        assert_eq!(before[key], after[key], "file {} must be byte-identical", key.display());
    }

    // Within project.yaml only `specify_version` changed.
    let after_cfg: ProjectConfig =
        serde_saphyr::from_str(std::str::from_utf8(&after[&project_yaml]).unwrap())
            .expect("parse after");
    assert_eq!(after_cfg.specify_version.as_deref(), Some(BINARY_VERSION));
    let normalised = ProjectConfig {
        specify_version: before_cfg.specify_version.clone(),
        ..after_cfg
    };
    assert_eq!(normalised, before_cfg, "only specify_version may change in project.yaml");

    // Second run is a byte-stable no-op.
    let snapshot_after_first = snapshot_tree(tmp.path());
    let assert2 = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "init", "--upgrade"])
        .assert()
        .success();
    let value2: serde_json::Value =
        serde_json::from_slice(&assert2.get_output().stdout).expect("json");
    assert_eq!(value2["specify-version-changed"], false, "re-run must be a no-op");
    assert_eq!(
        snapshot_tree(tmp.path()),
        snapshot_after_first,
        "second --upgrade must leave the tree byte-identical"
    );
}

#[test]
fn upgrade_preserves_workspace_registry() {
    let tmp = tempdir().unwrap();
    let specify = tmp.path().join(".specify");
    fs::create_dir_all(&specify).unwrap();
    fs::write(
        specify.join("project.yaml"),
        "name: platform-workspace\nspecify_version: 0.1.0\nworkspace: true\n",
    )
    .unwrap();
    fs::write(tmp.path().join("registry.yaml"), "version: 1\nprojects: []\n").unwrap();
    fs::write(tmp.path().join("AGENTS.md"), "# Workspace sentinel\n").unwrap();

    let registry_before = fs::read(tmp.path().join("registry.yaml")).unwrap();

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "init", "--upgrade"])
        .assert()
        .success();
    let value: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("json");
    assert_eq!(value["specify-version"], BINARY_VERSION);
    assert_eq!(value["specify-version-changed"], true);
    assert_eq!(value["adapter-name"], "workspace");

    let cfg: ProjectConfig =
        serde_saphyr::from_str(&fs::read_to_string(specify.join("project.yaml")).unwrap())
            .expect("parse workspace project.yaml");
    assert!(cfg.workspace, "workspace discriminator must survive");
    assert!(cfg.adapter.is_none(), "workspace upgrade must not synthesise an adapter");
    assert_eq!(cfg.specify_version.as_deref(), Some(BINARY_VERSION));
    let project_yaml = fs::read_to_string(specify.join("project.yaml")).unwrap();
    assert!(project_yaml.contains("workspace: true"), "upgrade must preserve workspace: key");
    assert_eq!(
        fs::read(tmp.path().join("registry.yaml")).unwrap(),
        registry_before,
        "registry.yaml must be byte-identical after a workspace upgrade"
    );

    // Second run no-op.
    let project_after_first = fs::read(specify.join("project.yaml")).unwrap();
    specify_cmd().current_dir(tmp.path()).args(["init", "--upgrade"]).assert().success();
    assert_eq!(
        fs::read(specify.join("project.yaml")).unwrap(),
        project_after_first,
        "second workspace --upgrade must be byte-stable"
    );
}

#[test]
fn upgrade_conflicts_adapter_workspace() {
    for extra in [vec!["omnia"], vec!["--workspace"]] {
        let tmp = tempdir().unwrap();
        let mut cmd = specify_cmd();
        cmd.current_dir(tmp.path()).args(["init", "--upgrade"]).args(&extra);
        let assert = cmd.assert().failure();
        assert_eq!(
            assert.get_output().status.code(),
            Some(2),
            "clap conflict for `init --upgrade {}` maps to exit 2",
            extra.join(" ")
        );
        let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("utf8");
        assert!(
            stderr.contains("cannot be used with"),
            "diagnostic must surface clap's conflict for `--upgrade {}`, got:\n{stderr}",
            extra.join(" ")
        );
    }
}

/// Tiny YAML→JSON helper — we only need it for the workspace on-disk shape
/// assertion, and pulling in a full yaml dependency for one test is
/// overkill. The registry file we write is shallow so a minimal hand
/// parser via `serde_json::from_str` after an indent-stripped
/// transform would be fragile; instead we re-use `serde_saphyr` (the
/// crate the rest of the CLI uses) by routing through a `Value`.
fn serde_yaml_to_json(yaml: &str) -> Result<serde_json::Value, String> {
    let value: serde_json::Value =
        serde_saphyr::from_str(yaml).map_err(|err| format!("parse error: {err}"))?;
    Ok(value)
}
