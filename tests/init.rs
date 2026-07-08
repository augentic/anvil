//! Integration tests for `specify init` scaffolding and init shapes.
//! Shared helpers live in [`common`].

mod base {
    //! Integration tests for `specify init` (adapter and `--workspace` modes).
    //!
    //! Covers the on-disk shape produced by `init`, the JSON envelope, and
    //! the clap-level invariants around the positional `<adapter>`
    //! argument and the `--workspace` flag.

    use std::fs;
    use std::path::{Path, PathBuf};

    use specify_workflow::config::ProjectConfig;
    use tempfile::tempdir;

    use crate::common::{
        expected_cache_dir, fixture_component, omnia_component, snapshot_tree, specify_cmd,
    };

    #[test]
    fn init_text_format_succeeds() {
        let tmp = tempdir().unwrap();
        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["init"])
            .arg(omnia_component())
            .args(["--name", "demo"])
            .assert()
            .success();

        let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
        assert!(stdout.contains("Initialized"));
        assert!(stdout.contains("omnia"));
        assert!(stdout.contains(".specify/project.yaml"));

        let config_path = tmp.path().join(".specify/project.yaml");
        assert!(config_path.is_file(), "project.yaml must exist");

        // Init is an RFC-65 manifest trigger: the deployment manifest
        // is generated into the per-project cache, covering the
        // mirrored local component as a bare-name adapter guest.
        let manifest = expected_cache_dir(tmp.path()).join("deployment").join("omnia.toml");
        let doc = fs::read_to_string(&manifest).expect("generated deployment manifest");
        assert!(doc.contains("id = \"target:omnia\""), "the mirrored component deploys: {doc}");
    }

    #[test]
    fn init_json_format_has_stable_shape() {
        let tmp = tempdir().unwrap();
        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "init"])
            .arg(omnia_component())
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
        // Postflight fields (RFC-65 operator onboarding): the hydrated
        // set, the store root, and the literal next command are part
        // of the stable envelope.
        assert!(value["hydrated"].is_array());
        assert!(value["adapter-store"].is_string());
        assert_eq!(value["next"], "/spec:plan <name>");
    }

    #[test]
    fn init_github_uri_refused_honestly() {
        // RFC-64: a GitHub source checkout no longer yields a usable
        // adapter artifact, so the URI form fails fast with the typed
        // `adapter-github-uri-unsupported` discriminant instead of
        // fetching a tree it cannot use.
        let tmp = tempdir().unwrap();
        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args([
                "--format",
                "json",
                "init",
                "https://github.com/augentic/specify-adapters",
                "--name",
                "demo",
            ])
            .assert()
            .failure();
        let envelope: serde_json::Value = serde_json::from_slice(&assert.get_output().stderr)
            .expect("stderr is the JSON envelope");
        assert_eq!(envelope["error"], "adapter-github-uri-unsupported");
    }

    #[test]
    #[ignore = "networked wasm-pkg registry fetch smoke test"]
    fn init_shorthand_resolves_via_registry() {
        // `specify init omnia@1.0.0` resolves the first-party shorthand
        // to the published `specify:omnia@1.0.0` component via wasm-pkg.
        // Networked.
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
            .arg(omnia_component())
            .args(["--name", "demo"])
            .assert()
            .success();

        let project_yaml = fs::read_to_string(tmp.path().join(".specify/project.yaml"))
            .expect("read project.yaml");
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
        specify_cmd()
            .current_dir(tmp.path())
            .args(["init"])
            .arg(fixture_component("vectis-platforms"))
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
        // The echo target guest's `describe` answers the `…limited…` id
        // with `platforms: { required: true, allowed: [core, ios] }`.
        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "init"])
            .arg(fixture_component("adapter-limited"))
            .args(["--name", "demo", "--platforms", "core,ios,android"])
            .assert()
            .failure();

        assert_eq!(
            assert.get_output().status.code(),
            Some(2),
            "a disallowed platform maps to the validation exit code"
        );
        let envelope: serde_json::Value = serde_json::from_slice(&assert.get_output().stderr)
            .expect("stderr is the JSON envelope");
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
            .arg(omnia_component())
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
            .arg(omnia_component())
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
        assert_eq!(
            upgraded_env["context-generated"], false,
            "re-init must not regenerate AGENTS.md"
        );
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
        // Acceptance (c): `specify init` (no positional, no `--workspace`)
        // off a TTY must exit `2` with the typed `init-adapter-required`
        // naming the missing argument and both alternatives. The
        // requiredness lives in the RFC-65 elicitation layer (so a TTY
        // can prompt instead of failing), not in clap — stdin is a pipe
        // under the test harness, so the non-interactive leg fires.
        let tmp = tempdir().unwrap();
        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "init"])
            .assert()
            .failure();
        assert_eq!(
            assert.get_output().status.code(),
            Some(2),
            "missing-flag errors map to the validation exit code"
        );
        let envelope: serde_json::Value = serde_json::from_slice(&assert.get_output().stderr)
            .expect("stderr is the JSON envelope");
        assert_eq!(envelope["error"], "init-adapter-required");
        assert_eq!(envelope["exit-code"], 2);
        let message = envelope["message"].as_str().expect("message");
        assert!(
            message.contains("<adapter>") && message.contains("--workspace"),
            "error names the missing positional and the workspace alternative: {message}"
        );
        assert!(
            !tmp.path().join(".specify").exists(),
            "no .specify must be scaffolded on the missing-flag failure"
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
            .arg(omnia_component())
            .arg("--workspace")
            .assert()
            .failure();
        assert_eq!(
            assert.get_output().status.code(),
            Some(2),
            "clap parse errors map to exit code 2"
        );
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
        let project_yaml = fs::read_to_string(tmp.path().join(".specify/project.yaml"))
            .expect("read project.yaml");
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
        "name: brownfield\ndescription: existing project\nadapter: omnia\nspecify: 0.1.0\nrules:\n  specs: specs.md\n",
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

        // Within project.yaml only `specify` changed.
        let after_cfg: ProjectConfig =
            serde_saphyr::from_str(std::str::from_utf8(&after[&project_yaml]).unwrap())
                .expect("parse after");
        assert_eq!(after_cfg.specify_version.as_deref(), Some(BINARY_VERSION));
        let normalised = ProjectConfig {
            specify_version: before_cfg.specify_version.clone(),
            ..after_cfg
        };
        assert_eq!(normalised, before_cfg, "only specify may change in project.yaml");

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
            "name: platform-workspace\nspecify: 0.1.0\nworkspace: true\n",
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
}

