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
    //! `transition`, `create`, `archive`, `propose`, `authority`). Every
    //! submodule pulls its shared surface in with `use super::support::*;`,
    //! so the common imports, helpers, and plan seeds live here once.

    pub use std::fs;
    pub use std::path::{Path, PathBuf};
    pub use std::process::Command as ProcessCommand;

    pub use serde_json::Value;
    pub use specify_workflow::change::{Plan, Status};
    pub use tempfile::{TempDir, tempdir};

    pub use crate::common::{
        Project, assert_golden_at, copy_dir, expected_cache_dir, init_workspace, omnia_schema_dir,
        parse_stderr, parse_stdout, repo_root, specify_cmd,
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

mod end_to_end {
    //! Plan-driven loop acceptance proof.
    //!
    //! This is the end-to-end fixture that proves fan-in *twice* (Lead sets
    //! at `survey`, Evidence at `extract`) and fan-out *once* (multiple
    //! single-target slices reconciled from shared source claims), then
    //! drives those slices all the way through `build` and `merge` under a
    //! `depends-on` ordering and asserts the kernel-projection determinism
    //! property.
    //!
    //! ```text
    //! documentation + typescript (sources: docs, legacy)
    //!   -> source survey            # fan-in #1: Lead sets (incl. docs:password-reset / legacy:reset-password mismatch)
    //!   -> plan propose --dry-run   # flat lead catalog + identity-contracts->contracts@1.0.0 / identity-service->omnia@1.0.0
    //!   -> plan propose --from      # agent groups leads; kernel writes single-target slices + project bindings + depends-on
    //!   -> per slice: source extract -> slice synthesize -> slice build -> slice merge
    //!   -> depends-on ordering: identity-contracts merges before identity-service starts
    //! ```
    //!
    //! ## Topology choice (documented simplification)
    //!
    //! An alternative *same-tree registry-symlink* topology exists, where
    //! two registry projects resolve into one working tree via `registry.yaml`
    //! URLs materialised as symlinks. This
    //! test uses the **workspace + committed `topology.lock`** projection that the
    //! shipped `plan propose` tests already exercise (see
    //! `tests/workflow/propose.rs::propose_*`) — it exposes the same two
    //! projects to `propose` without the symlink-materialisation machinery,
    //! which the deterministic proof does not need. Both slices live in one
    //! `.specify/slices/` tree and merge into one baseline (`.specify/specs/`),
    //! so "two single-target slices sharing one baseline tree, ordered by
    //! depends-on" is proven directly. Each slice's bound target is set via
    //! `slice create --target <t>` (the CLI surface that stores the bound
    //! adapter on `metadata.yaml`); `slice build` resolves it from there,
    //! exactly as in production.
    //!
    //! ## Coverage delegated to existing tests (not re-implemented here)
    //!
    //! The exhaustive malformed-`--from` reconcile codes
    //! (`lead-coverage-orphan`, `plan-reconcile-lead-orphan`,
    //! `-slice-source-collision`, `-slice-name-collision`,
    //! `-depends-on-cycle`, `-project-binding-required`, `-project-orphan`,
    //! `-plan-not-replaceable`) are covered over this exact identity fan-out
    //! shape in `tests/workflow/`. The synthesis-kernel
    //! normalize-not-reject and per-source orphan/kind-mismatch aborts are
    //! covered in `tests/slice.rs::synthesize_normalizes_pre_assigned_fields`.
    //! This test asserts the *composed* path and the fan-out-specific guards
    //! (`plan-propose-mode-required` plus a `project-binding-required`
    //! spot-check on this workspace), then the build / merge / ordering /
    //! determinism behaviour no existing test covers.
    //!
    //! `change.md` rendering of cross-source matches is **agent-owned**: the
    //! response `rationale` field is kernel-ignored (see
    //! `change/plan/core/propose.rs`), so `change.md` is authored by the
    //! `/spec:plan` skill, not the deterministic CLI this test drives. We
    //! therefore assert the kernel-side effects of the fan-out (slice
    //! bindings, depends-on, the `plan.reconcile.completed` event) rather
    //! than the skill-authored `change.md`.

    use std::fs;
    use std::path::{Path, PathBuf};

    use serde_json::Value;
    use tempfile::{TempDir, tempdir};

    use crate::common::{
        Project, copy_dir, hold_plan_lock, init_workspace, omnia_schema_dir, parse_json,
        parse_stderr, parse_stdout, repo_root, specify_cmd,
    };

    // ---------------------------------------------------------------------------
    // Fixture + seed material
    // ---------------------------------------------------------------------------

    fn fixture_dir() -> PathBuf {
        repo_root().join("tests/fixtures/fan-in-fan-out")
    }

    fn fixture(rel: &str) -> String {
        let path = fixture_dir().join(rel);
        fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read fixture {}: {err}", path.display()))
    }

    /// Hub registry: the two projects bound to different target adapters
    /// that the fan-out response binds against.
    const REGISTRY_HUB: &str = "\
version: 1
projects:
  - name: identity-contracts
    url: git@github.com:org/identity-contracts.git
    adapter: contracts@1.0.0
    description: Versioned API contracts crate for the identity domain.
  - name: identity-service
    url: git@github.com:org/identity-service.git
    adapter: omnia@1.0.0
    description: Omnia identity service implementing auth and password flows.
";

    /// Committed plan-time topology projection the workspace reads in
    /// place of materialising the remote members.
    const TOPOLOGY_HUB: &str = "\
version: 1
projects:
  - name: identity-contracts
    target: contracts@1.0.0
    description: Versioned API contracts crate for the identity domain.
  - name: identity-service
    target: omnia@1.0.0
    description: Omnia identity service implementing auth and password flows.
";

    /// Hub plan declaring the two surveyed sources, no slices yet.
    const PLAN_HUB: &str = "\
name: identity-revamp
sources:
  docs:
    adapter: documentation
    path: ./docs
  legacy:
    adapter: typescript
    path: ./legacy
slices: []
";

    const CONTRACTS_ADAPTER: &str = "\
name: contracts
version: 1.0.0
axis: target
execution: agent
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
inputs:
  - path: contracts
    required: true
description: Versioned API contracts target.
";

    // ---------------------------------------------------------------------------
    // Staging helpers
    // ---------------------------------------------------------------------------

    /// Author a minimal `execution: agent` source adapter with the two
    /// briefs the survey/extract agents read.
    fn stage_source_adapter(root: &Path, name: &str, description: &str) {
        let dir = root.join(format!("adapters/sources/{name}"));
        fs::create_dir_all(dir.join("briefs")).expect("mkdir source adapter briefs");
        fs::write(
            dir.join("adapter.yaml"),
            format!(
                "name: {name}\nversion: 1.0.0\naxis: source\nexecution: agent\nbriefs:\n  survey: \
             briefs/survey.md\n  extract: briefs/extract.md\ndescription: {description}\n"
            ),
        )
        .expect("write source adapter.yaml");
        fs::write(dir.join("briefs/survey.md"), "# survey brief\n").expect("write survey brief");
        fs::write(dir.join("briefs/extract.md"), "# extract brief\n").expect("write extract brief");
    }

    /// Author the `contracts` target adapter (declares a required
    /// `contracts` build input) and copy the in-repo `omnia` target.
    fn stage_target_adapters(root: &Path) {
        copy_dir(&omnia_schema_dir(), &root.join("adapters/targets/omnia"));
        let dir = root.join("adapters/targets/contracts");
        fs::create_dir_all(dir.join("briefs")).expect("mkdir contracts adapter briefs");
        fs::write(dir.join("adapter.yaml"), CONTRACTS_ADAPTER)
            .expect("write contracts adapter.yaml");
        for op in ["shape", "build", "merge"] {
            fs::write(dir.join(format!("briefs/{op}.md")), format!("# {op} brief\n"))
                .expect("write contracts brief");
        }
    }

    /// Stand in for the survey agent: drop the golden lead-set into scratch
    /// and run `source survey <source> --phase finalize`.
    fn survey_finalize(root: &Path, source: &str, adapter: &str, lead_set: &str) {
        let scratch = root.join(format!(".specify/scratch/{adapter}/survey"));
        fs::create_dir_all(&scratch).expect("mkdir survey scratch");
        fs::write(scratch.join("leads.md"), lead_set).expect("write leads.md");
        specify_cmd()
            .current_dir(root)
            .args(["source", "survey", source, "--phase", "finalize"])
            .assert()
            .success();
    }

    /// Stand in for the extract agent: drop the golden Evidence into scratch
    /// and run `source extract <source> <lead> --slice <slice> --phase finalize`.
    fn extract_finalize(
        root: &Path, source: &str, adapter: &str, lead: &str, slice: &str, evidence: &str,
    ) {
        let scratch = root.join(format!(".specify/scratch/{adapter}/{slice}"));
        fs::create_dir_all(&scratch).expect("mkdir extract scratch");
        fs::write(scratch.join("evidence.yaml"), evidence).expect("write evidence.yaml");
        specify_cmd()
            .current_dir(root)
            .args(["source", "extract", source, lead, "--slice", slice, "--phase", "finalize"])
            .assert()
            .success();
    }

    fn journal_lines(root: &Path) -> Vec<String> {
        let path = root.join(".specify/journal.jsonl");
        fs::read_to_string(&path)
            .map(|raw| raw.lines().filter(|l| !l.is_empty()).map(str::to_string).collect())
            .unwrap_or_default()
    }

    fn journal_has(root: &Path, event: &str) -> bool {
        journal_lines(root).iter().any(|l| l.contains(&format!(r#""event":"{event}""#)))
    }

    fn read_plan(root: &Path) -> String {
        fs::read_to_string(root.join("plan.yaml")).expect("read plan.yaml")
    }

    // ---------------------------------------------------------------------------
    // Scenario setup
    // ---------------------------------------------------------------------------

    /// Stand up the workspace, stage adapters + sources, and run both
    /// surveys so `discovery.md` carries all four leads.
    fn scenario() -> TempDir {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        init_workspace(&tmp, "identity-revamp");
        fs::write(root.join("registry.yaml"), REGISTRY_HUB).expect("write registry.yaml");
        fs::write(root.join(".specify/topology.lock"), TOPOLOGY_HUB).expect("write topology.lock");
        fs::write(root.join("plan.yaml"), PLAN_HUB).expect("write plan.yaml");

        stage_source_adapter(root, "documentation", "Operator-provided written intent.");
        stage_source_adapter(
            root,
            "typescript",
            "Behavioural evidence from a TypeScript codebase.",
        );
        stage_target_adapters(root);

        // Both bound source paths exist on disk for the survey/extract runs.
        for src in ["docs", "legacy"] {
            fs::create_dir_all(root.join(src)).expect("mkdir bound source dir");
            fs::write(root.join(src).join(".keep"), "").expect("seed bound source dir");
        }

        // Fan-in #1: survey both sources into one discovery.md.
        survey_finalize(root, "docs", "documentation", &fixture("leads/docs.md"));
        survey_finalize(root, "legacy", "typescript", &fixture("leads/legacy.md"));
        tmp
    }

    // ---------------------------------------------------------------------------
    // The acceptance proof
    // ---------------------------------------------------------------------------

    /// The plan-time half of the proof: survey leads (with the deliberate
    /// slug mismatch), the `--dry-run` request envelope, the
    /// `plan-propose-mode-required` / `project-binding-required` guards, and
    /// the `--from` fan-out that writes single-target slices with project
    /// bindings + depends-on and emits `plan.reconcile.completed`.
    fn prove_plan_time_fan_out(root: &Path) {
        // Survey produced schema-valid leads, including the deliberate
        // docs:password-reset / legacy:reset-password slug mismatch.
        let discovery = fs::read_to_string(root.join("discovery.md")).expect("read discovery.md");
        for block in [
            "### docs:identity-api",
            "### docs:password-reset",
            "### legacy:identity-api",
            "### legacy:reset-password",
        ] {
            assert!(discovery.contains(block), "discovery.md missing {block}, got:\n{discovery}");
        }

        // `propose --dry-run` returns a kind:request envelope exposing both
        // projects and one lead row per (source, lead), and writes nothing.
        let plan_before = read_plan(root);
        let dry = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "plan", "propose", "--dry-run"])
            .assert()
            .success();
        let request = parse_stdout(&dry.get_output().stdout, root);
        assert_eq!(request["kind"], "request");
        let projects = request["projects"].as_array().expect("projects array");
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0]["name"], "identity-contracts");
        assert_eq!(projects[0]["target"], "contracts@1.0.0");
        assert_eq!(projects[1]["name"], "identity-service");
        assert_eq!(projects[1]["target"], "omnia@1.0.0");
        let leads = request["leads"].as_array().expect("leads array");
        assert_eq!(leads.len(), 4, "one row per (source, lead): {leads:#?}");
        assert_eq!(read_plan(root), plan_before, "--dry-run must not touch plan.yaml");

        // Neither flag aborts mode-required.
        let no_mode = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "plan", "propose"])
            .assert()
            .failure();
        assert_eq!(no_mode.get_output().status.code(), Some(2));
        assert_eq!(
            parse_stderr(&no_mode.get_output().stderr, root)["error"],
            "plan-propose-mode-required"
        );

        // Fan-out-specific reconcile guard: with two projects offered, a slice
        // that covers its leads cleanly but omits `project` aborts
        // project-binding-required. (The full partition is satisfied so this
        // is not a partition/collision abort.) The remaining reconcile codes
        // are covered exhaustively over this shape in tests/workflow/.
        fs::write(
        root.join("bad-response.json"),
        r#"{"version":1,"kind":"response","slices":[{"name":"unbound","sources":[{"source":"docs","lead":"identity-api"},{"source":"legacy","lead":"identity-api"}]},{"name":"reset","project":"identity-service","sources":[{"source":"docs","lead":"password-reset"},{"source":"legacy","lead":"reset-password"}]}]}"#,
    )
    .expect("write bad response");
        let bound = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "plan", "propose", "--from", "bad-response.json"])
            .assert()
            .failure();
        assert_eq!(
            parse_stderr(&bound.get_output().stderr, root)["error"],
            "plan-reconcile-project-binding-required"
        );

        // `propose --from` writes single-target slices with project bindings +
        // depends-on and emits plan.reconcile.completed.
        fs::write(root.join("response.json"), fixture("propose-response.json"))
            .expect("write response.json");
        let from = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "plan", "propose", "--from", "response.json"])
            .assert()
            .success();
        let summary = parse_stdout(&from.get_output().stdout, root);
        assert_eq!(summary["slice-count"], 3);
        assert_eq!(
            summary["slice-names"],
            serde_json::json!(["identity-contracts", "identity-service", "password-reset"])
        );
        assert!(
            journal_has(root, "plan.reconcile.completed"),
            "fan-out must emit reconcile.completed"
        );

        let plan = read_plan(root);
        assert!(plan.contains("project: identity-contracts"), "contracts slice binds its project");
        assert!(plan.contains("project: identity-service"), "service slice binds its project");
        assert!(plan.contains("depends-on:"), "service depends-on contracts");
        // The cross-source slug mismatch is matched into the third slice.
        assert!(plan.contains("name: password-reset"), "password-reset slice present");
        assert!(
            plan.contains("lead: reset-password"),
            "legacy reset-password lead carried verbatim"
        );
    }

    #[test]
    fn fan_in_twice_fan_out_once() {
        let tmp = scenario();
        let root = tmp.path();

        prove_plan_time_fan_out(root);

        // From here on this test *is* the driver session: hold the plan
        // lock the way the /spec:execute snippet does, so the gated verbs
        // (`plan next`, `slice merge run`) accept the writes.
        let _lock = hold_plan_lock(root);

        // --- depends-on ordering, gate 1: the driver must pick
        // identity-contracts first — never identity-service while its upstream
        // is unmerged. -------------------------------------------------------
        assert_eq!(plan_next(root)["next"], "identity-contracts");
        // A second poll while contracts is in-progress returns the active
        // entry, never advancing to the dependent.
        let active = plan_next(root);
        assert_eq!(active["reason"], "in-progress");
        assert_eq!(active["active"], "identity-contracts");

        // --- Slice-time: drive identity-contracts (bound target: contracts). -
        drive_slice_to_built(root, "identity-contracts", "contracts", Sources::DocsOnly);

        // The contracts build request carries the adapter-declared `contracts`
        // input in `additional[]`; the bound target is `contracts`.
        let contracts_request =
            fs::read_to_string(root.join(".specify/slices/identity-contracts/build/request.yaml"))
                .expect("read contracts build request");
        assert!(
            contracts_request.contains("additional:") && contracts_request.contains("- contracts"),
            "contracts request resolves the declared `contracts` input into additional[], got:\n{contracts_request}"
        );

        specify_cmd()
            .current_dir(root)
            .args(["slice", "merge", "run", "identity-contracts"])
            .assert()
            .success();
        assert!(read_plan(root).contains("status: done"), "merge stamps the contracts entry done");
        // Upstream output is now visible in the shared baseline tree — the
        // in-tree dependency identity-service reads (no cross-slice channel).
        assert!(
            root.join(".specify/specs/identity/spec.md").is_file(),
            "contracts merge writes the shared baseline before the dependent starts"
        );

        // --- depends-on ordering, gate 2: only now does the driver advance to
        // identity-service. --------------------------------------------------
        assert_eq!(plan_next(root)["next"], "identity-service");

        // --- Slice-time: drive identity-service (bound target: omnia). ------
        drive_slice_to_built(root, "identity-service", "omnia", Sources::DocsAndLegacy);

        // The omnia build request declares no extra inputs, so additional[] is
        // absent (skip_serializing_if empty); the bound target is `omnia`.
        let service_request =
            fs::read_to_string(root.join(".specify/slices/identity-service/build/request.yaml"))
                .expect("read service build request");
        assert!(
            !service_request.contains("additional"),
            "omnia declares no extra inputs, so additional[] is omitted, got:\n{service_request}"
        );

        specify_cmd()
            .current_dir(root)
            .args(["slice", "merge", "run", "identity-service"])
            .assert()
            .success();

        // --- Final plan state: both driven slices done; the cross-source
        // password-reset slice remains pending (proven at plan time only). ---
        let final_plan = read_plan(root);
        let done = final_plan.matches("status: done").count();
        assert_eq!(
            done, 2,
            "both identity-contracts and identity-service reach done, got:\n{final_plan}"
        );
        assert!(
            final_plan.contains("status: pending"),
            "password-reset stays pending, got:\n{final_plan}"
        );
    }

    /// Which `(source, lead)` pairs a slice extracts Evidence for.
    #[derive(Clone, Copy)]
    enum Sources {
        DocsOnly,
        DocsAndLegacy,
    }

    /// Run `plan next --format json` and return the parsed body.
    fn plan_next(root: &Path) -> Value {
        let out = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "plan", "next"])
            .assert()
            .success();
        parse_json(&out.get_output().stdout)
    }

    /// Create a slice bound to `target`, extract its Evidence, synthesize it,
    /// assert the slice-time invariants, then build it to `built`.
    fn drive_slice_to_built(root: &Path, slice: &str, target: &str, sources: Sources) {
        specify_cmd()
            .current_dir(root)
            .args(["slice", "create", slice, "--target", target])
            .assert()
            .success();

        // Fan-in #2: Evidence per (slice, source).
        extract_finalize(
            root,
            "docs",
            "documentation",
            "identity-api",
            slice,
            &fixture(&format!("evidence/{slice}/docs.yaml")),
        );
        if matches!(sources, Sources::DocsAndLegacy) {
            extract_finalize(
                root,
                "legacy",
                "typescript",
                "identity-api",
                slice,
                &fixture(&format!("evidence/{slice}/legacy.yaml")),
            );
        }
        let evidence_dir = root.join(format!(".specify/slices/{slice}/evidence"));
        assert!(evidence_dir.join("docs.yaml").is_file(), "{slice} docs Evidence persisted");
        if matches!(sources, Sources::DocsAndLegacy) {
            assert!(
                evidence_dir.join("legacy.yaml").is_file(),
                "{slice} legacy Evidence persisted"
            );
        }

        // Synthesis: project the agent response into model.yaml + artifacts.
        fs::write(root.join("synth.json"), fixture(&format!("synthesis/{slice}.json")))
            .expect("write synth response");
        let synth = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "slice", "synthesize", slice, "--from", "synth.json"])
            .assert()
            .success();
        let artifacts: Vec<String> = parse_json(&synth.get_output().stdout)["artifacts"]
            .as_array()
            .expect("artifacts array")
            .iter()
            .map(|a| a.as_str().unwrap_or_default().to_string())
            .collect();
        for expected in
            ["proposal.md", "specs/identity/spec.md", "design.md", "tasks.md", "model.yaml"]
        {
            assert!(artifacts.contains(&expected.to_string()), "{slice} missing {expected}");
        }

        // model.yaml carries inline provenance; `slice validate` flags no
        // staleness; `slice provenance` projects the audit view.
        let validate = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "slice", "validate", slice])
            .assert();
        assert_no_staleness(validate.get_output());
        specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "slice", "provenance", slice])
            .assert()
            .success();

        // Contracts declares a required `contracts` build input; seed the
        // slice tree so request assembly resolves it.
        if target == "contracts" {
            let contracts_dir = root.join(format!(".specify/slices/{slice}/contracts"));
            fs::create_dir_all(&contracts_dir).expect("mkdir slice contracts");
            fs::write(contracts_dir.join("openapi.yaml"), fixture("contracts-input/openapi.yaml"))
                .expect("seed contracts input");
        }

        specify_cmd()
            .current_dir(root)
            .args(["slice", "transition", slice, "refined"])
            .assert()
            .success();

        // Build, prepare phase: assemble + schema-validate + persist request.
        let prepare = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "slice", "build", slice])
            .assert()
            .success();
        let handoff = parse_json(&prepare.get_output().stdout);
        assert_eq!(handoff["slice"], slice);
        assert_eq!(handoff["target"], target);
        assert_eq!(handoff["execution"], "agent");
        assert!(
            journal_has(root, "target.execution.agent"),
            "prepare emits target.execution.agent"
        );

        // Build, finalize phase: validate the golden report + gate `built`.
        fs::write(
            root.join(format!(".specify/slices/{slice}/build/report.yaml")),
            fixture(&format!("reports/{slice}.yaml")),
        )
        .expect("write golden build report");
        let finalize = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "slice", "build", slice, "--phase", "finalize"])
            .assert()
            .success();
        let result = parse_json(&finalize.get_output().stdout);
        assert_eq!(result["status"], "success");
        assert!(journal_has(root, "slice.build.started"));
        assert!(journal_has(root, "slice.build.succeeded"));
        let meta = fs::read_to_string(root.join(format!(".specify/slices/{slice}/metadata.yaml")))
            .expect("read slice metadata");
        assert!(meta.contains("status: built"), "finalize gates `built`, got:\n{meta}");
    }

    /// Assert the rendered `DiagnosticReport` on stdout carries no
    /// slice-model / provenance staleness finding. Tolerates unrelated
    /// adapter findings — the slice-time assertion is specifically "no
    /// staleness".
    fn assert_no_staleness(output: &std::process::Output) {
        let Ok(report) = serde_json::from_slice::<Value>(&output.stdout) else {
            return;
        };
        let Some(findings) = report["findings"].as_array() else {
            return;
        };
        for rule in [
            "slice-model-schema",
            "slice-spec-provenance-stale",
            "slice-model-target-drift",
            "slice-model-source-orphan",
            "slice-model-cross-ref-orphan",
            "slice-model-claim-kind-mismatch",
            "slice-model-id-grammar",
        ] {
            assert!(
                findings.iter().all(|f| f["rule-id"] != rule),
                "staleness rule {rule} must not fire on a freshly synthesized slice: {findings:#?}"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // Non-blocking determinism property
    // ---------------------------------------------------------------------------

    /// Re-running kernel projection twice over a golden synthesis response
    /// yields byte-identical kernel-owned `model.yaml` fields, and the
    /// projection is target-independent: the same Evidence + response
    /// projected for two slices bound to *different* targets yields
    /// identical kernel-owned requirements, and the model carries no
    /// target/adapter field.
    #[test]
    fn kernel_projection_deterministic() {
        let project = Project::init().with_schemas();
        let root = project.root();

        // Two slices bound to different targets; `slice build` resolves the
        // target from `metadata.yaml`, but the synthesis kernel never sees
        // it — that target-independence is what this test pins.
        project.seed_plan(
            "\
name: determinism
sources:
  docs:
    adapter: documentation
    path: ./docs
  legacy:
    adapter: typescript
    path: ./legacy
slices:
  - name: bound-contracts
    project: identity-contracts
    status: pending
    sources:
      - { source: docs, lead: identity-api }
  - name: bound-omnia
    project: identity-service
    status: pending
    sources:
      - { source: docs, lead: identity-api }
",
        );

        let evidence = fixture("evidence/identity-contracts/docs.yaml");
        let response = fixture("synthesis/identity-contracts.json");
        fs::write(root.join("synth.json"), &response).expect("write synth response");

        let mut requirements: Vec<Value> = Vec::new();
        for (slice, target) in [("bound-contracts", "contracts"), ("bound-omnia", "omnia")] {
            specify_cmd()
                .current_dir(root)
                .args(["slice", "create", slice, "--target", target])
                .assert()
                .success();
            let evidence_dir = project.slices_dir().join(format!("{slice}/evidence"));
            fs::create_dir_all(&evidence_dir).expect("mkdir evidence");
            fs::write(evidence_dir.join("docs.yaml"), &evidence).expect("write evidence");

            specify_cmd()
                .current_dir(root)
                .args(["slice", "synthesize", slice, "--from", "synth.json"])
                .assert()
                .success();

            let show = specify_cmd()
                .current_dir(root)
                .args(["--format", "json", "slice", "model", "show", slice])
                .assert()
                .success();
            let model = parse_json(&show.get_output().stdout);
            assert!(model.get("target").is_none(), "kernel model carries no target field");
            assert!(model.get("adapter").is_none(), "kernel model carries no adapter field");
            requirements.push(model["requirements"].clone());
        }

        // Target-independence: the kernel-owned requirements are identical
        // across the two differently-targeted slices.
        assert_eq!(
            requirements[0], requirements[1],
            "kernel-owned requirements must be target-independent"
        );

        // Byte-identical re-projection: a second `--from` over the same golden
        // response reproduces the same `model.yaml` exactly.
        let model_path = project.slices_dir().join("bound-contracts/model.yaml");
        let first = fs::read_to_string(&model_path).expect("first model.yaml");
        specify_cmd()
            .current_dir(root)
            .args(["slice", "synthesize", "bound-contracts", "--from", "synth.json"])
            .assert()
            .success();
        let second = fs::read_to_string(&model_path).expect("second model.yaml");
        assert_eq!(first, second, "re-running projection must be byte-identical");
    }

    // ---------------------------------------------------------------------------
    // Composition accumulation + the baseline-overwrite gate
    // ---------------------------------------------------------------------------
    //
    // The merge-kernel accumulation + gate assertions live in
    // `crates/workflow/tests/merge_slice.rs`. These two tests are
    // the integration layer: they drive whole `specify slice merge run`
    // invocations across several slices and assert (1) the merged baseline
    // at `.specify/specs/composition.yaml` grows monotonically as
    // screen-introducing slices accumulate via `delta.added`, and (2) the
    // `composition-baseline-overwrite-blocked` gate fires (and is overridable
    // with `--allow-composition-replace`) in a realistic multi-slice run.

    /// Read the merged composition baseline's `screens` map. An absent
    /// baseline or a baseline without a `screens` mapping yields an empty
    /// map. Parsed with `serde_saphyr` into a `serde_json::Value`, the same
    /// path the merge engine itself uses.
    fn composition_screens(root: &Path) -> serde_json::Map<String, Value> {
        let path = root.join(".specify/specs/composition.yaml");
        let Ok(text) = fs::read_to_string(&path) else {
            return serde_json::Map::new();
        };
        let doc: Value = serde_saphyr::from_str(&text)
            .unwrap_or_else(|err| panic!("parse {}: {err}", path.display()));
        doc.get("screens").and_then(Value::as_object).cloned().unwrap_or_default()
    }

    /// A one-screen `delta: { added }` composition — the accumulating shape a
    /// non-bootstrap screen-introducing slice emits.
    fn delta_added(screen: &str, name: &str) -> String {
        format!(
            "version: 1\ndelta:\n  added:\n    {screen}:\n      name: {name}\n  modified: {{}}\n  \
         removed: {{}}\n"
        )
    }

    /// A whole-document `screens:` composition — the replacement shape the
    /// overwrite gate guards against once a non-empty baseline exists.
    fn whole_document(screen: &str, name: &str) -> String {
        format!("version: 1\nscreens:\n  {screen}:\n    name: {name}\n")
    }

    /// Create `name`, stage its top-level `composition.yaml`, and drive it to
    /// `built` through the real lifecycle verbs (`slice create` →
    /// `slice transition refined` → `slice transition built`) so
    /// `slice merge run` accepts it. Composition-only by design: the
    /// spec-delta accumulation path is covered elsewhere, so these tests
    /// isolate the composition baseline behaviour.
    fn stage_built_composition_slice(project: &Project, name: &str, composition: &str) {
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", name])
            .assert()
            .success();
        fs::write(project.slices_dir().join(name).join("composition.yaml"), composition)
            .expect("write slice composition");
        for target in ["refined", "built"] {
            specify_cmd()
                .current_dir(project.root())
                .args(["slice", "transition", name, target])
                .assert()
                .success();
        }
    }

    /// Three screen-introducing slices, each contributing one `delta.added`
    /// screen, merged in sequence: the baseline `screens` map must grow
    /// 1 → 2 → 3 with no prior screen lost — the data-loss regression
    /// accumulation closes.
    #[test]
    fn composition_accumulates_across_slices() {
        let project = Project::init().with_schemas();
        let root = project.root();

        let slices = [
            ("intro-home", "home", "Home"),
            ("intro-settings", "settings", "Settings"),
            ("intro-profile", "profile", "Profile"),
        ];

        let mut accumulated: Vec<&str> = Vec::new();
        for (index, (slice, screen, name)) in slices.iter().enumerate() {
            stage_built_composition_slice(&project, slice, &delta_added(screen, name));
            specify_cmd()
                .current_dir(root)
                .args(["slice", "merge", "run", slice])
                .assert()
                .success();

            accumulated.push(screen);
            let screens = composition_screens(root);
            assert_eq!(
                screens.len(),
                index + 1,
                "baseline must hold {} screen(s) after merging {slice}, got {screens:?}",
                index + 1
            );
            for slug in &accumulated {
                assert!(
                    screens.contains_key(*slug),
                    "screen `{slug}` must survive in the accumulated baseline, got {screens:?}"
                );
            }
        }
    }

    /// A whole-document (`screens:`) slice composition over a non-empty
    /// baseline aborts `slice merge run` with
    /// `composition-baseline-overwrite-blocked`; the gate is a precondition
    /// (the baseline is untouched and the slice stays `built`), and
    /// `--allow-composition-replace` authorises the full replacement.
    #[test]
    fn composition_overwrite_gate_blocks() {
        let project = Project::init().with_schemas();
        let root = project.root();

        // Establish a non-empty baseline via an accumulating first slice.
        stage_built_composition_slice(&project, "intro-home", &delta_added("home", "Home"));
        specify_cmd()
            .current_dir(root)
            .args(["slice", "merge", "run", "intro-home"])
            .assert()
            .success();
        assert_eq!(composition_screens(root).len(), 1);

        // A whole-document slice composition is blocked over the non-empty
        // baseline.
        stage_built_composition_slice(
            &project,
            "rewrite-all",
            &whole_document("dashboard", "Dashboard"),
        );
        let blocked = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "slice", "merge", "run", "rewrite-all"])
            .assert()
            .failure();
        assert_eq!(
            parse_stderr(&blocked.get_output().stderr, root)["error"],
            "composition-baseline-overwrite-blocked"
        );

        // Precondition semantics: nothing moved — the baseline is intact and
        // `rewrite-all` is still `built`, so the override can re-run it.
        let preserved = composition_screens(root);
        assert_eq!(preserved.len(), 1, "blocked merge must not touch the baseline");
        assert!(preserved.contains_key("home"), "the prior screen must survive a blocked merge");

        // The narrow override authorises the whole-document replacement.
        specify_cmd()
            .current_dir(root)
            .args(["slice", "merge", "run", "rewrite-all", "--allow-composition-replace"])
            .assert()
            .success();
        let replaced = composition_screens(root);
        assert_eq!(replaced.len(), 1);
        assert!(
            replaced.contains_key("dashboard"),
            "the override replaces the baseline with the slice document, got {replaced:?}"
        );
        assert!(
            !replaced.contains_key("home"),
            "whole-document replacement drops the prior screen, got {replaced:?}"
        );
    }

    // ---------------------------------------------------------------------------
    // Composition accumulation + safety nets (host-only)
    // ---------------------------------------------------------------------------
    //
    // Exercises monotonic composition merge, retroactive cross-slice factoring,
    // and the A3/A4 safety nets without catalog infer or WASM dispatch.

    /// The shared bottom-navigation `footer` group repeated across every UI
    /// screen in the capstone. Structurally identical instances cluster to
    /// one component under the default `--min-occurrences 2`.
    const SHARED_FOOTER: &str = "    footer:
      - group:
          items:
            - icon-button: { bind: home, event: Navigate(Home) }
            - icon-button: { bind: search, event: Navigate(Search) }
";

    /// The same `footer` group with the factored `component: shared-nav`
    /// directive attached (B7 directive-only modification). The skeleton is
    /// byte-identical to [`SHARED_FOOTER`]; only the directive differs.
    const FACTORED_FOOTER: &str = "    footer:
      - group:
          component: shared-nav
          items:
            - icon-button: { bind: home, event: Navigate(Home) }
            - icon-button: { bind: search, event: Navigate(Search) }
";

    /// A minimal, schema-valid build report carrying the A4 `ui-surface`
    /// signal. `target` is the bound omnia adapter; `screens` is the
    /// brief-authored count of screen-bearing requirements this slice
    /// introduces or modifies.
    fn build_report(name: &str, ui_screens: u32) -> String {
        format!(
            "version: 1\nslice: {name}\ntarget: omnia\nstatus: success\nfindings: []\nui-surface:\n  \
         screens: {ui_screens}\n"
        )
    }

    /// Create a slice bound to omnia, stage its top-level `composition.yaml`
    /// and a build report carrying `ui-surface.screens`, drive it to
    /// `refined`, then run `slice build --phase finalize` (which gates the
    /// `Refined → Built` transition and runs the A4 coherence checks).
    /// Returns the parsed `BuildResult` so callers can assert its warnings.
    fn build_ui_slice(root: &Path, name: &str, composition: &str, ui_screens: u32) -> Value {
        specify_cmd()
            .current_dir(root)
            .args(["slice", "create", name, "--target", "omnia"])
            .assert()
            .success();
        let slice_dir = root.join(".specify/slices").join(name);
        fs::write(slice_dir.join("composition.yaml"), composition)
            .expect("write slice composition");
        let build_dir = slice_dir.join("build");
        fs::create_dir_all(&build_dir).expect("mkdir slice build dir");
        fs::write(build_dir.join("report.yaml"), build_report(name, ui_screens))
            .expect("write build report");

        specify_cmd()
            .current_dir(root)
            .args(["slice", "transition", name, "refined"])
            .assert()
            .success();

        let finalize = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "slice", "build", name, "--phase", "finalize"])
            .assert()
            .success();
        parse_json(&finalize.get_output().stdout)
    }

    /// Whether a finalize `BuildResult` carries a warning with `rule-id`.
    fn has_warning(result: &Value, rule_id: &str) -> bool {
        result["warnings"].as_array().is_some_and(|w| w.iter().any(|d| d["rule-id"] == rule_id))
    }

    /// The `component:` directive on `<screen>`'s first `footer` group in the
    /// merged baseline, if any.
    fn screen_footer_component(root: &Path, screen: &str) -> Option<String> {
        composition_screens(root)
            .get(screen)?
            .get("footer")?
            .get(0)?
            .get("group")?
            .get("component")?
            .as_str()
            .map(str::to_string)
    }

    /// Host-only composition acceptance: monotonic accumulation, retroactive
    /// cross-slice factoring, and the non-UI-slice safety nets.
    #[test]
    fn composition_accumulation_and_safety_nets() {
        let project = Project::init().with_schemas();
        let root = project.root();

        accumulate_two_screens(root);
        factor_third_slice(root);
        assert_doc_only_safety_nets(root);
    }

    /// Slices 1 + 2 (`home`, `search`), each carrying the same footer group:
    /// the first is a whole-document baseline, the second a `delta.added`
    /// that accumulates. Asserts the baseline grows 1 → 2 with no loss, and
    /// that a faithful UI slice raises no A4 warning at finalize.
    fn accumulate_two_screens(root: &Path) {
        let home_doc = format!("version: 1\nscreens:\n  home:\n    name: Home\n{SHARED_FOOTER}");
        let result = build_ui_slice(root, "intro-home", &home_doc, 1);
        assert!(
            !has_warning(&result, "composition-unexpected-for-non-ui-slice")
                && !has_warning(&result, "composition-empty-for-ui-slice"),
            "a faithful UI slice is self-consistent at finalize: {result}"
        );
        merge_slice(root, "intro-home");
        assert_eq!(composition_screens(root).len(), 1, "baseline holds home after the first merge");

        let search_doc = format!(
            "version: 1\ndelta:\n  added:\n    search:\n      name: Search\n{}\n  modified: {{}}\n  \
         removed: {{}}\n",
            indent_block(SHARED_FOOTER)
        );
        build_ui_slice(root, "intro-search", &search_doc, 1);
        merge_slice(root, "intro-search");
        let screens = composition_screens(root);
        assert_eq!(screens.len(), 2, "baseline accumulates to home + search");
        assert!(screens.contains_key("home") && screens.contains_key("search"));
    }

    /// Slice 3 (`profile`): the discovering build introduces a new screen
    /// carrying the factored component AND retroactively attaches the
    /// `component:` directive to the two prior-slice screens it did not
    /// author (directive-only `delta.modified`, B7). Asserts monotonic
    /// accumulation to three screens, the directive on all three, and the
    /// presence of the simulated shared component module path.
    fn factor_third_slice(root: &Path) {
        let profile_doc = format!(
            "version: 1\ndelta:\n  added:\n    profile:\n      name: Profile\n{factored}\n  \
         modified:\n    home:\n      name: Home\n{factored}\n    search:\n      name: Search\n\
         {factored}\n  removed: {{}}\n",
            factored = indent_block(FACTORED_FOOTER)
        );
        build_ui_slice(root, "intro-profile", &profile_doc, 3);

        // Simulate the writer sub-brief dropping the shared component module
        // into the live shell tree (B7 code side); merge leaves it as residue.
        let module_path = root.join("shared/src/components/shared-nav.rs");
        fs::create_dir_all(module_path.parent().unwrap()).expect("mkdir shared components");
        fs::write(&module_path, "// shared-nav component (retroactive cross-slice factoring)\n")
            .expect("write shared component module");
        merge_slice(root, "intro-profile");

        let screens = composition_screens(root);
        assert_eq!(
            screens.len(),
            3,
            "baseline accumulates to home + search + profile: {screens:?}"
        );
        for screen in ["home", "search", "profile"] {
            assert_eq!(
                screen_footer_component(root, screen).as_deref(),
                Some("shared-nav"),
                "screen `{screen}` carries the factored component directive"
            );
        }
        assert!(
            module_path.is_file(),
            "the shared component module path is present (B7 code side)"
        );
    }

    /// The documentation-only slice: it reports no UI surface
    /// (`ui-surface.screens: 0`) yet emits a non-empty whole-document
    /// composition — the motivating self-contradiction. Asserts A4 warns at
    /// finalize and A3 stops the merge from wiping the accumulated baseline.
    fn assert_doc_only_safety_nets(root: &Path) {
        let doc_only_doc = "version: 1\nscreens:\n  platform-notes:\n    name: Platform Notes\n";
        let result = build_ui_slice(root, "platform-requirements", doc_only_doc, 0);
        assert!(
            has_warning(&result, "composition-unexpected-for-non-ui-slice"),
            "A4 surfaces the non-UI-slice composition mismatch at finalize: {result}"
        );

        let blocked = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "slice", "merge", "run", "platform-requirements"])
            .assert()
            .failure();
        assert_eq!(
            parse_stderr(&blocked.get_output().stderr, root)["error"],
            "composition-baseline-overwrite-blocked",
            "A3 stops the whole-document doc-only slice from wiping the baseline"
        );

        let final_screens = composition_screens(root);
        assert_eq!(final_screens.len(), 3, "the baseline survives the blocked doc-only merge");
        assert!(!final_screens.contains_key("platform-notes"), "the doc-only screen never landed");
    }

    /// Run `specify slice merge run <name>`, asserting success.
    fn merge_slice(root: &Path, name: &str) {
        specify_cmd().current_dir(root).args(["slice", "merge", "run", name]).assert().success();
    }

    /// Re-indent a top-level (4-space) `footer:` block by two more spaces so
    /// it nests correctly under a `delta.added.<screen>` / `delta.modified.<screen>`
    /// entry (which sits two levels deeper than a top-level `screens.<screen>`).
    fn indent_block(block: &str) -> String {
        block
            .lines()
            .map(|line| if line.is_empty() { String::new() } else { format!("  {line}") })
            .collect::<Vec<_>>()
            .join("\n")
    }
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
        copy_dir(
            &omnia_schema_dir(),
            &expected_cache_dir(&tmp.path().join("workspace/alpha"))
                .join("manifests/targets/omnia"),
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
        let _lock = project.hold_plan_lock();

        let assert =
            specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();
        let stdout = std::str::from_utf8(&assert.get_output().stdout).expect("utf8");
        assert_eq!(stdout, "b\n", "text next should be bare '<name>\\n', got: {stdout:?}");
    }

    #[test]
    fn plan_next_picks_first_pending_json() {
        let project = Project::init();
        project.seed_plan(A_DONE_B_PENDING);
        let _lock = project.hold_plan_lock();

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
        let _lock = project.hold_plan_lock();

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
        let _lock = project.hold_plan_lock();

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
        let _lock = project.hold_plan_lock();

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
        let _lock = project.hold_plan_lock();

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
        let _lock = project.hold_plan_lock();

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
        let _lock = project.hold_plan_lock();

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
        let _lock = project.hold_plan_lock();

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

mod plan_lock {
    //! Dual-driving refusal and the `specify plan lock -- <cmd>` wrapper.
    //!
    //! The plan-state-writing verbs (`plan next`, per-entry
    //! `plan transition`, `slice merge run`) probe
    //! `<plan-root>/.specify/plan.lock` and refuse an unlocked driver with
    //! `plan-lock-not-held` (exit 2); `Project::hold_plan_lock` stands in
    //! for a driver session in those refusal tests. The `plan lock` verb is
    //! the CLI-native acquirer: it holds the lock for the spawned child's
    //! lifetime, passes the child's exit code through, refuses a busy lock
    //! with `plan-lock-busy` (exit 2), and skips re-acquisition under
    //! `SPECIFY_PLAN_LOCK_HELD=1`. This file is the named CLI-test
    //! replacement for the retired `dual-driving-refused` eval scenario.

    use super::support::*;

    /// Absolute path to the `specify` binary under test, used as the child
    /// command for `plan lock -- specify …` round-trips.
    fn specify_bin() -> String {
        assert_cmd::cargo::cargo_bin("specify").to_string_lossy().into_owned()
    }

    #[test]
    fn gated_verbs_refuse_unlocked_driver() {
        // One case per gated verb: (seed, argv, status line that must
        // survive). The surviving status proves the refusal wrote no plan
        // state; the journal must stay absent because the lock probe fires
        // before any event bracket (`plan.entry.advanced`, the
        // `slice.merge.*` pair). The merge slice doesn't even need to exist
        // for the refusal path.
        let cases: [(&str, &[&str], &str); 4] = [
            (SINGLE_PENDING, &["plan", "next"], "status: pending"),
            (A_IN_PROGRESS, &["plan", "transition", "a", "done"], "status: in-progress"),
            (ALL_DONE, &["plan", "transition", "a", "--undo"], "status: done"),
            (A_IN_PROGRESS, &["slice", "merge", "run", "a"], "status: in-progress"),
        ];

        for (seed, args, surviving_status) in cases {
            let project = Project::init();
            project.seed_plan(seed);

            let assert = specify_cmd()
                .current_dir(project.root())
                .args(["--format", "json"])
                .args(args)
                .assert()
                .failure();
            assert_eq!(
                assert.get_output().status.code(),
                Some(2),
                "{args:?}: lock refusal is a validation exit"
            );
            let stderr = parse_stderr(&assert.get_output().stderr, project.root());
            assert_eq!(
                stderr["error"], "plan-lock-not-held",
                "{args:?}: stderr envelope: {stderr}"
            );

            let plan = fs::read_to_string(project.plan_path()).expect("read plan.yaml");
            assert!(
                plan.contains(surviving_status),
                "{args:?}: refusal must not write plan state: {plan}"
            );
            assert!(
                !project.root().join(".specify/journal.jsonl").exists(),
                "{args:?}: refusal must not journal"
            );
        }
    }

    #[test]
    fn gate_one_approved_is_exempt() {
        // The plan-level Gate 1 stamp precedes any driver session — it must
        // succeed without the lock.
        let project = Project::init();
        project.seed_plan(SINGLE_PENDING);

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "transition", "demo", "approved"])
            .assert()
            .success();
    }

    #[test]
    fn gated_verbs_pass_under_lock() {
        let project = Project::init();
        project.seed_plan(SINGLE_PENDING);
        let _lock = project.hold_plan_lock();

        specify_cmd().current_dir(project.root()).args(["plan", "next"]).assert().success();
        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "transition", "foo", "done"])
            .assert()
            .success();
    }

    #[test]
    fn holds_so_child_can_advance() {
        // The wrapper holds the lock for the child's lifetime, so a nested
        // `plan next` (which probes the lock) passes and advances the entry
        // — the end-to-end proof that CLI-owned acquisition satisfies the
        // CLI-owned probe.
        let project = Project::init();
        project.seed_plan(SINGLE_PENDING);

        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "lock", "--", &specify_bin(), "plan", "next"])
            .assert()
            .success();

        let plan = load_plan(&project);
        let foo = plan.entries.iter().find(|e| e.name == "foo").expect("entry foo");
        assert_eq!(foo.status, Status::InProgress, "child `plan next` must advance under the lock");

        // The lock is released once the wrapper's child exits, so a fresh
        // driver session can acquire it again.
        assert_eq!(
            specify_workflow::plan_lock::probe(&project.root().join(".specify/plan.lock"))
                .expect("probe"),
            specify_workflow::plan_lock::LockProbe::Unheld,
            "wrapper must release the lock on child exit"
        );
    }

    #[test]
    fn busy_when_another_driver_holds() {
        // A second driver that finds the lock held fails fast with
        // `plan-lock-busy` (exit 2) before spawning the child.
        let project = Project::init();
        project.seed_plan(SINGLE_PENDING);
        let _held = project.hold_plan_lock();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "lock", "--", &specify_bin(), "--version"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2), "busy lock is a validation exit");
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "plan-lock-busy", "stderr envelope: {stderr}");
    }

    #[cfg(unix)]
    #[test]
    fn passes_child_exit_code_through() {
        // The child's exit code is forwarded unchanged (here, a non-zero
        // code that is neither 0 nor a CLI exit code).
        let project = Project::init();
        project.seed_plan(SINGLE_PENDING);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["plan", "lock", "--", "sh", "-c", "exit 7"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(7), "wrapper must pass the child code");
    }

    #[test]
    fn reentrant_skips_acquire_when_held() {
        // A breakout under a parent `/spec:execute` inherits
        // `SPECIFY_PLAN_LOCK_HELD=1`; the wrapper must skip acquisition
        // rather than deadlock on the lock the parent already holds. The
        // in-process guard stands in for the parent session.
        let project = Project::init();
        project.seed_plan(SINGLE_PENDING);
        let _held = project.hold_plan_lock();

        specify_cmd()
            .current_dir(project.root())
            .env("SPECIFY_PLAN_LOCK_HELD", "1")
            .args(["plan", "lock", "--", &specify_bin(), "plan", "next"])
            .assert()
            .success();

        let plan = load_plan(&project);
        let foo = plan.entries.iter().find(|e| e.name == "foo").expect("entry foo");
        assert_eq!(foo.status, Status::InProgress, "re-entrant child must still advance the entry");
    }

    #[test]
    fn resolves_workspace_lock_via_plan_dir() {
        // The lock anchors at the plan root (`--plan-dir`), so slot-side
        // work locks the workspace, not the slot CWD.
        let project = Project::init();
        let workspace = tempdir().expect("workspace tempdir");

        specify_cmd()
            .current_dir(project.root())
            .args(["--plan-dir", workspace.path().to_str().expect("utf-8 path")])
            .args(["plan", "lock", "--", &specify_bin(), "--version"])
            .assert()
            .success();

        assert!(
            workspace.path().join(".specify/plan.lock").exists(),
            "lock must be created at the --plan-dir workspace root"
        );
        assert!(
            !project.root().join(".specify/plan.lock").exists(),
            "lock must not be created at the slot CWD"
        );
    }
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
        let _lock = project.hold_plan_lock();

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

    // -- plan execute (guest-only verb: native refusal) --------------------------------------

    #[test]
    fn plan_execute_refused_natively() {
        // `plan execute` lives in the shared grammar but runs only in
        // the workflow guest; the native binary refuses it with the
        // standard argument error (wire code `argument`, exit 2) — the
        // mirror image of the guest's native-only refusals.
        let project = Project::init();
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "execute"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "argument");
        let message = stderr["message"].as_str().expect("message string");
        assert!(message.contains("workflow guest"), "{message}");
    }

    // -- plan author (guest-only verb: native refusal) ----------------------------------------

    #[test]
    fn plan_author_refused_natively() {
        // `plan author` (the collapsed /spec:plan flow, RFC-61 S1)
        // lives in the shared grammar but runs only in the workflow
        // guest — the same refusal posture as `plan execute`.
        let project = Project::init();
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "author", "fresh", "--intent", "Fix the typo."])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "argument");
        let message = stderr["message"].as_str().expect("message string");
        assert!(message.contains("workflow guest"), "{message}");
        assert!(!project.plan_path().exists(), "a refused author must not write plan.yaml");
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

    // ===================================================================
    // `specify plan propose` — lead reconciliation
    // (end-to-end coverage of the shipped command surface).
    //
    // `--dry-run` emits the `kind: request` envelope (flat lead catalog +
    // project topology) and writes nothing; `--from` schema-gates the
    // agent response, projects it onto `plan.yaml.slices[]`, and emits the
    // paired `plan.reconcile.{agent,completed}` journal events. JSON shapes
    // are pinned by goldens under `tests/fixtures/plan/`; regenerate with
    // `REGENERATE_GOLDENS=1 cargo nextest run --test plan`.
    // ===================================================================
}

