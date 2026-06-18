//! Integration tests for the `specify source` subcommand tree
//! (`resolve`, `extract`, `preview`, `survey`). Shared helpers live in
//! [`common`].

mod extract {
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
        let src = repo_root().join(
            "crates/workflow/tests/fixtures/plugins/adapters/sources/typescript/adapter.yaml",
        );
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
        let raw =
            fs::read_to_string(&path).unwrap_or_else(|err| panic!("read journal.jsonl: {err}"));
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
        let adapter_src = repo_root().join(
            "crates/workflow/tests/fixtures/plugins/adapters/sources/typescript/adapter.yaml",
        );
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
                || !journal_events(&project)
                    .iter()
                    .any(|e| e["event"] == "slice.extract.completed"),
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
                || !journal_events(&project)
                    .iter()
                    .any(|e| e["event"] == "slice.extract.completed"),
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
}

mod preview {
    //! Integration tests for `specify source preview` (`specify source preview` contract).

    use std::fs;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use crate::common::{copy_dir, parse_stderr, parse_stdout, repo_root, specify_cmd};

    fn plugin_fixtures_root() -> PathBuf {
        repo_root().join("crates/workflow/tests/fixtures/plugins")
    }

    fn stage_source_adapter(root: &std::path::Path, name: &str) {
        let src = plugin_fixtures_root().join("adapters").join("sources").join(name);
        let dst = root.join("adapters").join("sources").join(name);
        copy_dir(&src, &dst);
    }

    #[test]
    fn preview_succeeds_without_specify_dir() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        stage_source_adapter(root, "typescript");

        let source_dir = root.join("my-source");
        fs::create_dir_all(&source_dir).expect("create source dir");

        let assert = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "source", "preview", "typescript"])
            .arg("--source")
            .arg(&source_dir)
            .arg("--project-dir")
            .arg(root)
            .assert()
            .success();

        let actual = parse_stdout(&assert.get_output().stdout, root);
        assert_eq!(actual["adapter"], "typescript");
        assert_eq!(actual["version"], "1.0.0");

        let briefs = actual["briefs"].as_array().expect("briefs array");
        assert_eq!(briefs.len(), 2);
        let ops: Vec<&str> = briefs.iter().map(|b| b["operation"].as_str().unwrap()).collect();
        assert!(ops.contains(&"survey"));
        assert!(ops.contains(&"extract"));
    }

    #[test]
    fn preview_creates_output_directory() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        stage_source_adapter(root, "typescript");

        let source_dir = root.join("my-source");
        fs::create_dir_all(&source_dir).expect("create source dir");

        let out_dir = root.join("custom-out");

        specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "source", "preview", "typescript"])
            .arg("--source")
            .arg(&source_dir)
            .arg("--out")
            .arg(&out_dir)
            .arg("--project-dir")
            .arg(root)
            .assert()
            .success();

        assert!(out_dir.join("evidence").is_dir(), "evidence/ subdirectory must be created");
        assert!(!root.join(".specify").exists(), "no .specify/ residue");
    }

    #[test]
    fn default_out_creates_preview() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        stage_source_adapter(root, "typescript");

        let source_dir = root.join("my-source");
        fs::create_dir_all(&source_dir).expect("create source dir");

        specify_cmd()
            .current_dir(root)
            .args(["source", "preview", "typescript"])
            .arg("--source")
            .arg(&source_dir)
            .arg("--project-dir")
            .arg(root)
            .assert()
            .success();

        assert!(
            root.join(".specify-preview/evidence").is_dir(),
            "default .specify-preview/evidence/ must be created"
        );
    }

    #[test]
    fn preview_passes_leads_through() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        stage_source_adapter(root, "typescript");

        let source_dir = root.join("my-source");
        fs::create_dir_all(&source_dir).expect("create source dir");

        let assert = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "source", "preview", "typescript"])
            .arg("--source")
            .arg(&source_dir)
            .args(["--lead", "login-screen", "--lead", "settings"])
            .arg("--project-dir")
            .arg(root)
            .assert()
            .success();

        let actual = parse_stdout(&assert.get_output().stdout, root);
        let leads = actual["leads"].as_array().expect("leads array");
        assert_eq!(leads.len(), 2);
        assert_eq!(leads[0], "login-screen");
        assert_eq!(leads[1], "settings");
    }

    #[test]
    fn preview_fails_when_source_path_missing() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        stage_source_adapter(root, "typescript");

        let assert = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "source", "preview", "typescript"])
            .arg("--source")
            .arg(root.join("nonexistent"))
            .arg("--project-dir")
            .arg(root)
            .assert()
            .failure();

        let stderr = parse_stderr(&assert.get_output().stderr, root);
        assert_eq!(stderr["error"], "argument");
        assert_eq!(stderr["exit-code"], 2);
    }

    #[test]
    fn preview_fails_when_adapter_not_found() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();

        let source_dir = root.join("my-source");
        fs::create_dir_all(&source_dir).expect("create source dir");

        let assert = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "source", "preview", "no-such-adapter"])
            .arg("--source")
            .arg(&source_dir)
            .arg("--project-dir")
            .arg(root)
            .assert()
            .failure();

        let stderr = parse_stderr(&assert.get_output().stderr, root);
        assert_eq!(stderr["error"], "adapter-not-found");
        assert_eq!(stderr["exit-code"], 1);
    }
}