mod shapes {
    //! Acceptance matrix for the three `specify init` shapes:
    //! `greenfield`, `brownfield`, and `workspace`.
    //!
    //! Each test drives the real `specify` binary over a throwaway tempdir
    //! and asserts the on-disk + JSON-envelope contract for one shape:
    //!
    //! - `greenfield` — a fresh `specify init <adapter>` over an empty dir
    //!   scaffolds `.specify/` and pins the current `specify` version.
    //! - `brownfield` — `specify init --upgrade` over a populated regular
    //!   project bumps only the pin, keeps operator artifacts byte-stable,
    //!   and re-runs as a no-op.
    //! - `workspace` — the same re-entry over a populated workspace,
    //!   with the `workspace: true` discriminator and `registry.yaml` preserved.
    //!
    //! The `brownfield` / `workspace` headline invariants are also covered with an
    //! exhaustive byte-level write-set diff by
    //! `init_upgrade_bumps_only_version_and_preserves_artifacts` and
    //! `init_upgrade_preserves_workspace_and_registry` in `tests/init.rs`; the
    //! versions here keep all three shapes co-located as one readable matrix.

    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use specify_workflow::config::ProjectConfig;
    use tempfile::tempdir;

    use crate::common::{omnia_component, parse_json, specify_cmd};

    /// Version this binary stamps into the `specify:` pin (the `specify`
    /// crate and this test crate share the workspace version).
    const BINARY_VERSION: &str = env!("CARGO_PKG_VERSION");

    // ---- greenfield ----