mod propose {
    //! `specify plan propose` CLI tests: dry-run request envelopes,
    //! `--from` happy paths, journal tail, negative gates, and re-propose
    //! semantics, plus the propose-only seeds and helpers.

    use super::support::*;

    // -- propose seeds ----------------------------------------------------

    /// N=1 plan: a single `intent` source, no slices yet (replaceable).
    const PROPOSE_PLAN_N1: &str = "\
name: demo
sources:
  intent:
    adapter: intent
    value: \"Fix a typo in user.rs.\"
slices: []
";

    /// N=1 surveyed inventory: one `intent` lead.
    const PROPOSE_DISCOVERY_N1: &str = "\
## Lead inventory

### intent:fix-typo

- lead: fix-typo
- source: intent
- synopsis: Fix a typo in user.rs.
";

    /// N=1 agent response: omits `project` (kernel auto-binds the sole
    /// project) and carries the explicit slice `name`.
    const PROPOSE_RESPONSE_N1: &str = r#"{
  "version": 1,
  "kind": "response",
  "slices": [
    { "name": "fix-typo", "sources": [{ "source": "intent", "lead": "fix-typo" }] }
  ]
}"#;

    /// Workspace registry with two projects bound to different target adapters —
    /// the topology the fan-out response binds against.
    const PROPOSE_REGISTRY_WORKSPACE: &str = "\
version: 1
projects:
  - name: identity-contracts
    url: git@github.com:org/identity-contracts.git
    adapter: contracts@1.0.0
    description: Versioned API contracts crate for the identity domain.
  - name: identity-service
    url: git@github.com:org/identity-service.git
    adapter: omnia@1.0.0
    description: Omnia identity service implementing auth and password flows.