mod resolve {
    //! Integration tests for `specify source resolve`.
    //!
    //! Mirrors the source-adapter loader exposed by
    //! `crates/workflow/src/plugin/`. The CLI verb is a thin
    //! `Plugin::resolve(Axis::Source, …)` wrapper; the cases below pin
    //! the wire shape skill bodies and downstream callers rely on.

    use std::path::PathBuf;

    use crate::common::{Project, copy_dir, parse_stderr, parse_stdout, repo_root, specify_cmd};

    fn plugin_fixtures_root() -> PathBuf {
        repo_root().join("crates/workflow/tests/fixtures/plugins")
    }

    fn stage_source_fixture(project: &Project, name: &str) {
        let src = plugin_fixtures_root().join("adapters").join("sources").join(name);
        let dst = project.root().join("adapters").join("sources").join(name);
        copy_dir(&src, &dst);
    }

    #[test]
    fn resolve_local_returns_manifest() {
        let project = Project::init();
        stage_source_fixture(&project, "typescript");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "source", "resolve", "typescript"])
            .arg("--project-dir")
            .arg(project.root())
            .assert()
            .success();

        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(actual["axis"], "sources");
        assert_eq!(actual["name"], "typescript");
        assert_eq!(actual["location"], "local");
        let operations = actual["operations"].as_array().expect("operations array");
        let ops: Vec<&str> = operations.iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(ops, vec!["extract", "survey"]);
        let resolved = actual["resolved-path"].as_str().expect("resolved-path str");
        assert!(
            resolved.ends_with("adapters/sources/typescript"),
            "resolved-path {resolved} must end with sources/typescript"
        );
        let briefs_dir = actual["briefs-dir"].as_str().expect("briefs-dir str");
        assert_eq!(
            briefs_dir,
            format!("{resolved}/briefs"),
            "briefs-dir must be the resolved adapter root joined with briefs/"
        );
    }

    #[test]
    fn resolve_missing_emits_not_found() {
        let project = Project::init();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "source", "resolve", "no-such-source"])
            .arg("--project-dir")
            .arg(project.root())
            .assert()
            .failure();
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "adapter-not-found");
        assert_eq!(stderr["exit-code"], 1);
    }
}

mod survey {
    //! Integration tests for `specify source survey`
    //! (DECISIONS.md §"Source operations").
    //!
    //! Covers source resolution against `plan.yaml.sources`, the agent
    //! two-phase dispatch (prepare prints the handoff envelope + emits
    //! `source.execution.agent`; finalize validates-before-visible and
    //! emits `source.survey.completed`), and the validate-before-visible
    //! guarantee that an invalid lead set leaves `discovery.md` untouched.

    use std::fs;
    use std::path::PathBuf;

    use crate::common::{
        Project, parse_stderr, parse_stdout, read_journal_normalized, repo_root, specify_cmd,
    };