    #[test]
    fn greenfield() {
        let tmp = tempdir().unwrap();
        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "init"])
            .arg(omnia_component())
            .args(["--name", "greenfield-proj"])
            .assert()
            .success();
        let body = parse_json(&assert.get_output().stdout);
        assert_eq!(body["adapter-name"], "omnia");
        assert_eq!(body["specify-version"], BINARY_VERSION);
        assert_eq!(body["specify-version-changed"], true);

        // Fresh init scaffolds the canonical `.specify/` skeleton. The cache
        // is regenerable, machine-owned state and lives out-of-tree, so it is
        // no longer part of the in-tree skeleton.
        for dir in [".specify", ".specify/slices", ".specify/specs", ".specify/archive"] {
            assert!(tmp.path().join(dir).is_dir(), "greenfield must scaffold {dir}");
        }

        let cfg = load_cfg(tmp.path());
        assert_eq!(cfg.specify_version.as_deref(), Some(BINARY_VERSION));
        // `adapter:` persists the resolved source value (the fixture URI),
        // while the JSON envelope above carries the resolved name `omnia`.
        assert!(
            cfg.adapter.as_deref().is_some_and(|value| value.contains("omnia")),
            "greenfield must persist the omnia adapter, got {:?}",
            cfg.adapter,
        );
        assert!(!cfg.workspace, "greenfield must not write the workspace discriminator");
    }

    // ---- brownfield ----

    #[test]
    fn brownfield() {
        // Concise matrix view of the regular re-entry upgrade. Exhaustive
        // write-set coverage: tests/init.rs::
        // init_upgrade_bumps_only_version_and_preserves_artifacts (Change E).
        let tmp = tempdir().unwrap();
        let specify = tmp.path().join(".specify");
        fs::create_dir_all(specify.join("slices/my-slice")).unwrap();
        fs::write(
            specify.join("project.yaml"),
            "name: brownfield\nadapter: omnia\nspecify: 0.1.0\nrules:\n  specs: specs.md\n",
        )
        .unwrap();
        fs::write(specify.join("slices/my-slice/spec.md"), "# operator slice\n").unwrap();
        fs::write(tmp.path().join("AGENTS.md"), "# operator AGENTS.md\n").unwrap();

        let before = snapshot(tmp.path());
        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "init", "--upgrade"])
            .assert()
            .success();
        let body = parse_json(&assert.get_output().stdout);
        assert_eq!(body["specify-version"], BINARY_VERSION);
        assert_eq!(body["specify-version-changed"], true);
        assert_eq!(body["adapter-name"], "omnia");

        assert_only_project_yaml_changed(&before, &snapshot(tmp.path()));
        let cfg = load_cfg(tmp.path());
        assert_eq!(cfg.specify_version.as_deref(), Some(BINARY_VERSION));
        assert_eq!(cfg.adapter.as_deref(), Some("omnia"));

        assert_second_run_is_noop(tmp.path());
    }

    // ---- workspace ----

    #[test]
    fn workspace() {
        // Concise matrix view of the workspace re-entry upgrade. Exhaustive
        // coverage: tests/init.rs::init_upgrade_preserves_workspace_and_registry
        // (Change E).
        let tmp = tempdir().unwrap();
        let specify = tmp.path().join(".specify");
        fs::create_dir_all(&specify).unwrap();
        fs::write(
            specify.join("project.yaml"),
            "name: platform-workspace\nspecify: 0.1.0\nworkspace: true\n",
        )
        .unwrap();
        fs::write(tmp.path().join("registry.yaml"), "version: 1\nprojects: []\n").unwrap();
        fs::write(tmp.path().join("AGENTS.md"), "# workspace sentinel\n").unwrap();

        let before = snapshot(tmp.path());
        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "init", "--upgrade"])
            .assert()
            .success();
        let body = parse_json(&assert.get_output().stdout);
        assert_eq!(body["specify-version"], BINARY_VERSION);
        assert_eq!(body["specify-version-changed"], true);
        assert_eq!(body["adapter-name"], "workspace");

        assert_only_project_yaml_changed(&before, &snapshot(tmp.path()));
        let cfg = load_cfg(tmp.path());
        assert!(cfg.workspace, "workspace discriminator must survive the upgrade");
        assert!(cfg.adapter.is_none(), "workspace upgrade must not synthesise an adapter");
        assert_eq!(cfg.specify_version.as_deref(), Some(BINARY_VERSION));

        assert_second_run_is_noop(tmp.path());
    }

    // ---- helpers ----

    /// Parse `.specify/project.yaml` under `root` into a [`ProjectConfig`].
    fn load_cfg(root: &Path) -> ProjectConfig {
        let text =
            fs::read_to_string(root.join(".specify/project.yaml")).expect("read project.yaml");
        serde_saphyr::from_str(&text).expect("parse project.yaml")
    }

    /// Snapshot every regular file under `root` as `relative-path -> bytes`.
    fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(dir).expect("read_dir") {
                let entry = entry.expect("dir entry");
                let path = entry.path();
                let rel = path.strip_prefix(root).expect("strip prefix").to_path_buf();
                if entry.file_type().expect("file_type").is_dir() {
                    walk(root, &path, out);
                } else {
                    out.insert(rel, fs::read(&path).expect("read file"));
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    /// Assert the only path that differs between two tree snapshots is
    /// `.specify/project.yaml` — the closed write set of `init --upgrade`.
    fn assert_only_project_yaml_changed(
        before: &BTreeMap<PathBuf, Vec<u8>>, after: &BTreeMap<PathBuf, Vec<u8>>,
    ) {
        let project_yaml = PathBuf::from(".specify/project.yaml");
        let before_keys: Vec<_> = before.keys().filter(|k| **k != project_yaml).collect();
        let after_keys: Vec<_> = after.keys().filter(|k| **k != project_yaml).collect();
        assert_eq!(before_keys, after_keys, "upgrade must not add or remove files");
        for key in before_keys {
            assert_eq!(before[key], after[key], "file {} must be byte-identical", key.display());
        }
    }

    /// Run a second `init --upgrade` over `root` and assert it is a
    /// byte-stable no-op (`specify-version-changed: false`, tree unchanged).
    fn assert_second_run_is_noop(root: &Path) {
        let before = snapshot(root);
        let assert = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "init", "--upgrade"])
            .assert()
            .success();
        assert_eq!(
            parse_json(&assert.get_output().stdout)["specify-version-changed"],
            false,
            "second --upgrade must be a no-op",
        );
        assert_eq!(snapshot(root), before, "second --upgrade must leave the tree byte-identical");
    }
}