";

    /// Workspace surveyed inventory: four leads across `docs` + `legacy` (the
    /// proposal-schema envelope example, in document order).
    const PROPOSE_DISCOVERY_WORKSPACE: &str = "\
## Lead inventory

### docs:identity-api

- lead: identity-api
- source: docs
- synopsis: Identity API contract for authentication and account access.

### legacy:identity-api

- lead: identity-api
- source: legacy
- synopsis: Legacy identity endpoints.

### docs:password-reset

- lead: password-reset
- source: docs
- synopsis: Users can request a password reset email.

### legacy:reset-password

- lead: reset-password
- source: legacy
- synopsis: Legacy reset-password flow.
";

    /// Committed `.specify/topology.lock` for the workspace fixture —
    /// the projection `workspace sync` would derive from each member
    /// project's `project.yaml`. Descriptions mirror the registry seeds so
    /// the request envelope's `projects[]` stays the authoritative shape.
    const PROPOSE_TOPOLOGY_WORKSPACE: &str = "\
version: 1
projects:
  - name: identity-contracts
    target: contracts@1.0.0
    description: Versioned API contracts crate for the identity domain.
  - name: identity-service
    target: omnia@1.0.0
    description: Omnia identity service implementing auth and password flows.
";

    /// Workspace plan declaring the two surveyed source keys, no slices yet.
    const PROPOSE_PLAN_WORKSPACE: &str = "\
name: identity-revamp
sources:
  docs:
    adapter: documentation
    path: ./docs
  legacy:
    adapter: typescript
    path: ./legacy