    fn stage_typescript(project: &Project) {
        // The in-repo fixture ships only `adapter.yaml` (execution: agent);
        // stage it, then author the `survey` brief the agent reads.
        let src = repo_root().join(
            "crates/workflow/tests/fixtures/plugins/adapters/sources/typescript/adapter.yaml",
        );
        let adapter_dir = project.root().join("adapters/sources/typescript");
        fs::create_dir_all(adapter_dir.join("briefs")).expect("create adapter briefs dir");
        fs::copy(&src, adapter_dir.join("adapter.yaml")).expect("copy adapter.yaml");
        fs::write(adapter_dir.join("briefs/survey.md"), "# survey brief\n")
            .expect("write survey brief");
    }

    fn seed_plan_with_legacy_source(project: &Project) {
        project.seed_plan(
            "name: platform-v2
sources:
  legacy:
    adapter: typescript
    path: vendor/legacy
slices:
  - name: a
    project: default
    status: pending
",
        );
    }

    fn survey_scratch_dir(project: &Project) -> PathBuf {
        project.root().join(".specify/scratch/typescript/survey")
    }

    // A `survey` lead-set omits `source`: attribution is CLI-owned,
    // so the runner stamps `legacy` onto every lead before the schema
    // check and the merge.
    const VALID_LEAD_SET: &str = "\
### user-registration

- lead: user-registration
- synopsis: Registration endpoint accepting email + password.
";