mod hydration {
    //! RFC-65 init-trigger wiring for the hydration kernel: `specify
    //! init --upgrade` re-runs hydration over the declared set — the
    //! `project.yaml.adapter` pin plus the `adapters:` prefetch list —
    //! against the global store. Warm-store probes only; the networked
    //! fetch leg stays behind the existing `#[ignore]` registry smoke
    //! tests.

    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use crate::common::{specify_cmd, stage_store_component};

    /// Seed a minimal initialised project whose `project.yaml` declares
    /// the given `adapters:` prefetch entries.
    fn seed_project_with_prefetch(root: &Path, entries: &[&str]) {
        let specify = root.join(".specify");
        fs::create_dir_all(&specify).expect("mkdir .specify");
        let list = entries.iter().fold(String::new(), |mut acc, entry| {
            acc.push_str("- ");
            acc.push_str(entry);
            acc.push('\n');
            acc
        });
        fs::write(
            specify.join("project.yaml"),
            format!("name: demo\nadapter: omnia\nspecify: 0.1.0\nadapters:\n{list}"),
        )
        .expect("write project.yaml");
        // Sentinel: keeps upgrade off the context-generation leg (which
        // would resolve the bare `omnia` dev adapter this seed lacks).
        fs::write(root.join("AGENTS.md"), "# Sentinel AGENTS.md — operator authored\n")
            .expect("write AGENTS.md");
    }

    #[test]
    fn upgrade_hydrates_warm_prefetch_list() {
        // A warm store makes upgrade-time hydration a no-op probe: the
        // staged entry satisfies the prefetch pin without any fetch (a
        // registry pull would fail in this sandbox), and the entry
        // survives byte-identical.
        let tmp = tempdir().unwrap();
        let entry = stage_store_component("demo-target", "1.0.0");
        let bytes_before = fs::read(&entry).expect("read staged entry");
        seed_project_with_prefetch(tmp.path(), &["demo-target@1.0.0"]);

        specify_cmd().current_dir(tmp.path()).args(["init", "--upgrade"]).assert().success();
        assert_eq!(
            fs::read(&entry).expect("re-read staged entry"),
            bytes_before,
            "warm-store hydration must leave the entry untouched"
        );
    }

    #[test]
    fn upgrade_refuses_unpinned_prefetch_entry() {
        // A bare prefetch name is refused with the typed
        // `adapter-prefetch-unpinned` before anything is fetched.
        let tmp = tempdir().unwrap();
        seed_project_with_prefetch(tmp.path(), &["demo-target"]);

        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "init", "--upgrade"])
            .assert()
            .failure();
        let envelope: serde_json::Value = serde_json::from_slice(&assert.get_output().stderr)
            .expect("stderr is the JSON envelope");
        assert_eq!(envelope["error"], "adapter-prefetch-unpinned");
        assert!(
            envelope["message"].as_str().is_some_and(|m| m.contains("demo-target")),
            "error names the offending entry: {envelope}"
        );
    }

    #[test]
    fn upgrade_refuses_drifted_prefetch_entry() {
        // RFC-48 D4 verify-on-read holds on the hydration path: a store
        // entry whose bytes drifted from the recorded sidecar digest
        // aborts the upgrade with `adapter-digest-mismatch`.
        let tmp = tempdir().unwrap();
        let entry = stage_store_component("demo-target", "1.0.0");
        fs::write(&entry, b"\0asm-drifted").expect("drift the entry");
        seed_project_with_prefetch(tmp.path(), &["demo-target@1.0.0"]);

        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "init", "--upgrade"])
            .assert()
            .failure();
        let envelope: serde_json::Value = serde_json::from_slice(&assert.get_output().stderr)
            .expect("stderr is the JSON envelope");
        assert_eq!(envelope["error"], "adapter-digest-mismatch");
        assert!(
            envelope["message"].as_str().is_some_and(|m| m.contains("demo-target@1.0.0")),
            "error names the identity: {envelope}"
        );
    }
}