slices: []
";

    /// Multi-source fan-out response (the proposal-schema envelope
    /// example): the `identity-api` lead is referenced by two slices
    /// (`identity-contracts` + `identity-service`, joined by `depends-on`);
    /// `password-reset` is a single slice matched across sources by summary.
    const PROPOSE_RESPONSE_FANOUT: &str = r#"{
  "version": 1,
  "kind": "response",
    "slices": [
    {
      "name": "identity-contracts",
      "sources": [
        { "source": "docs", "lead": "identity-api" },
        { "source": "legacy", "lead": "identity-api" }
      ],
      "project": "identity-contracts",
      "rationale": "identity API surface matched by shared slug across docs + legacy"
    },
    {
      "name": "identity-service",
      "sources": [
        { "source": "docs", "lead": "identity-api" },
        { "source": "legacy", "lead": "identity-api" }
      ],
      "project": "identity-service",
      "depends-on": ["identity-contracts"]
    },
    {
      "name": "password-reset",
      "sources": [
        { "source": "docs", "lead": "password-reset" },
        { "source": "legacy", "lead": "reset-password" }
      ],
      "project": "identity-service",
      "rationale": "password-reset (docs) and reset-password (legacy) are the same flow by synopsis judgment"
    }
  ]
}"#;

    // -- propose helpers --------------------------------------------------

    /// Build a minimal `discovery.md` body with one `### source:lead` block
    /// per `(source, lead)` pair — mirrors the kernel unit-test
    /// seeding so negative fixtures stay one-liners.
    fn discovery_doc(leads: &[(&str, &str)]) -> String {
        use std::fmt::Write as _;
        let mut body = String::from("## Lead inventory\n\n");
        for (source, lead) in leads {
            let _ = write!(
                body,
                "### {source}:{lead}\n\n\
             - lead: {lead}\n\
             - source: {source}\n\
             - synopsis: {lead} synopsis.\n\n",
            );
        }
        body
    }

    fn seed_discovery(root: &Path, body: &str) {
        fs::write(root.join("discovery.md"), body).expect("write discovery.md");
    }

    /// Write a `--from` response file under `root`, returning its path.
    fn write_response(root: &Path, body: &str) -> PathBuf {
        let path = root.join("response.json");
        fs::write(&path, body).expect("write response.json");
        path
    }

    /// Scaffold a workspace project in a fresh tempdir, seeding
    /// `registry.yaml`, `discovery.md`, and `plan.yaml`.
    fn workspace_project(registry: &str, discovery: &str, plan: &str) -> TempDir {
        let tmp = tempdir().expect("tempdir");
        init_workspace(&tmp, "platform-workspace");
        fs::write(tmp.path().join("registry.yaml"), registry).expect("write registry.yaml");
        seed_discovery(tmp.path(), discovery);
        fs::write(tmp.path().join("plan.yaml"), plan).expect("write plan.yaml");
        // Workspace plan-time topology reads the committed cache, not the
        // registry. Seed the projection `workspace sync` would produce for
        // the remote members (which a unit test cannot materialise).
        fs::write(tmp.path().join(".specify/topology.lock"), PROPOSE_TOPOLOGY_WORKSPACE)
            .expect("write topology.lock");
        tmp
    }

    /// Run `plan propose --from <body>` expecting an exit-2 abort and
    /// return the parsed `--format json` stderr envelope.
    fn propose_from_stderr(root: &Path, body: &str) -> Value {
        let response = write_response(root, body);
        let assert = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "plan", "propose", "--from"])
            .arg(&response)
            .assert()
            .failure();
        assert_eq!(
            assert.get_output().status.code(),
            Some(2),
            "every propose --from invariant aborts at exit 2"
        );
        parse_stderr(&assert.get_output().stderr, root)
    }

    /// Run `plan propose --from <body>` expecting success and return the
    /// parsed `--format json` stdout summary.
    fn propose_from_ok(root: &Path, body: &str) -> Value {
        let response = write_response(root, body);
        let assert = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "plan", "propose", "--from"])
            .arg(&response)
            .assert()
            .success();
        parse_stdout(&assert.get_output().stdout, root)
    }

    // -- dry-run request envelope goldens --------------------------------

    #[test]
    fn propose_dry_run_n1_request_golden() {
        // N=1: the sole regular project is synthesised from `project.yaml`
        // (`test-proj` → `omnia@1.0.0`); one `intent` lead surfaces.
        let project = Project::init();
        project.seed_plan(PROPOSE_PLAN_N1);
        seed_discovery(project.root(), PROPOSE_DISCOVERY_N1);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "propose", "--dry-run"])
            .assert()
            .success();
        let actual = parse_stdout(&assert.get_output().stdout, project.root());

        assert_eq!(actual["kind"], "request");
        assert_eq!(actual["projects"].as_array().expect("projects").len(), 1);
        assert_eq!(actual["projects"][0]["name"], "test-proj");
        assert_eq!(actual["projects"][0]["target"], "omnia@1.0.0");
        assert_eq!(actual["leads"].as_array().expect("leads").len(), 1);
        assert_eq!(actual["leads"][0]["source"], "intent");
        assert_eq!(actual["leads"][0]["lead"], "fix-typo");

        // The plan is untouched by --dry-run.
        assert_eq!(fs::read_to_string(project.plan_path()).expect("read plan"), PROPOSE_PLAN_N1);

        // The plan scratch lane is recreated empty for the response envelope.
        let lane = project.root().join(".specify/scratch/plan");
        assert!(lane.is_dir(), "dry-run must create the plan scratch lane");
        assert_eq!(
            fs::read_dir(&lane).expect("read lane").count(),
            0,
            "dry-run must leave the plan scratch lane empty"
        );

        assert_golden("propose-dry-run-n1-request.json", actual);
    }

    #[test]
    fn propose_dry_run_clears_stale_response() {
        // A prior run's response envelope must never survive a fresh
        // dry-run, so `--from` cannot consume a stale grouping.
        let project = Project::init();
        project.seed_plan(PROPOSE_PLAN_N1);
        seed_discovery(project.root(), PROPOSE_DISCOVERY_N1);
        let lane = project.root().join(".specify/scratch/plan");
        fs::create_dir_all(&lane).expect("mkdir plan lane");
        fs::write(lane.join("propose-response.json"), "{}").expect("seed stale response");

        specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "propose", "--dry-run"])
            .assert()
            .success();

        assert!(
            !lane.join("propose-response.json").exists(),
            "dry-run must drop a stale response envelope"
        );
    }

    // -- `--from` happy-path goldens -------------------------------------

    #[test]
    fn propose_from_n1_auto_bind_golden() {
        let project = Project::init();
        project.seed_plan(PROPOSE_PLAN_N1);
        seed_discovery(project.root(), PROPOSE_DISCOVERY_N1);

        let actual = propose_from_ok(project.root(), PROPOSE_RESPONSE_N1);
        assert_eq!(actual["plan"]["name"], "demo");
        assert_eq!(actual["slice-count"], 1);
        assert_eq!(actual["slice-names"], serde_json::json!(["fix-typo"]));
        assert_golden("propose-from-n1-summary.json", actual);

        // The projected plan: one slice, target derived from the
        // auto-bound project, structured source binding.
        let plan = Plan::load(&project.plan_path()).expect("load plan");
        assert_eq!(plan.entries.len(), 1);
        let entry = &plan.entries[0];
        assert_eq!(entry.name, "fix-typo");
        // Target is no longer stored on the slice; the bound project is the
        // sole binding and the target resolves from it on demand.
        assert_eq!(entry.project.as_deref(), Some("test-proj"));
        assert_eq!(entry.sources.len(), 1);
        assert_eq!(entry.sources[0].source(), "intent");
        assert_eq!(entry.sources[0].lead("fix-typo"), "fix-typo");
    }

    // -- journal tail -----------------------------------------------------

    #[test]
    fn propose_from_emits_single_journal_tail() {
        let tmp = workspace_project(
            PROPOSE_REGISTRY_WORKSPACE,
            PROPOSE_DISCOVERY_WORKSPACE,
            PROPOSE_PLAN_WORKSPACE,
        );
        let response = write_response(tmp.path(), PROPOSE_RESPONSE_FANOUT);
        specify_cmd()
            .current_dir(tmp.path())
            .args(["plan", "propose", "--from"])
            .arg(&response)
            .assert()
            .success();

        let raw =
            fs::read_to_string(tmp.path().join(".specify/journal.jsonl")).expect("read journal");
        let events: Vec<Value> = raw
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).expect("journal line is JSON"))
            .collect();
        assert_eq!(events.len(), 1, "exactly one reconcile event fires, got:\n{events:#?}");

        // The former agent/completed pair folded into one
        // `plan.reconcile.completed` event carrying the slice names in order.
        let completed = &events[0];
        assert_eq!(completed["event"], "plan.reconcile.completed");
        assert_eq!(completed["payload"]["plan-name"], "identity-revamp");
        assert_eq!(completed["payload"]["slice-count"], 3);
        assert_eq!(
            completed["payload"]["slice-names"],
            serde_json::json!(["identity-contracts", "identity-service", "password-reset"])
        );
    }

    // -- negative: command-mode + response read/parse gates --------------

    #[test]
    fn propose_mode_required() {
        let project = Project::init();
        project.seed_plan("name: demo\nslices: []\n");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "propose"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        let body = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(body["error"], "plan-propose-mode-required");
    }

    #[test]
    fn propose_response_not_found() {
        let project = Project::init();
        project.seed_plan("name: demo\nslices: []\n");
        let missing = project.root().join("absent.json");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "propose", "--from"])
            .arg(&missing)
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        let body = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(body["error"], "plan-propose-response-not-found");
    }

    #[test]
    fn propose_response_schema_rejected() {
        let project = Project::init();
        project.seed_plan("name: demo\nslices: []\n");
        seed_discovery(project.root(), &discovery_doc(&[("docs", "a")]));

        // Drop the required `kind` discriminator: the envelope matches
        // neither `oneOf` branch and is rejected by the schema gate before
        // the structural deserialise.
        let body = propose_from_stderr(
            project.root(),
            r#"{"version":1,"slices":[{"name":"a","sources":[{"source":"docs","lead":"a"}]}]}"#,
        );
        assert_eq!(body["error"], "proposal-schema");
    }

    // -- negative: propagated `plan-reconcile-*` codes -------------------
    //
    // One representative fixture: each `plan-reconcile-*` invariant is
    // exercised per-code at the kernel unit layer
    // (`crates/workflow/src/change/plan/core/propose/tests.rs`); the binary
    // layer only locks the exit-2 stderr propagation path they all share.

    #[test]
    fn propose_reconcile_lead_orphan() {
        let project = Project::init();
        project.seed_plan("name: demo\nslices: []\n");
        seed_discovery(project.root(), &discovery_doc(&[("docs", "real")]));

        let body = propose_from_stderr(
            project.root(),
            r#"{"version":1,"kind":"response","slices":[{"name":"s","sources":[{"source":"docs","lead":"ghost"}]}]}"#,
        );
        assert_eq!(body["error"], "plan-reconcile-lead-orphan");
    }

    #[test]
    fn propose_dry_run_empty_catalog() {
        // `plan-reconcile-empty-catalog` is reachable via --dry-run (no
        // surveyed leads). Under --from it is masked by lead-orphan /
        // partition, since a schema-valid response must cite at least one
        // lead against the empty catalog.
        let project = Project::init();
        project.seed_plan("name: demo\nslices: []\n");
        // Deliberately no discovery.md.

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "plan", "propose", "--dry-run"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        let body = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(body["error"], "plan-reconcile-empty-catalog");
    }

    // -- re-propose semantics --------------------------------------------

    #[test]
    fn propose_re_propose_replaces_all_slices() {
        // `--from` is a wholesale projection, not a merge: a second run on a
        // still-pending plan replaces the prior slice set entirely.
        let project = Project::init();
        project.seed_plan(PROPOSE_PLAN_N1);
        seed_discovery(project.root(), PROPOSE_DISCOVERY_N1);

        propose_from_ok(
            project.root(),
            r#"{"version":1,"kind":"response","slices":[{"name":"first","sources":[{"source":"intent","lead":"fix-typo"}]}]}"#,
        );
        let plan_after_first = Plan::load(&project.plan_path()).expect("load plan");
        assert_eq!(
            plan_after_first.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["first"]
        );

        propose_from_ok(
            project.root(),
            r#"{"version":1,"kind":"response","slices":[{"name":"second","sources":[{"source":"intent","lead":"fix-typo"}]}]}"#,
        );
        let plan_after_second = Plan::load(&project.plan_path()).expect("load plan");
        assert_eq!(
            plan_after_second.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["second"],
            "the second --from wholesale-replaces the first slice set"
        );
    }

    #[test]
    fn propose_refuses_on_approved_plan() {
        // Once the operator stamps Gate 1 (`approved`), the plan is no
        // longer replaceable and `--from` aborts.
        let project = Project::init();
        project.seed_plan(PROPOSE_PLAN_N1);
        seed_discovery(project.root(), PROPOSE_DISCOVERY_N1);

        propose_from_ok(project.root(), PROPOSE_RESPONSE_N1);
        specify_cmd()
            .current_dir(project.root())
            .args(["plan", "transition", "demo", "approved"])
            .assert()
            .success();

        let body = propose_from_stderr(project.root(), PROPOSE_RESPONSE_N1);
        assert_eq!(body["error"], "plan-reconcile-plan-not-replaceable");
    }
}