    #[test]
    fn prepare_prints_envelope_emits_event() {
        let project = Project::init();
        stage_typescript(&project);
        seed_plan_with_legacy_source(&project);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "source", "survey", "legacy"])
            .assert()
            .success();

        let body = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(body["adapter"], "typescript");
        assert_eq!(body["version"], "1.0.0");
        assert_eq!(body["execution"], "agent");
        assert!(
            body.get("evidence-dir").is_none(),
            "survey handoff must NOT carry evidence-dir, got:\n{body}"
        );
        let scratch = body["scratch-dir"].as_str().expect("scratch-dir str");
        assert!(
            scratch.ends_with(".specify/scratch/typescript/survey"),
            "scratch-dir {scratch} must key under the survey segment"
        );
        let briefs = body["briefs-dir"].as_str().expect("briefs-dir str");
        assert!(briefs.ends_with("adapters/sources/typescript/briefs"), "briefs-dir: {briefs}");
        let source_dir = body["source-dir"].as_str().expect("source-dir str");
        assert!(source_dir.ends_with("vendor/legacy"), "source-dir: {source_dir}");
        assert_eq!(
            body["leads"].as_array().expect("leads array").len(),
            0,
            "fresh survey has no leads"
        );

        // prepare builds the scratch dir up front.
        assert!(survey_scratch_dir(&project).is_dir(), "prepare must create the scratch dir");

        let events = read_journal_normalized(project.root());
        assert_eq!(events.len(), 1, "prepare emits exactly one event");
        assert_eq!(events[0]["event"], "source.execution.agent");
        assert_eq!(events[0]["payload"]["source"], "legacy");
        assert_eq!(events[0]["payload"]["adapter"], "typescript");
        assert_eq!(events[0]["payload"]["operation"], "survey");
    }

    #[test]
    fn prepare_clears_stale_scratch() {
        let project = Project::init();
        stage_typescript(&project);
        seed_plan_with_legacy_source(&project);

        // A prior run left a stale lead set in the scratch lane.
        let scratch = survey_scratch_dir(&project);
        fs::create_dir_all(&scratch).expect("create scratch dir");
        fs::write(scratch.join("leads.md"), "### stale-lead\n").expect("write stale leads.md");

        specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "source", "survey", "legacy"])
            .assert()
            .success();

        assert!(scratch.is_dir(), "prepare must recreate the scratch dir");
        assert!(
            !scratch.join("leads.md").exists(),
            "prepare must drop stale artifacts so finalize only sees this run's output"
        );
    }

    #[test]
    fn finalize_merges_and_completes() {
        let project = Project::init();
        stage_typescript(&project);
        seed_plan_with_legacy_source(&project);

        // Stand in for the agent: write the produced lead set into scratch.
        let scratch = survey_scratch_dir(&project);
        fs::create_dir_all(&scratch).expect("create scratch dir");
        fs::write(scratch.join("leads.md"), VALID_LEAD_SET).expect("write leads.md");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "source", "survey", "legacy", "--phase", "finalize"])
            .assert()
            .success();

        let body = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(body["adapter"], "typescript");
        assert_eq!(body["source"], "legacy");
        let leads: Vec<&str> =
            body["leads"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(leads, vec!["user-registration"]);

        // The lead is now visible in discovery.md.
        let discovery =
            fs::read_to_string(project.root().join("discovery.md")).expect("discovery.md");
        assert!(
            discovery.contains("### legacy:user-registration"),
            "merged lead must appear:\n{discovery}"
        );
        assert!(discovery.contains("- source: legacy"), "merged lead records its source");

        let events = read_journal_normalized(project.root());
        let completed = events
            .iter()
            .find(|e| e["event"] == "source.survey.completed")
            .expect("a survey-completed event");
        assert_eq!(completed["payload"]["source"], "legacy");
        assert_eq!(completed["payload"]["adapter"], "typescript");
    }

    #[test]
    fn finalize_unparseable_lead_set_errors() {
        let project = Project::init();
        stage_typescript(&project);
        seed_plan_with_legacy_source(&project);

        let scratch = survey_scratch_dir(&project);
        fs::create_dir_all(&scratch).expect("create scratch dir");
        fs::write(scratch.join("leads.md"), "The survey found registration behavior.\n")
            .expect("write unparseable leads.md");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "source", "survey", "legacy", "--phase", "finalize"])
            .assert()
            .failure();

        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "survey-leads-empty");
        assert_eq!(stderr["exit-code"], 1);
        assert!(
            !project.root().join("discovery.md").exists(),
            "an unparseable lead set must leave discovery.md untouched"
        );
        assert!(
            !project.root().join(".specify/journal.jsonl").exists()
                || !read_journal_normalized(project.root())
                    .iter()
                    .any(|e| e["event"] == "source.survey.completed"),
            "unparseable lead set must not emit a completion event"
        );
    }

    #[test]
    fn finalize_invalid_lead_set_untouched() {
        let project = Project::init();
        stage_typescript(&project);
        seed_plan_with_legacy_source(&project);

        // `bad_id` parses as a lead block but fails the kebab-case schema.
        let scratch = survey_scratch_dir(&project);
        fs::create_dir_all(&scratch).expect("create scratch dir");
        fs::write(
            scratch.join("leads.md"),
            "## Lead inventory\n\n### bad_id\n\n- lead: bad_id\n- synopsis: Bad id.\n",
        )
        .expect("write invalid leads.md");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "source", "survey", "legacy", "--phase", "finalize"])
            .assert()
            .failure();

        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "discovery-lead-schema");
        assert_eq!(stderr["exit-code"], 2);

        // Validate-before-visible: nothing was written.
        assert!(
            !project.root().join("discovery.md").exists(),
            "an invalid lead set must leave discovery.md untouched"
        );
        // No completion event fires for an invalid lead set.
        assert!(
            !project.root().join(".specify/journal.jsonl").exists()
                || !read_journal_normalized(project.root())
                    .iter()
                    .any(|e| e["event"] == "source.survey.completed"),
            "invalid lead set must not emit a completion event"
        );
    }

    #[test]
    fn unknown_source_errors() {
        let project = Project::init();
        stage_typescript(&project);
        seed_plan_with_legacy_source(&project);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "source", "survey", "not-a-source"])
            .assert()
            .failure();

        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "source-unknown");
        assert_eq!(stderr["exit-code"], 1);
    }

    #[test]
    fn plan_name_mismatch_errors() {
        let project = Project::init();
        stage_typescript(&project);
        seed_plan_with_legacy_source(&project);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "source", "survey", "legacy", "--plan", "wrong-plan"])
            .assert()
            .failure();

        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["exit-code"], 2, "a --plan mismatch is an argument error");
    }
}