mod onboarding {
    //! RFC-65 §"Operator onboarding": idempotent re-entry routing to
    //! `--upgrade` and the postflight report (hydrated set, store
    //! root, literal next command). The typed missing-flag error for
    //! the non-TTY substrate is pinned by
    //! `base::init_with_no_args_errors`; the TTY prompt path cannot be
    //! reached through a subprocess (stdin is a pipe) and shares the
    //! same decision logic, so the non-TTY coverage carries it.

    use std::fs;

    use tempfile::tempdir;

    use crate::common::{
        isolated_adapter_store_root, omnia_component, parse_json, specify_cmd,
        stage_store_component,
    };

    #[test]
    fn reentry_exits_zero_and_routes_to_upgrade() {
        // Rerunning the init door over an initialized project is never
        // an error: it changes nothing, exits 0, and prints the
        // literal `specify init --upgrade` re-entry command.
        let tmp = tempdir().unwrap();
        specify_cmd()
            .current_dir(tmp.path())
            .args(["init"])
            .arg(omnia_component())
            .args(["--name", "demo"])
            .assert()
            .success();
        let config_path = tmp.path().join(".specify/project.yaml");
        let before = fs::read(&config_path).expect("read project.yaml");

        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "init"])
            .arg(omnia_component())
            .assert()
            .success();
        let body = parse_json(&assert.get_output().stdout);
        assert_eq!(body["already-initialized"], true);
        assert_eq!(body["name"], "demo");
        assert_eq!(body["next"], "specify init --upgrade");
        assert_eq!(
            fs::read(&config_path).expect("re-read project.yaml"),
            before,
            "re-entry must leave project.yaml byte-identical"
        );
    }

    #[test]
    fn reentry_wins_over_missing_adapter() {
        // Re-entry detection runs ahead of the elicitation layer:
        // `specify init` with no arguments inside an initialized
        // project routes to `--upgrade` (exit 0) instead of raising
        // the typed missing-adapter error.
        let tmp = tempdir().unwrap();
        specify_cmd()
            .current_dir(tmp.path())
            .args(["init"])
            .arg(omnia_component())
            .args(["--name", "demo"])
            .assert()
            .success();

        let assert = specify_cmd().current_dir(tmp.path()).args(["init"]).assert().success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
        assert!(
            stdout.contains("Already initialized"),
            "text report names the re-entry state: {stdout}"
        );
        assert!(
            stdout.contains("specify init --upgrade"),
            "text report carries the literal re-entry command: {stdout}"
        );
    }

    #[test]
    fn postflight_reports_hydration_and_store() {
        // A pinned init against a warm store reports the hydrated
        // identity (`<name>@<version>`), the store root, and the
        // literal next command; a fully-flagged run teaches no
        // `equivalent` invocation.
        let tmp = tempdir().unwrap();
        stage_store_component("demo-target", "1.0.0");

        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "init", "demo-target@1.0.0", "--name", "demo"])
            .assert()
            .success();
        let body = parse_json(&assert.get_output().stdout);
        let hydrated: Vec<&str> = body["hydrated"]
            .as_array()
            .expect("hydrated array")
            .iter()
            .map(|v| v.as_str().expect("identity string"))
            .collect();
        assert_eq!(hydrated, vec!["demo-target@1.0.0"], "the warm-store pin resolves");
        assert_eq!(
            body["adapter-store"],
            isolated_adapter_store_root().display().to_string(),
            "the report names the resolved store root"
        );
        assert_eq!(body["next"], "/spec:plan <name>");
        assert!(
            body.get("equivalent").is_none(),
            "no prompt fired, so no equivalent invocation is taught: {body}"
        );
    }

    #[test]
    fn postflight_text_names_store_and_next() {
        // The text renderer carries the same postflight facts: the
        // (empty, for a local component) hydrated set, the store root,
        // and the literal next command.
        let tmp = tempdir().unwrap();
        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["init"])
            .arg(omnia_component())
            .args(["--name", "demo"])
            .assert()
            .success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
        assert!(
            stdout.contains("hydrated: nothing (components resolved locally)"),
            "a local-component init hydrates nothing: {stdout}"
        );
        assert!(
            stdout.contains(&format!("adapter store: {}", isolated_adapter_store_root().display())),
            "the report names the store root: {stdout}"
        );
        assert!(stdout.contains("Next: run `/spec:plan <name>`"), "literal next command: {stdout}");
    }
}
