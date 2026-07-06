//! Integration tests for the `specify slice` subcommand tree.
//!
//! Every test stands up a fresh `.specify/` project via `specify init`,
//! drives `specify slice *` through `assert_cmd`, and inspects both the
//! structured stdout (`--format json`) and the on-disk side effects the
//! verb is responsible for.
//!
//! Test style follows `tests/e2e.rs`: favour end-to-end execution of the
//! built binary over unit tests so the behaviour the skills consume is
//! the behaviour under test.
//!
//! The suite is split across themed submodules under `tests/slice/`;
//! shared imports, helpers, and seeds live in [`support`].

mod support {
    //! Shared imports, helpers, and seeds for the `slice` integration suite.
    //!
    //! The suite was a single ~1,700-line file; it is now split across the
    //! sibling `#[path]` submodules (`create`, `transition`, `touched_specs`,
    //! `overlap`, `drop`, `metadata`, `validate`, `provenance`, `model_show`,
    //! `validate_file_location`, `validate_catalog`). Every
    //! submodule pulls its shared surface in with `use super::support::*;`,
    //! so the common imports, helpers, and seeds live here once.

    pub use std::fs;

    pub use crate::common::{Project, parse_json, specify_cmd};

    // ---------------------------------------------------------------------------
    // Shared seeds
    // ---------------------------------------------------------------------------

    pub const PLAN_WITH_LEGACY_MONOLITH: &str = "\
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

    pub const CLEAN_SPEC_MD: &str = "### Requirement: Password reset request

ID: REQ-001
Sources: [legacy-monolith]
Status: agreed

The system lets a registered user request a password reset link by email.
";

    /// Minimal projectable `model.yaml` for a slice named `my-slice` on the
    /// earned core (`requirements` + `tasks`), with one fully-projected
    /// requirement (kernel-owned fields present), so `slice provenance` can
    /// reshape it into the audit view. `value` / `path` are no longer stored
    /// here — the projection reads them from `evidence/<source>.yaml`.
    pub const CLEAN_MODEL_YAML: &str = "version: 1
slice: my-slice
requirements:
  - id: REQ-001
    title: Password reset request
    status: agreed
    sources: [legacy-monolith]
    claims:
      - source: legacy-monolith
        id: password-reset.request
        kind: requirement
    statement: The system lets a registered user request a password reset link by email.
tasks: []
";

    /// Evidence the synthesis kernel resolves authority and anchors claims
    /// against. One `requirement` claim, behaviour authority. Used by
    /// the `plan_dir` suite.
    pub const SYNTH_EVIDENCE_YAML: &str = "authority: behaviour
lead: my-slice
claims:
  - id: password-reset.request
    kind: requirement
    statement: \"The system lets a user request a reset link.\"
    path: src/users/reset.ts#L42
";

    // ---------------------------------------------------------------------------
    // Shared helpers
    // ---------------------------------------------------------------------------

    /// Stage a slice with one bound source's Evidence but **no**
    /// `plan.yaml` — the workspace-slot shape, where the governing plan
    /// lives elsewhere. Callers either seed a project-root plan
    /// ([`stage_synthesizable_slice`]-style) or point `--plan-dir` at one.
    pub fn stage_synthesizable_slice_without_plan() -> Project {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();
        let evidence_dir = project.slices_dir().join("my-slice/evidence");
        fs::create_dir_all(&evidence_dir).expect("mkdir evidence");
        fs::write(evidence_dir.join("legacy-monolith.yaml"), SYNTH_EVIDENCE_YAML)
            .expect("write evidence");
        project
    }

    /// Stage a slice on disk and seed `<slice>/specs/login/spec.md`
    /// directly, plus optionally a `plan.yaml` at the project root, so the
    /// provenance gate inside `specify slice validate` has both the spec
    /// file and a plan-level source-bindings context to cross-validate
    /// against. Returns the project handle so the caller can drive
    /// `specify slice validate` on it.
    pub fn stage_slice_with_spec(spec_md: &str, plan_yaml: Option<&str>) -> Project {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();
        let specs_dir = project.slices_dir().join("my-slice/specs/login");
        fs::create_dir_all(&specs_dir).expect("mkdir specs/login");
        fs::write(specs_dir.join("spec.md"), spec_md).expect("write spec.md");
        if let Some(yaml) = plan_yaml {
            project.seed_plan(yaml);
        }
        project
    }

    /// Assert the rendered `DiagnosticReport` on stdout carries no finding
    /// citing `rule_id`. Tolerates an empty stdout (e.g. a `--dump-model`
    /// short-circuit) by treating it as "no findings".
    pub fn assert_no_finding(output: &std::process::Output, rule_id: &str) {
        let report: serde_json::Value = match serde_json::from_slice(&output.stdout) {
            Ok(value) => value,
            Err(_) => return,
        };
        if let Some(findings) = report["findings"].as_array() {
            for finding in findings {
                assert_ne!(
                    finding["rule-id"], rule_id,
                    "no `{rule_id}` finding may appear; got: {findings:#?}"
                );
            }
        }
    }

    /// Locate the rendered diagnostic on stdout for `rule_id` and return
    /// its operator-facing `impact` row. Asserts exit
    /// 2 along the way so callers can focus on the impact text.
    pub fn find_finding_impact(output: &std::process::Output, rule_id: &str) -> String {
        let err = parse_json(&output.stderr);
        assert_eq!(err["exit-code"], 2);
        let report = parse_json(&output.stdout);
        let findings = report["findings"].as_array().expect("findings array");
        findings
            .iter()
            .find(|r| r["rule-id"] == rule_id)
            .and_then(|r| r["impact"].as_str())
            .unwrap_or_else(|| panic!("`{rule_id}` finding must be present in {findings:#?}"))
            .to_string()
    }
}

mod create {
    //! `slice create` CLI tests.

    use super::support::*;

    #[test]
    fn create_writes_dir_and_metadata() {
        let project = Project::init();
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "create", "my-slice"])
            .assert()
            .success();

        let value = parse_json(&assert.get_output().stdout);
        let dir = value["dir"].as_str().expect("dir string");
        assert!(dir.ends_with("/my-slice"), "dir should end with /my-slice, got: {dir}");
        assert_eq!(value["status"], "refining");
        let target = value["target"].as_str().expect("target string");
        assert!(target.starts_with("file://"));
        assert!(
            target.ends_with("/omnia.wasm"),
            "target should be the component URI, got: {target}"
        );
        assert_eq!(value["created"], true);
        assert_eq!(value["restarted"], false);

        let slice_dir = project.slices_dir().join("my-slice");
        assert!(slice_dir.is_dir(), "slice dir must exist");
        assert!(slice_dir.join("specs").is_dir(), "specs/ must exist");
        let meta = fs::read_to_string(slice_dir.join("metadata.yaml")).expect("read metadata");
        assert!(meta.contains("status: refining"));
        assert!(meta.contains("file://") && meta.contains("omnia.wasm"));
        assert!(meta.contains("created-at:"));
    }

    #[test]
    fn create_rejects_uppercase_name() {
        let project = Project::init();
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "create", "BadName"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(1));
        let value = parse_json(&assert.get_output().stderr);
        assert_eq!(value["error"], "invalid-name");
        assert!(
            value["message"].as_str().unwrap().contains("kebab-case")
                || value["message"].as_str().unwrap().contains("invalid name")
        );
    }

    #[test]
    fn create_errors_on_collision() {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "create", "my-slice"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(1));
        let value = parse_json(&assert.get_output().stderr);
        assert_eq!(value["error"], "slice-already-exists");
        assert!(value["message"].as_str().unwrap().contains("already exists"));
    }

    #[test]
    fn create_continue_reuses_existing_dir() {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "create", "my-slice", "--if-exists", "continue"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        assert_eq!(value["created"], false);
        assert_eq!(value["restarted"], false);
    }
}

mod decisions {
    //! Integration tests for the Decision Record gate in
    //! `specify slice validate` — the five `decision-*` findings over
    //! `<slice>/decisions/*.md`.
    //!
    //! Each test crafts a slice that trips exactly one finding and asserts
    //! it fires (blocking, exit 2); the clean slice asserts none fire. Test
    //! style follows `tests/slice_drift.rs`: drive the built binary and
    //! inspect the rendered `DiagnosticReport` on stdout.

    use std::fs;

    use crate::common::{Project, parse_json, specify_cmd};

    /// Body of a well-formed slice-authored Decision Record.
    fn record(slug: &str, status: &str, supersedes: &[&str]) -> String {
        let sup = if supersedes.is_empty() {
            String::new()
        } else {
            format!("supersedes: [{}]\n", supersedes.join(", "))
        };
        format!(
            "---\nslug: {slug}\nstatus: {status}\n{sup}---\n# Title for {slug}\n\n\
         ## Context\nWhy.\n\n## Decision\nWhat.\n\n## Consequences\nTrade-offs.\n"
        )
    }

    /// Stage `my-slice` with the given `decisions/<file>` entries and
    /// optional promoted baseline records under `.specify/decisions/`.
    fn stage(decisions: &[(&str, String)], baseline: &[(&str, String)]) -> Project {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();

        let dir = project.slices_dir().join("my-slice").join("decisions");
        fs::create_dir_all(&dir).expect("mkdir slice decisions");
        for (file, body) in decisions {
            fs::write(dir.join(file), body).expect("write slice decision");
        }

        if !baseline.is_empty() {
            let base = project.root().join(".specify/decisions");
            fs::create_dir_all(&base).expect("mkdir baseline decisions");
            for (file, body) in baseline {
                fs::write(base.join(file), body).expect("write baseline decision");
            }
        }
        project
    }

    fn validate(project: &Project) -> std::process::Output {
        specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert()
            .get_output()
            .clone()
    }

    fn assert_fires(output: &std::process::Output, rule_id: &str) {
        assert_eq!(output.status.code(), Some(2), "decision findings must gate exit 2");
        let report = parse_json(&output.stdout);
        let findings = report["findings"].as_array().expect("findings array");
        assert!(
            findings.iter().any(|f| f["rule-id"] == rule_id),
            "expected finding `{rule_id}` in: {findings:#?}"
        );
    }

    fn assert_silent(output: &std::process::Output, rule_id: &str) {
        let Ok(report) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
            return;
        };
        if let Some(findings) = report["findings"].as_array() {
            for finding in findings {
                assert_ne!(finding["rule-id"], rule_id, "`{rule_id}` must not fire: {findings:#?}");
            }
        }
    }

    #[test]
    fn clean_record_raises_no_decision_findings() {
        let project = stage(&[("use-postgres.md", record("use-postgres", "accepted", &[]))], &[]);
        let output = validate(&project);
        for rule in [
            "decision-record-schema",
            "decision-record-section-missing",
            "decision-slug-grammar",
            "decision-slug-collision",
            "decision-supersede-orphan",
        ] {
            assert_silent(&output, rule);
        }
    }

    #[test]
    fn missing_section_fires() {
        let body = "---\nslug: ok\nstatus: accepted\n---\n# T\n\n## Context\nc\n\n## Decision\nd\n";
        let project = stage(&[("ok.md", body.to_string())], &[]);
        assert_fires(&validate(&project), "decision-record-section-missing");
    }

    #[test]
    fn bad_slug_grammar_fires() {
        let body = record("Bad_Slug", "accepted", &[]);
        let project = stage(&[("bad.md", body)], &[]);
        assert_fires(&validate(&project), "decision-slug-grammar");
    }

    #[test]
    fn bad_schema_fires() {
        // `status: maybe` is not in the closed enum.
        let body = "---\nslug: ok\nstatus: maybe\n---\n# T\n\n## Context\nc\n\n## Decision\nd\n\n## Consequences\ne\n";
        let project = stage(&[("ok.md", body.to_string())], &[]);
        assert_fires(&validate(&project), "decision-record-schema");
    }

    #[test]
    fn slug_collision_fires() {
        let project = stage(
            &[("a.md", record("dup", "accepted", &[])), ("b.md", record("dup", "rejected", &[]))],
            &[],
        );
        assert_fires(&validate(&project), "decision-slug-collision");
    }

    #[test]
    fn supersede_orphan_fires() {
        let project = stage(&[("new.md", record("new-store", "accepted", &["DEC-9999"]))], &[]);
        assert_fires(&validate(&project), "decision-supersede-orphan");
    }

    #[test]
    fn supersede_to_baseline_silent() {
        let baseline = "---\nid: DEC-0001\nslug: old-store\nstatus: accepted\nslice: s\ndate: 2026-06-02\n---\n# Old\n\n## Context\nc\n\n## Decision\nd\n\n## Consequences\ne\n";
        let project = stage(
            &[("new.md", record("new-store", "accepted", &["DEC-0001"]))],
            &[("DEC-0001-old-store.md", baseline.to_string())],
        );
        assert_silent(&validate(&project), "decision-supersede-orphan");
    }

    #[test]
    fn supersede_to_sibling_silent() {
        let project = stage(
            &[
                ("alpha.md", record("alpha", "accepted", &[])),
                ("beta.md", record("beta", "accepted", &["alpha"])),
            ],
            &[],
        );
        assert_silent(&validate(&project), "decision-supersede-orphan");
    }
}

mod drift {
    //! Integration tests for the drift-validation gate in
    //! `specify slice validate` — the seven typed-model findings over
    //! `<slice>/model.yaml`.
    //!
    //! Each test crafts a slice that trips exactly one finding and asserts
    //! it fires; a final clean synthesized slice asserts none of the seven
    //! fire. Test style follows `tests/slice.rs`: drive the built binary
    //! and inspect the rendered `DiagnosticReport` on stdout. Helpers are
    //! `drift_`-prefixed so the file can be shared without name collisions.

    use std::fs;

    use crate::common::{Project, parse_json, specify_cmd};

    // ---------------------------------------------------------------------------
    // Fixtures
    // ---------------------------------------------------------------------------

    /// A clean, fully-projected `model.yaml`: one `agreed` requirement
    /// (REQ-001) citing one in-Evidence claim, one task (TASK-001) that
    /// satisfies it. `project` matches the plan entry below.
    const DRIFT_CLEAN_MODEL: &str = "version: 1
slice: my-slice
project: test-proj
requirements:
  - id: REQ-001
    title: Password reset request
    status: agreed
    domain: password-reset
    sources: [legacy-monolith]
    claims:
      - source: legacy-monolith
        id: password-reset.request
        kind: requirement
    statement: The system lets a registered user request a password reset link by email.
tasks:
  - id: TASK-001
    text: Implement password reset request handling.
    satisfies: [REQ-001]
";

    /// Evidence the clean model's single claim traces to.
    const DRIFT_CLEAN_EVIDENCE: &str = "authority: behaviour
lead: my-slice
claims:
  - id: password-reset.request
    kind: requirement
    statement: \"Password reset request returns a 200 response.\"
    path: src/users/reset.ts#L42
";

    /// `specs/password-reset/spec.md` whose kernel-rendered provenance
    /// lines agree with the clean model (REQ-001 / legacy-monolith / agreed).
    const DRIFT_CLEAN_SPEC: &str = "### Requirement: Password reset request

ID: REQ-001
Sources: legacy-monolith
Status: agreed

The system lets a registered user request a password reset link by email.
";

    /// Plan binding `legacy-monolith` to the `my-slice` entry, project
    /// `test-proj` (matching the clean model).
    const DRIFT_PLAN: &str = "\
name: drift
lifecycle: pending
sources:
  legacy-monolith:
    adapter: typescript
    path: ./legacy
slices:
  - name: my-slice
    status: pending
    project: test-proj
    sources:
      - { source: legacy-monolith, lead: my-slice }
";

    /// Stage `my-slice` with a `model.yaml`, optional Evidence files
    /// (`<key>` → body), optional `specs/<domain>/spec.md` files, and an
    /// optional `plan.yaml`. Returns the project handle for driving
    /// `specify slice validate`.
    fn drift_stage(
        model: &str, evidence: &[(&str, &str)], specs: &[(&str, &str)], plan: Option<&str>,
    ) -> Project {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();
        let slice_dir = project.slices_dir().join("my-slice");
        fs::write(slice_dir.join("model.yaml"), model).expect("write model.yaml");

        if !evidence.is_empty() {
            let evidence_dir = slice_dir.join("evidence");
            fs::create_dir_all(&evidence_dir).expect("mkdir evidence");
            for (key, body) in evidence {
                fs::write(evidence_dir.join(format!("{key}.yaml")), body).expect("write evidence");
            }
        }
        for (domain, body) in specs {
            let domain_dir = slice_dir.join("specs").join(domain);
            fs::create_dir_all(&domain_dir).expect("mkdir specs domain");
            fs::write(domain_dir.join("spec.md"), body).expect("write spec.md");
        }
        if let Some(yaml) = plan {
            project.seed_plan(yaml);
        }
        project
    }

    /// Run `specify slice validate my-slice` and return the process output.
    fn drift_validate(project: &Project) -> std::process::Output {
        specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert()
            .get_output()
            .clone()
    }

    /// Assert the rendered report carries a finding citing `rule_id` and
    /// that the command failed with the blocking exit code 2.
    fn drift_assert_fires(output: &std::process::Output, rule_id: &str) {
        assert_eq!(output.status.code(), Some(2), "drift findings must gate exit 2");
        let report = parse_json(&output.stdout);
        let findings = report["findings"].as_array().expect("findings array");
        assert!(
            findings.iter().any(|f| f["rule-id"] == rule_id),
            "expected finding `{rule_id}` in: {findings:#?}"
        );
    }

    /// Assert the rendered report carries no finding citing `rule_id`.
    fn drift_assert_silent(output: &std::process::Output, rule_id: &str) {
        let Ok(report) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
            return;
        };
        if let Some(findings) = report["findings"].as_array() {
            for finding in findings {
                assert_ne!(finding["rule-id"], rule_id, "`{rule_id}` must not fire: {findings:#?}");
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Each finding fires on a crafted bad slice
    // ---------------------------------------------------------------------------

    #[test]
    fn drift_flags_model_schema() {
        // `tasks` is required by model.schema.json; omitting it fails the
        // schema (and the typed view still deserialises tasks to empty, so
        // the gate short-circuits to the schema finding alone).
        let model = "version: 1\nslice: my-slice\nrequirements: []\n";
        let project = drift_stage(model, &[], &[], Some(DRIFT_PLAN));
        drift_assert_fires(&drift_validate(&project), "slice-model-schema");
    }

    #[test]
    fn drift_flags_spec_provenance_stale() {
        // Spec on disk says `Status: divergence` (coherent with its own
        // tag) but the model says `agreed` — an operator hand-edited the
        // rendered provenance line without re-synthesising.
        let stale_spec = "### Requirement: Password reset request [divergence]

ID: REQ-001
Sources: legacy-monolith
Status: divergence

The system lets a registered user request a password reset link by email.
";
        let project = drift_stage(
            DRIFT_CLEAN_MODEL,
            &[("legacy-monolith", DRIFT_CLEAN_EVIDENCE)],
            &[("password-reset", stale_spec)],
            Some(DRIFT_PLAN),
        );
        drift_assert_fires(&drift_validate(&project), "slice-spec-provenance-stale");
    }

    #[test]
    fn drift_flags_target_drift() {
        // model.project = test-proj, but the plan entry binds project beta.
        let plan = "\
name: drift
lifecycle: pending
sources:
  legacy-monolith:
    adapter: typescript
    path: ./legacy
slices:
  - name: my-slice
    status: pending
    project: beta
    sources:
      - { source: legacy-monolith, lead: my-slice }
";
        let project = drift_stage(
            DRIFT_CLEAN_MODEL,
            &[("legacy-monolith", DRIFT_CLEAN_EVIDENCE)],
            &[("password-reset", DRIFT_CLEAN_SPEC)],
            Some(plan),
        );
        drift_assert_fires(&drift_validate(&project), "slice-model-target-drift");
    }

    #[test]
    fn drift_flags_source_orphan() {
        // The claim cites an Evidence id that is absent from
        // `evidence/legacy-monolith.yaml`.
        let model = "version: 1
slice: my-slice
project: test-proj
requirements:
  - id: REQ-001
    title: Password reset request
    status: agreed
    domain: password-reset
    sources: [legacy-monolith]
    claims:
      - source: legacy-monolith
        id: ghost-claim
        kind: requirement
    statement: The system lets a registered user request a password reset link by email.
tasks:
  - id: TASK-001
    text: Implement password reset request handling.
    satisfies: [REQ-001]
";
        let project = drift_stage(
            model,
            &[("legacy-monolith", DRIFT_CLEAN_EVIDENCE)],
            &[("password-reset", DRIFT_CLEAN_SPEC)],
            Some(DRIFT_PLAN),
        );
        drift_assert_fires(&drift_validate(&project), "slice-model-source-orphan");
    }

    #[test]
    fn drift_flags_cross_ref_orphan() {
        // TASK-001 satisfies REQ-999, which is well-formed but not a
        // requirement id (so id-grammar stays silent and only cross-ref fires).
        let model = "version: 1
slice: my-slice
project: test-proj
requirements:
  - id: REQ-001
    title: Password reset request
    status: agreed
    domain: password-reset
    sources: [legacy-monolith]
    claims:
      - source: legacy-monolith
        id: password-reset.request
        kind: requirement
    statement: The system lets a registered user request a password reset link by email.
tasks:
  - id: TASK-001
    text: Implement password reset request handling.
    satisfies: [REQ-999]
";
        let project = drift_stage(
            model,
            &[("legacy-monolith", DRIFT_CLEAN_EVIDENCE)],
            &[("password-reset", DRIFT_CLEAN_SPEC)],
            Some(DRIFT_PLAN),
        );
        drift_assert_fires(&drift_validate(&project), "slice-model-cross-ref-orphan");
    }

    #[test]
    fn drift_flags_claim_kind_mismatch() {
        // The model claim says `kind: criterion`, but the matching Evidence
        // claim records `kind: requirement`.
        let model = "version: 1
slice: my-slice
project: test-proj
requirements:
  - id: REQ-001
    title: Password reset request
    status: agreed
    domain: password-reset
    sources: [legacy-monolith]
    claims:
      - source: legacy-monolith
        id: password-reset.request
        kind: criterion
    statement: The system lets a registered user request a password reset link by email.
tasks:
  - id: TASK-001
    text: Implement password reset request handling.
    satisfies: [REQ-001]
";
        let project = drift_stage(
            model,
            &[("legacy-monolith", DRIFT_CLEAN_EVIDENCE)],
            &[("password-reset", DRIFT_CLEAN_SPEC)],
            Some(DRIFT_PLAN),
        );
        drift_assert_fires(&drift_validate(&project), "slice-model-claim-kind-mismatch");
    }

    #[test]
    fn drift_flags_id_grammar() {
        // The task id `TASK-1` violates `^TASK-[0-9]{3}$`. (The schema pins
        // the same pattern, so `slice-model-schema` also fires; the
        // assertion only requires the grammar finding to be present.)
        let model = "version: 1
slice: my-slice
project: test-proj
requirements:
  - id: REQ-001
    title: Password reset request
    status: agreed
    domain: password-reset
    sources: [legacy-monolith]
    claims:
      - source: legacy-monolith
        id: password-reset.request
        kind: requirement
    statement: The system lets a registered user request a password reset link by email.
tasks:
  - id: TASK-1
    text: Implement password reset request handling.
    satisfies: [REQ-001]
";
        let project = drift_stage(
            model,
            &[("legacy-monolith", DRIFT_CLEAN_EVIDENCE)],
            &[("password-reset", DRIFT_CLEAN_SPEC)],
            Some(DRIFT_PLAN),
        );
        drift_assert_fires(&drift_validate(&project), "slice-model-id-grammar");
    }

    // ---------------------------------------------------------------------------
    // A clean synthesized slice trips none of the seven
    // ---------------------------------------------------------------------------

    /// The complete drift surface: a clean model must leave every one of
    /// these silent. (Overall `slice validate` exit is governed by the
    /// separate adapter content rules — `proposal.*`, `specs.*` — which are
    /// outside the C9 drift gate; the suite asserts drift-finding absence,
    /// matching the `slice-catalog-drift` convention in `tests/slice.rs`.)
    const DRIFT_RULE_IDS: [&str; 7] = [
        "slice-model-schema",
        "slice-spec-provenance-stale",
        "slice-model-target-drift",
        "slice-model-source-orphan",
        "slice-model-cross-ref-orphan",
        "slice-model-claim-kind-mismatch",
        "slice-model-id-grammar",
    ];

    #[test]
    fn drift_clean_slice_fires_none() {
        let project = drift_stage(
            DRIFT_CLEAN_MODEL,
            &[("legacy-monolith", DRIFT_CLEAN_EVIDENCE)],
            &[("password-reset", DRIFT_CLEAN_SPEC)],
            Some(DRIFT_PLAN),
        );
        let output = drift_validate(&project);
        for rule_id in DRIFT_RULE_IDS {
            drift_assert_silent(&output, rule_id);
        }
    }
}

mod drop {
    //! `slice drop` CLI tests.

    use super::support::*;

    #[test]
    fn drop_transitions_and_archives() {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args([
                "--format",
                "json",
                "slice",
                "drop",
                "my-slice",
                "--reason",
                "Needs design call-out",
            ])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        assert_eq!(value["status"], "dropped");
        assert_eq!(value["drop-reason"], "Needs design call-out");
        let archive_path = value["archive-path"].as_str().unwrap();
        assert!(archive_path.ends_with("-my-slice"));

        // `metadata.yaml` inside the archive should reflect the drop.
        let archived_meta = fs::read_to_string(format!("{archive_path}/metadata.yaml")).unwrap();
        assert!(archived_meta.contains("status: dropped"));
        assert!(archived_meta.contains("drop-reason: Needs design call-out"));
        assert!(archived_meta.contains("dropped-at:"));
    }
}

mod merge {
    //! Integration tests for `specify slice merge preview` and
    //! `specify slice merge conflict-check`.
    //!
    //! These are the two no-write counterparts to `specify slice merge run`
    //! used by the merge-skill rewrite: `preview` computes the operation
    //! list without touching disk; `conflict-check` flags `type: modified`
    //! baselines that have drifted since `defined_at`.

    use std::fs::{self, File, FileTimes};
    use std::path::Path;
    use std::time::{Duration, SystemTime};

    use crate::common::{Project, copy_dir, parse_json, repo_root, specify_cmd};

    /// Stamp `path` with a fixed mtime comfortably after the 2020
    /// `defined_at` the drift tests seed, so `slice merge conflict-check`'s
    /// `mtime > defined_at` comparison fires deterministically — regardless
    /// of filesystem mtime granularity or host clock. Replaces the former
    /// `sleep`-then-rewrite, which leaned on the live clock advancing past a
    /// coarse fs mtime resolution (flaky on fast machines / coarse FSes).
    fn stamp_mtime_after_defined_at(path: &Path) {
        // 2023-11-14T22:13:20Z — strictly after the seeded `defined_at`
        // of 2020-01-01 and before the far-future 2099 used by the
        // "older" no-drift case.
        let when = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        File::options()
            .write(true)
            .open(path)
            .expect("open baseline to set mtime")
            .set_times(FileTimes::new().set_modified(when))
            .expect("set explicit baseline mtime");
    }

    /// Stage the two-spec fixture content into a fresh slice and drive it to
    /// `refined` through the real CLI verbs (`slice create` →
    /// `slice transition`), instead of staging the `built` fixture and
    /// rewriting `metadata.yaml` by hand (testing.md:45). The merge surface
    /// reads the slice's `specs/` tree, so only the fixture's spec content is
    /// copied in; its `built` `metadata.yaml` is left behind.
    fn stage_refined_slice(project: &Project) {
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();
        let slice_dir = project.slices_dir().join("my-slice");
        copy_dir(
            &repo_root().join("tests/fixtures/e2e/merge-two-spec-slice/specs"),
            &slice_dir.join("specs"),
        );
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "transition", "my-slice", "refined"])
            .assert()
            .success();
    }

    // ---------------------------------------------------------------------------
    // slice merge preview
    // ---------------------------------------------------------------------------

    #[test]
    fn preview_reports_operations() {
        let project = Project::init();
        let slice_dir = project.stage_slice("merge-two-spec-slice");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "merge", "preview", "my-slice"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);

        let specs = value["specs"].as_array().expect("specs array");
        // Two-spec fixture: each spec uses `## ADDED Requirements` with one
        // REQ-001 block, producing exactly one `added` op per spec. The
        // `created-baseline` op only fires for verbatim copies without
        // delta headers (see merge-two-spec.json golden).
        let names: Vec<&str> = specs.iter().map(|s| s["name"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["login", "oauth"]);
        for spec in specs {
            let ops = spec["operations"].as_array().unwrap();
            assert_eq!(ops.len(), 1, "expected one op per spec, got {ops:?}");
            let kind = ops[0]["kind"].as_str().unwrap();
            assert!(
                ["added", "modified", "removed", "renamed", "created-baseline"].contains(&kind),
                "merge-op `kind` must be kebab-case v3 contract, got {kind:?}"
            );
            assert_eq!(kind, "added");
            assert_eq!(ops[0]["id"], "REQ-001");
            assert!(spec["baseline-path"].is_string());
        }

        // No filesystem mutation: no archive, slice dir still in place,
        // baselines under .specify/specs/ untouched.
        assert!(slice_dir.is_dir(), "preview must not archive the slice");
        let archive = project.root().join(".specify/archive");
        assert!(
            !archive.exists() || fs::read_dir(&archive).unwrap().next().is_none(),
            "preview must not create archive entries",
        );
        assert!(
            !project.root().join(".specify/specs/login/spec.md").exists(),
            "preview must not write baselines",
        );
        assert!(
            !project.root().join(".specify/specs/oauth/spec.md").exists(),
            "preview must not write baselines",
        );
    }

    #[test]
    fn preview_doesnt_require_built_status() {
        let project = Project::init();
        // `slice merge run` refuses a non-`built` slice but `slice merge
        // preview` must accept one. Reach `refined` through the real verbs
        // rather than rewriting `metadata.yaml` by hand.
        stage_refined_slice(&project);

        specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "merge", "preview", "my-slice"])
            .assert()
            .success();
    }

    #[test]
    fn preview_emits_readable_text() {
        let project = Project::init();
        project.stage_slice("merge-two-spec-slice");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["slice", "merge", "preview", "my-slice"])
            .assert()
            .success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
        assert!(stdout.contains("login:"));
        assert!(stdout.contains("oauth:"));
        assert!(
            stdout.contains("ADDING: REQ-001"),
            "expected ADDING line in text output, got: {stdout}"
        );
    }

    // ---------------------------------------------------------------------------
    // slice merge conflict-check
    // ---------------------------------------------------------------------------

    #[test]
    fn conflict_check_no_conflicts_unmodified() {
        let project = Project::init();
        project.stage_slice("merge-two-spec-slice");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        let conflicts = value["conflicts"].as_array().unwrap();
        assert!(conflicts.is_empty(), "fixture has only `new` entries, got {conflicts:?}");
    }

    #[test]
    fn conflict_check_flags_modified_newer() {
        let project = Project::init();
        let slice_dir = project.stage_slice("merge-two-spec-slice");

        // Seed a baseline file under .specify/specs/login/spec.md then rewrite
        // the slice's metadata to mark `login` as `modified` with a historic
        // defined_at. touching the baseline afterwards puts its mtime in the
        // future relative to defined_at, producing a conflict.
        let baseline = project.root().join(".specify/specs/login/spec.md");
        fs::create_dir_all(baseline.parent().unwrap()).unwrap();
        fs::write(&baseline, "# Login baseline\n").unwrap();

        let metadata_path = slice_dir.join("metadata.yaml");
        fs::write(
        &metadata_path,
        "target: omnia\nstatus: built\ndefined-at: \"2020-01-01T00:00:00Z\"\ntouched-specs:\n  - name: login\n    type: modified\n",
    )
    .unwrap();

        // Set an explicit baseline mtime after `defined_at` so the drift
        // check fires deterministically, insensitive to clock skew or
        // filesystem mtime resolution.
        stamp_mtime_after_defined_at(&baseline);

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        let conflicts = value["conflicts"].as_array().unwrap();
        assert_eq!(conflicts.len(), 1, "expected one conflict, got {conflicts:?}");
        assert_eq!(conflicts[0]["adapter"], "login");
        assert_eq!(conflicts[0]["defined-at"], "2020-01-01T00:00:00Z");
        assert!(conflicts[0]["baseline-modified-at"].is_string());
    }

    #[test]
    fn conflict_check_no_drift_when_older() {
        let project = Project::init();
        let slice_dir = project.stage_slice("merge-two-spec-slice");

        // Set defined_at to the far future so nothing is "newer".
        let metadata_path = slice_dir.join("metadata.yaml");
        fs::write(
        &metadata_path,
        "target: omnia\nstatus: built\ndefined-at: \"2099-01-01T00:00:00Z\"\ntouched-specs:\n  - name: login\n    type: new\n",
    )
    .unwrap();

        // Seed a baseline contract file (its mtime will be well before 2099).
        let baseline_contract = project.root().join("contracts/schemas/test.yaml");
        fs::create_dir_all(baseline_contract.parent().unwrap()).unwrap();
        fs::write(&baseline_contract, "type: object\n").unwrap();

        // Seed the corresponding slice contract so the drift walker visits it.
        let slice_contract = slice_dir.join("contracts/schemas/test.yaml");
        fs::create_dir_all(slice_contract.parent().unwrap()).unwrap();
        fs::write(&slice_contract, "type: object\nproperties: {}\n").unwrap();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        let conflicts = value["conflicts"].as_array().unwrap();
        assert!(
            conflicts.is_empty(),
            "baseline is older than defined_at, expected no conflicts, got {conflicts:?}"
        );
    }

    #[test]
    fn conflict_check_detects_drift_when_newer() {
        let project = Project::init();
        let slice_dir = project.stage_slice("merge-two-spec-slice");

        // defined_at in the deep past — any real file mtime will be newer.
        let metadata_path = slice_dir.join("metadata.yaml");
        fs::write(
        &metadata_path,
        "target: omnia\nstatus: built\ndefined-at: \"2020-01-01T00:00:00Z\"\ntouched-specs:\n  - name: login\n    type: new\n",
    )
    .unwrap();

        let baseline_contract = project.root().join("contracts/schemas/test.yaml");
        fs::create_dir_all(baseline_contract.parent().unwrap()).unwrap();
        fs::write(&baseline_contract, "type: object\n").unwrap();

        // Set an explicit baseline mtime after `defined_at` so the opaque
        // drift walker reports a conflict deterministically.
        stamp_mtime_after_defined_at(&baseline_contract);

        let slice_contract = slice_dir.join("contracts/schemas/test.yaml");
        fs::create_dir_all(slice_contract.parent().unwrap()).unwrap();
        fs::write(&slice_contract, "type: object\nproperties: {}\n").unwrap();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        let conflicts = value["conflicts"].as_array().unwrap();
        assert_eq!(conflicts.len(), 1, "expected one contract conflict, got {conflicts:?}");
        assert_eq!(conflicts[0]["adapter"], "contracts/schemas/test.yaml");
        assert_eq!(conflicts[0]["defined-at"], "2020-01-01T00:00:00Z");
    }

    #[test]
    fn conflict_check_no_drift_for_new_files() {
        let project = Project::init();
        let slice_dir = project.stage_slice("merge-two-spec-slice");

        let metadata_path = slice_dir.join("metadata.yaml");
        fs::write(
        &metadata_path,
        "target: omnia\nstatus: built\ndefined-at: \"2020-01-01T00:00:00Z\"\ntouched-specs:\n  - name: login\n    type: new\n",
    )
    .unwrap();

        // Slice has a contract file, but no corresponding baseline exists.
        let slice_contract = slice_dir.join("contracts/schemas/new.yaml");
        fs::create_dir_all(slice_contract.parent().unwrap()).unwrap();
        fs::write(&slice_contract, "type: object\n").unwrap();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        let conflicts = value["conflicts"].as_array().unwrap();
        assert!(
            conflicts.is_empty(),
            "new contract files (not in baseline) should not produce conflicts, got {conflicts:?}"
        );
    }

    #[test]
    fn conflict_check_no_drift_no_contracts() {
        let project = Project::init();
        let slice_dir = project.stage_slice("merge-two-spec-slice");

        let metadata_path = slice_dir.join("metadata.yaml");
        fs::write(
        &metadata_path,
        "target: omnia\nstatus: built\ndefined-at: \"2020-01-01T00:00:00Z\"\ntouched-specs:\n  - name: login\n    type: new\n",
    )
    .unwrap();

        // Seed a baseline contract but do NOT create contracts/ in the slice.
        let baseline_contract = project.root().join("contracts/schemas/test.yaml");
        fs::create_dir_all(baseline_contract.parent().unwrap()).unwrap();
        fs::write(&baseline_contract, "type: object\n").unwrap();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        let conflicts = value["conflicts"].as_array().unwrap();
        assert!(
            conflicts.is_empty(),
            "no contracts/ in the slice means no contract drift, got {conflicts:?}"
        );
    }

    #[test]
    fn archive_prune_keeps_recent_by_count() {
        let project = Project::init();
        let archive = project.root().join(".specify/archive");
        fs::create_dir_all(&archive).unwrap();
        for name in ["2026-01-01-alpha", "2026-03-01-beta", "2026-05-01-gamma"] {
            fs::create_dir_all(archive.join(name)).unwrap();
        }

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "archive", "prune", "--keep", "2"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        let pruned = value["pruned"].as_array().unwrap();
        assert_eq!(pruned.len(), 1, "keep 2 of 3 prunes the oldest, got {pruned:?}");
        assert_eq!(pruned[0], "2026-01-01-alpha");

        assert!(!archive.join("2026-01-01-alpha").exists(), "oldest must be removed");
        assert!(archive.join("2026-05-01-gamma").exists(), "newest must remain");
    }

    #[test]
    fn archive_prune_dry_run_removes_nothing() {
        let project = Project::init();
        let archive = project.root().join(".specify/archive");
        fs::create_dir_all(archive.join("2026-01-01-alpha")).unwrap();

        specify_cmd()
            .current_dir(project.root())
            .args(["archive", "prune", "--keep", "0", "--dry-run"])
            .assert()
            .success();

        assert!(archive.join("2026-01-01-alpha").exists(), "dry-run must not remove folders");
    }

    #[test]
    fn archive_prune_requires_a_bound() {
        let project = Project::init();
        fs::create_dir_all(project.root().join(".specify/archive")).unwrap();

        specify_cmd().current_dir(project.root()).args(["archive", "prune"]).assert().failure();
    }

    #[test]
    fn conflict_check_ignores_new_entries() {
        // `type: new` baselines are "we're creating this adapter" — even
        // if a file already exists at the baseline path, it is not a drift
        // conflict in the mtime-vs-defined_at sense, just a different kind
        // of integrity issue the caller should handle separately.
        let project = Project::init();
        project.stage_slice("merge-two-spec-slice");
        let baseline = project.root().join(".specify/specs/login/spec.md");
        fs::create_dir_all(baseline.parent().unwrap()).unwrap();
        fs::write(&baseline, "# Login baseline\n").unwrap();

        // touched_specs keeps the fixture's `new` classification; no
        // `defined_at` means conflict_check returns empty regardless.
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "merge", "conflict-check", "my-slice"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        assert!(value["conflicts"].as_array().unwrap().is_empty());
    }
}

mod metadata {
    //! `SliceMetadata` / `Outcome` serde round-trips, plus the top-level
    //! `--help` axis-verb surface.

    use super::support::*;

    #[test]
    fn metadata_without_outcome_still_parses() {
        use specify_workflow::slice::SliceMetadata;
        // A freshly-created slice writes `metadata.yaml` with no `outcome`
        // key (omitted via `skip_serializing_if`) — byte-for-byte the
        // back-compat shape of metadata that predates the field. Drive
        // creation through `slice create` rather than hand-writing the file
        // (testing.md:45), then assert `SliceMetadata::load` leaves `outcome`
        // as None.
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();
        let slice_dir = project.slices_dir().join("my-slice");
        let meta = SliceMetadata::load(&slice_dir).expect("freshly-created metadata parses");
        assert!(meta.outcome.is_none(), "metadata without an outcome field must load as None");
    }

    #[test]
    fn phase_outcome_round_trips_serde() {
        use specify_workflow::slice::Outcome;
        // Construction via struct literal would require crossing the
        // `#[non_exhaustive]` boundary on `Outcome`; round-trip through
        // YAML instead so the wire shape is what's exercised.
        for kind in ["success", "failure", "deferred"] {
            for phase in ["shape", "build", "merge"] {
                let yaml = format!(
                    "phase: {phase}\noutcome: {kind}\nat: \"2024-08-01T10:00:00Z\"\nsummary: some summary\n"
                );
                let parsed: Outcome = serde_saphyr::from_str(&yaml).expect("parse");
                let reserialised = serde_saphyr::to_string(&parsed).expect("serialize");
                let reparsed: Outcome = serde_saphyr::from_str(&reserialised).expect("reparse");
                assert_eq!(parsed, reparsed, "round-trip failed for yaml:\n{yaml}");
            }
        }
    }

    // ---- Top-level verb surface carries source/target axis verbs ----

    #[test]
    fn help_lists_axis_verbs() {
        // Inventory parsed from the help `Commands:` section, not exact
        // clap wording: the axis verbs must be present and the retired
        // `change` verb must stay gone.
        let verbs = crate::common::help_verbs(&[]);
        for verb in ["slice", "source", "target"] {
            assert!(verbs.iter().any(|v| v == verb), "top level must declare `{verb}`: {verbs:?}");
        }
        let retired = "change";
        assert!(
            !verbs.iter().any(|v| v == retired),
            "retired verb `{retired}` must not resurface: {verbs:?}"
        );
    }
}

mod model_show {
    //! Model viewer — `slice model show`.

    use super::support::*;

    #[test]
    fn model_show_renders_json_and_text() {
        let project = stage_slice_with_spec(CLEAN_SPEC_MD, Some(PLAN_WITH_LEGACY_MONOLITH));
        let slice_dir = project.slices_dir().join("my-slice");
        fs::write(slice_dir.join("model.yaml"), CLEAN_MODEL_YAML).expect("write model.yaml");

        // `--format json` serialises the persisted model verbatim.
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "model", "show", "my-slice"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        assert_eq!(value["slice"], "my-slice");
        assert_eq!(value["requirements"][0]["id"], "REQ-001");
        assert_eq!(value["requirements"][0]["title"], "Password reset request");

        // Text mode prints the concise human view.
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["slice", "model", "show", "my-slice"])
            .assert()
            .success();
        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(stdout.contains("slice: my-slice"), "header must name the slice, got: {stdout}");
        assert!(
            stdout.contains("REQ-001 [agreed] Password reset request"),
            "requirement line must render id/status/title, got: {stdout}"
        );
        assert!(
            stdout.contains("sources: legacy-monolith"),
            "requirement line must render sources, got: {stdout}"
        );
    }

    #[test]
    fn model_show_fails_without_model() {
        let project = stage_slice_with_spec(CLEAN_SPEC_MD, Some(PLAN_WITH_LEGACY_MONOLITH));
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "model", "show", "my-slice"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        let value = parse_json(&assert.get_output().stderr);
        assert_eq!(value["error"], "slice-model-missing");
    }
}

mod overlap {
    //! `slice overlap` CLI tests.

    use super::support::*;

    #[test]
    fn overlap_reports_shared_adapters() {
        let project = Project::init();
        // Two active slices both claim `login`.
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "first"])
            .assert()
            .success();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "second"])
            .assert()
            .success();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "touched-specs", "first", "--set", "login:new,oauth:new"])
            .assert()
            .success();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "touched-specs", "second", "--set", "login:modified"])
            .assert()
            .success();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "overlap", "first"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        let overlaps = value["overlaps"].as_array().unwrap();
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0]["capability"], "login");
        assert_eq!(overlaps[0]["other-slice"], "second");
        assert_eq!(overlaps[0]["our-spec-type"], "new");
        assert_eq!(overlaps[0]["other-spec-type"], "modified");
    }

    #[test]
    fn overlap_empty_for_disjoint_slices() {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "alpha"])
            .assert()
            .success();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "beta"])
            .assert()
            .success();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "touched-specs", "alpha", "--set", "aa:new"])
            .assert()
            .success();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "touched-specs", "beta", "--set", "bb:new"])
            .assert()
            .success();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "overlap", "alpha"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        assert!(value["overlaps"].as_array().unwrap().is_empty());
    }
}

mod plan_dir {
    //! Global `--plan-dir` plan-root override (env `SPECIFY_PLAN_DIR`).
    //!
    //! Workspace routing runs phase verbs inside a materialised slot while
    //! the governing `plan.yaml` stays at the initiating workspace root —
    //! by design no slot grows its own plan. These tests pin the bridge:
    //! slice-time plan readers resolve the plan against the override
    //! instead of the project root, and a wrong override fails with the
    //! same typed error citing the overridden path.

    use tempfile::tempdir;

    use super::support::*;

    /// A clean, fully-projected `model.yaml` whose `project: test-proj`
    /// must agree with the plan entry's `project` — the plan-dependent
    /// `slice-model-target-drift` gate is the probe that proves which
    /// plan `slice validate` consulted.
    const PLAN_DIR_MODEL: &str = "version: 1
slice: my-slice
project: test-proj
requirements:
  - id: REQ-001
    title: Password reset request
    status: agreed
    domain: password-reset
    sources: [legacy-monolith]
    claims:
      - source: legacy-monolith
        id: password-reset.request
        kind: requirement
    statement: The system lets a registered user request a password reset link by email.
tasks:
  - id: TASK-001
    text: Implement password reset request handling.
    satisfies: [REQ-001]
";

    /// Workspace plan whose `my-slice` entry binds `project: <project>`.
    fn plan_binding_project(project: &str) -> String {
        format!(
            "\
name: plan-dir
lifecycle: pending
sources:
  legacy-monolith:
    adapter: typescript
    path: ./legacy
slices:
  - name: my-slice
    status: pending
    project: {project}
    sources:
      - {{ source: legacy-monolith, lead: my-slice }}
"
        )
    }

    /// Stage the slot slice (model + spec on top of the shared Evidence
    /// staging), write `plan_yaml` into a sibling workspace dir, and run
    /// `slice validate --plan-dir <workspace>`, returning the output.
    fn validate_with_workspace_plan(plan_yaml: &str) -> std::process::Output {
        let project = stage_synthesizable_slice_without_plan();
        let slice_dir = project.slices_dir().join("my-slice");
        fs::write(slice_dir.join("model.yaml"), PLAN_DIR_MODEL).expect("write model.yaml");
        let spec_dir = slice_dir.join("specs/password-reset");
        fs::create_dir_all(&spec_dir).expect("mkdir specs/password-reset");
        fs::write(
            spec_dir.join("spec.md"),
            "### Requirement: Password reset request

ID: REQ-001
Sources: legacy-monolith
Status: agreed

The system lets a registered user request a password reset link by email.
",
        )
        .expect("write spec.md");

        let workspace = tempdir().expect("workspace tempdir");
        fs::write(workspace.path().join("plan.yaml"), plan_yaml)
            .expect("write workspace plan.yaml");

        specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "--plan-dir"])
            .arg(workspace.path())
            .args(["slice", "validate", "my-slice"])
            .assert()
            .get_output()
            .clone()
    }

    #[test]
    fn validate_reads_plan_via_flag() {
        // Agreeing projects: the plan-dependent target-drift gate stays
        // silent, proving the override plan satisfied the cross-check.
        let output = validate_with_workspace_plan(&plan_binding_project("test-proj"));
        assert_no_finding(&output, "slice-model-target-drift");

        // Disagreeing projects: the same gate fires — the workspace plan,
        // not a (non-existent) slot plan, is the one consulted.
        let output = validate_with_workspace_plan(&plan_binding_project("beta"));
        assert_eq!(output.status.code(), Some(2));
        let report = parse_json(&output.stdout);
        let findings = report["findings"].as_array().expect("findings array");
        assert!(
            findings.iter().any(|f| f["rule-id"] == "slice-model-target-drift"),
            "target-drift must fire against the override plan, got: {findings:#?}"
        );
    }
}

mod provenance {
    //! Provenance projection — `slice provenance`.

    use super::support::*;

    /// Evidence the provenance projection reads `value` / `path` and
    /// document-level `authority` from when reshaping `CLEAN_MODEL_YAML`.
    const CLEAN_EVIDENCE_YAML: &str = "authority: behaviour
lead: my-slice
claims:
  - id: password-reset.request
    kind: requirement
    statement: \"Password reset request returns a 200 response.\"
    path: src/users/reset.ts#L42
";

    #[test]
    fn provenance_projects_from_model() {
        let project = stage_slice_with_spec(CLEAN_SPEC_MD, Some(PLAN_WITH_LEGACY_MONOLITH));
        let slice_dir = project.slices_dir().join("my-slice");
        fs::write(slice_dir.join("model.yaml"), CLEAN_MODEL_YAML).expect("write model.yaml");
        let evidence_dir = slice_dir.join("evidence");
        fs::create_dir_all(&evidence_dir).expect("mkdir evidence");
        fs::write(evidence_dir.join("legacy-monolith.yaml"), CLEAN_EVIDENCE_YAML)
            .expect("write evidence");
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "provenance", "my-slice"])
            .assert()
            .success();
        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(stdout.contains("REQ-001"), "projection should list REQ-001, got: {stdout}");
        assert!(
            stdout.contains("single-source"),
            "projection should carry the resolution, got: {stdout}"
        );
    }

    #[test]
    fn provenance_fails_without_model() {
        let project = stage_slice_with_spec(CLEAN_SPEC_MD, Some(PLAN_WITH_LEGACY_MONOLITH));
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "provenance", "my-slice"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
    }
}

mod touched_specs {
    //! `slice touched-specs` CLI tests.

    use super::support::*;

    #[test]
    fn touched_specs_classifies_new_vs_modified() {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();
        let slice_dir = project.slices_dir().join("my-slice");

        // Adapter `alpha` — no baseline, should classify as `new`.
        fs::create_dir_all(slice_dir.join("specs/alpha")).unwrap();
        fs::write(slice_dir.join("specs/alpha/spec.md"), "# Alpha\n").unwrap();

        // Adapter `beta` — baseline exists, should classify as `modified`.
        fs::create_dir_all(project.specs_dir().join("beta")).unwrap();
        fs::write(project.specs_dir().join("beta/spec.md"), "# Beta baseline\n").unwrap();
        fs::create_dir_all(slice_dir.join("specs/beta")).unwrap();
        fs::write(slice_dir.join("specs/beta/spec.md"), "# Beta delta\n").unwrap();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "touched-specs", "my-slice", "--scan"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        let items = value["touched-specs"].as_array().expect("touched-specs array");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["name"], "alpha");
        assert_eq!(items[0]["type"], "new");
        assert_eq!(items[1]["name"], "beta");
        assert_eq!(items[1]["type"], "modified");

        // Scanning must have persisted the list into `metadata.yaml`.
        let meta = fs::read_to_string(slice_dir.join("metadata.yaml")).unwrap();
        assert!(meta.contains("touched-specs:"));
        assert!(meta.contains("name: alpha"));
        assert!(meta.contains("type: new"));
        assert!(meta.contains("name: beta"));
        assert!(meta.contains("type: modified"));
    }

    #[test]
    fn touched_specs_accepts_explicit_list() {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args([
                "--format",
                "json",
                "slice",
                "touched-specs",
                "my-slice",
                "--set",
                "alpha:new,beta:modified",
            ])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        let items = value["touched-specs"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["name"], "alpha");
        assert_eq!(items[1]["type"], "modified");
    }
}

mod transition {
    //! `slice transition` CLI tests.

    use super::support::*;

    #[test]
    fn transition_walks_happy_path() {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();

        for target in ["refined", "built"] {
            let assert = specify_cmd()
                .current_dir(project.root())
                .args(["--format", "json", "slice", "transition", "my-slice", target])
                .assert()
                .success();
            let value = parse_json(&assert.get_output().stdout);
            assert_eq!(value["status"], target);
        }

        let meta = fs::read_to_string(project.slices_dir().join("my-slice").join("metadata.yaml"))
            .expect("read metadata");
        assert!(meta.contains("status: built"));
        assert!(meta.contains("defined-at:"));
        assert!(meta.contains("completed-at:"));
    }

    #[test]
    fn transition_rejects_illegal_edge() {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();
        // Refining -> Built is not a legal edge (must pass through refined).
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "transition", "my-slice", "built"])
            .assert()
            .failure();
        let value = parse_json(&assert.get_output().stderr);
        assert_eq!(value["error"], "lifecycle");
    }

    #[test]
    fn transition_rejects_merged_target() {
        // The `merged` lifecycle status is reserved for `slice merge run`,
        // which writes it atomically alongside the spec merge and archive
        // move. Hand-driven `slice transition <name> merged` would skip
        // that bookkeeping, so the dispatcher refuses the value with an
        // argument-error envelope (exit 2) before lifecycle ever runs.
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "transition", "my-slice", "merged"])
            .assert()
            .code(2);
        let value = parse_json(&assert.get_output().stderr);
        assert_eq!(value["error"], "argument");
        assert_eq!(value["exit-code"], 2);
        let message = value["message"].as_str().expect("message string");
        assert!(
            message.contains("specify slice merge run"),
            "argument-error message must redirect to the merge runner; got:\n{message}"
        );
        assert!(
            message.contains("merged"),
            "argument-error message must name the rejected target; got:\n{message}"
        );
    }
}

mod validate {
    //! `slice validate` gates: the workflow §Requirement-block provenance
    //! contract, the metadata-free provenance skip, and the
    //! `discovery-lead-synopsis-thin` advisory.

    use super::support::*;

    /// The validate surface now renders a `DiagnosticReport` on stdout and
    /// fails payload-free: the per-rule discriminant lives in
    /// `findings[].rule-id` on stdout, while stderr carries only the
    /// payload-free `Error::Validation` envelope (exit 2). Assert the
    /// expected `rule_id` appears in the rendered findings exactly.
    fn assert_provenance_fail_rule(output: &std::process::Output, rule_id: &str) {
        let err = parse_json(&output.stderr);
        assert_eq!(err["exit-code"], 2);
        let report = parse_json(&output.stdout);
        let findings = report["findings"].as_array().expect("findings array");
        assert!(
            findings.iter().any(|r| r["rule-id"] == rule_id),
            "expected rule_id `{rule_id}` in findings: {findings:#?}"
        );
    }

    #[test]
    fn validate_rejects_missing_id() {
        let spec = "### Requirement: Missing id\n\n\
                Sources: [legacy-monolith]\n\
                Status: agreed\n\n\
                body\n";
        let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        assert_provenance_fail_rule(assert.get_output(), "spec.requirement-id-missing");
    }

    #[test]
    fn validate_rejects_malformed_id() {
        let spec = "### Requirement: Malformed id\n\n\
                ID: REQ-1\n\
                Sources: [legacy-monolith]\n\
                Status: agreed\n";
        let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        assert_provenance_fail_rule(assert.get_output(), "spec.requirement-id-malformed");
    }

    #[test]
    fn validate_rejects_missing_sources() {
        let spec = "### Requirement: No sources\n\n\
                ID: REQ-001\n\
                Status: agreed\n";
        let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        assert_provenance_fail_rule(assert.get_output(), "spec.requirement-sources-missing");
    }

    #[test]
    fn validate_rejects_missing_status() {
        let spec = "### Requirement: No status\n\n\
                ID: REQ-001\n\
                Sources: [legacy-monolith]\n";
        let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        assert_provenance_fail_rule(assert.get_output(), "spec.requirement-status-missing");
    }

    #[test]
    fn validate_rejects_unknown_status() {
        let spec = "### Requirement: Bogus status\n\n\
                ID: REQ-001\n\
                Sources: [legacy-monolith]\n\
                Status: maybe\n";
        let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        assert_provenance_fail_rule(assert.get_output(), "spec.requirement-status-unknown-value");
    }

    #[test]
    fn validate_rejects_source_not_in_plan() {
        let spec = "### Requirement: Stray source key\n\n\
                ID: REQ-001\n\
                Sources: [phantom]\n\
                Status: agreed\n";
        let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        assert_provenance_fail_rule(assert.get_output(), "spec.requirement-source-undefined");
    }

    #[test]
    fn validate_rejects_tag_status_mismatch() {
        let spec = "### Requirement: Lying tag [divergence]\n\n\
                ID: REQ-001\n\
                Sources: [legacy-monolith]\n\
                Status: agreed\n";
        let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        assert_provenance_fail_rule(assert.get_output(), "spec.requirement-tag-status-mismatch");
    }

    #[test]
    fn skips_provenance_no_metadata() {
        // Metadata-free (pre-synthesis) state. The provenance gate must
        // not fire and the slice progresses to the existing adapter rule
        // run. The adapter rules will still surface deferred /
        // pass-style results — we only assert the provenance rule ids
        // are NOT present.
        let spec = "### Requirement: metadata-free body\n\n\
                ID: REQ-001\n\n\
                body that has no Sources or Status yet\n";
        let project = stage_slice_with_spec(spec, None);
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert();
        // Whether the run passes or fails (existing adapter rules may
        // still produce findings on the synthetic slice), no provenance
        // rule should appear on the rendered report.
        if let Ok(report) = serde_json::from_slice::<serde_json::Value>(&assert.get_output().stdout)
            && let Some(findings) = report["findings"].as_array()
        {
            for finding in findings {
                let rule_id = finding["rule-id"].as_str().unwrap_or("");
                assert!(
                    !rule_id.starts_with("spec.requirement-"),
                    "no provenance rule should fire on a metadata-free spec.md, got: {rule_id}"
                );
            }
        }
    }

    #[test]
    fn flags_thin_synopsis_non_blocking() {
        // A thin same-slug synopsis the agent cannot match or split on,
        // alongside a content-bearing one. The advisory must surface at
        // `suggestion` severity (non-blocking by the shared
        // `blocking_present` predicate — only `critical`/`important`
        // violations gate exit), nudging without parking the slice. Only
        // the thin `docs:identity-api` lead is flagged; the content-bearing
        // `legacy:identity-api` lead is not. (Adapter validation still
        // surfaces unrelated findings on this synthetic slice, so the test
        // asserts on the advisory finding itself rather than the overall
        // exit code — matching the suite's `assert_no_finding` convention.)
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();

        let discovery = "\
# Discovery — identity

## Lead inventory

### docs:identity-api

- lead: identity-api
- source: docs
- synopsis: Identity API.

### legacy:identity-api

- lead: identity-api
- source: legacy
- synopsis: Authentication and account-access API covering login, token refresh, and profile reads.
";
        fs::write(project.root().join("discovery.md"), discovery).expect("write discovery.md");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert();
        let report = parse_json(&assert.get_output().stdout);
        let findings = report["findings"].as_array().expect("findings array");
        let thin: Vec<_> =
            findings.iter().filter(|f| f["rule-id"] == "discovery-lead-synopsis-thin").collect();
        assert_eq!(
            thin.len(),
            1,
            "exactly one thin-synopsis finding expected (only the `docs:identity-api` lead), got: \
         {findings:#?}"
        );
        let impact = thin[0]["impact"].as_str().unwrap_or_default();
        assert!(
            impact.contains("docs:identity-api"),
            "finding must name the thin lead, got: {impact}"
        );
        let severity = thin[0]["severity"].as_str().unwrap_or_default();
        assert_eq!(
            severity, "suggestion",
            "advisory finding must be `suggestion` severity so it never blocks"
        );
    }
}

mod validate_catalog {
    //! Component catalog contract — `slice validate` catalog drift gate.

    use super::support::*;

    /// Evidence with a `component:` directive on a claim.
    const EVIDENCE_WITH_COMPONENT: &str = "authority: behaviour
lead: my-slice
claims:
  - kind: region
    id: task-list-footer
    component: tab-bar
    statement: \"Bottom tab bar with three tabs.\"
";

    /// Evidence with `notes.candidate_component` (informational hint,
    /// not a hard `component:` directive).
    const EVIDENCE_WITH_CANDIDATE_COMPONENT: &str = "authority: behaviour
lead: my-slice
claims:
  - kind: region
    id: task-list-header
    notes:
      candidate_component: hero-banner
    statement: \"Hero banner at top of screen.\"
";

    /// A minimal catalog YAML with one confirmed and one rejected entry.
    const CATALOG_YAML: &str = "version: 1
components:
  tab-bar:
    status: confirmed
    description: \"Bottom navigation across the primary app sections.\"
  hero-banner:
    status: rejected
    description: \"Not a real shared component.\"
";

    /// Plan that declares a `ui-screens` source for the `my-slice` entry.
    const PLAN_WITH_UI_SCREENS: &str = "\
name: component-catalog
lifecycle: pending
sources:
  ui-screens:
    adapter: screenshots
    path: ./screens
slices:
  - name: my-slice
    status: pending
    sources:
      - { source: ui-screens, lead: my-slice }
";

    /// Stage a slice with Evidence containing `component:` directives
    /// and optionally a component catalog.
    fn stage_slice_with_catalog(
        evidence: &str, catalog: Option<&str>, plan: Option<&str>,
    ) -> Project {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();
        let slice_dir = project.slices_dir().join("my-slice");
        let evidence_dir = slice_dir.join("evidence");
        fs::create_dir_all(&evidence_dir).expect("mkdir evidence");
        fs::write(evidence_dir.join("ui-screens.yaml"), evidence).expect("write evidence");

        if let Some(cat) = catalog {
            let catalog_dir = project.root().join(".specify/design-system");
            fs::create_dir_all(&catalog_dir).expect("mkdir design-system");
            fs::write(catalog_dir.join("components.yaml"), cat).expect("write catalog");
        }

        if let Some(yaml) = plan {
            project.seed_plan(yaml);
        }
        project
    }

    #[test]
    fn skips_catalog_drift_no_catalog() {
        let project =
            stage_slice_with_catalog(EVIDENCE_WITH_COMPONENT, None, Some(PLAN_WITH_UI_SCREENS));
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert();
        assert_no_finding(assert.get_output(), "slice-catalog-drift");
    }

    #[test]
    fn validate_passes_when_slug_confirmed() {
        let project = stage_slice_with_catalog(
            EVIDENCE_WITH_COMPONENT,
            Some(CATALOG_YAML),
            Some(PLAN_WITH_UI_SCREENS),
        );
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert();
        assert_no_finding(assert.get_output(), "slice-catalog-drift");
    }

    #[test]
    fn validate_detects_missing_catalog_entry() {
        let catalog_without_tab_bar =
            "version: 1\ncomponents:\n  card-row:\n    status: confirmed\n";
        let project = stage_slice_with_catalog(
            EVIDENCE_WITH_COMPONENT,
            Some(catalog_without_tab_bar),
            Some(PLAN_WITH_UI_SCREENS),
        );
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        let detail = find_finding_impact(assert.get_output(), "slice-catalog-drift");
        assert!(
            detail.contains("tab-bar") && detail.contains("no entry exists"),
            "drift detail should name the missing slug, got: {detail}"
        );
    }

    #[test]
    fn validate_detects_rejected_catalog_entry() {
        let catalog_with_rejected = "version: 1\ncomponents:\n  tab-bar:\n    status: rejected\n";
        let project = stage_slice_with_catalog(
            EVIDENCE_WITH_COMPONENT,
            Some(catalog_with_rejected),
            Some(PLAN_WITH_UI_SCREENS),
        );
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        let detail = find_finding_impact(assert.get_output(), "slice-catalog-drift");
        assert!(
            detail.contains("tab-bar") && detail.contains("rejected"),
            "drift detail should describe the rejected status, got: {detail}"
        );
    }

    #[test]
    fn validate_ignores_candidate_notes() {
        let project = stage_slice_with_catalog(
            EVIDENCE_WITH_CANDIDATE_COMPONENT,
            Some(CATALOG_YAML),
            Some(PLAN_WITH_UI_SCREENS),
        );
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert();
        assert_no_finding(assert.get_output(), "slice-catalog-drift");
    }

    #[test]
    fn validate_passes_with_empty_catalog() {
        let empty_catalog = "version: 1\ncomponents: {}\n";
        let evidence_no_component = "authority: behaviour
lead: my-slice
claims:
  - kind: region
    id: task-list-body
    statement: \"Main task list body.\"
";
        let project = stage_slice_with_catalog(
            evidence_no_component,
            Some(empty_catalog),
            Some(PLAN_WITH_UI_SCREENS),
        );
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert();
        assert_no_finding(assert.get_output(), "slice-catalog-drift");
    }
}

mod validate_file_location {
    //! `slice validate` spec file-location gate.

    use super::support::*;

    #[test]
    fn root_spec_without_canonical() {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();
        let slice_dir = project.slices_dir().join("my-slice");
        fs::write(slice_dir.join("spec.md"), CLEAN_SPEC_MD).expect("write root spec.md");
        fs::remove_dir_all(slice_dir.join("specs"))
            .expect("remove specs dir created by slice create");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert()
            .failure();
        assert_eq!(assert.get_output().status.code(), Some(2));
        let value = parse_json(&assert.get_output().stderr);
        assert_eq!(value["error"], "slice-pre-adapter-gate");
        let detail = find_finding_impact(assert.get_output(), "specs.file-location");
        assert!(
            detail.contains("specs/<domain>/spec.md"),
            "detail must name the canonical layout, got: {detail}"
        );
        assert!(detail.contains("slice root"), "detail must mention the slice root, got: {detail}");
    }

    #[test]
    fn skipped_when_canonical_exists() {
        let project = stage_slice_with_spec(CLEAN_SPEC_MD, Some(PLAN_WITH_LEGACY_MONOLITH));
        let slice_dir = project.slices_dir().join("my-slice");
        fs::write(slice_dir.join("spec.md"), "stale root copy").expect("write root spec.md");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert();
        let stderr = assert.get_output().stderr.clone();
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&stderr)
            && let Some(results) = value["results"].as_array()
        {
            for r in results {
                let rule_id = r["rule-id"].as_str().unwrap_or("");
                assert_ne!(
                    rule_id, "specs.file-location",
                    "file-location gate must not fire when canonical specs exist"
                );
            }
        }
    }

    #[test]
    fn skipped_when_no_root_spec() {
        let project = Project::init();
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "create", "my-slice"])
            .assert()
            .success();
        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "slice", "validate", "my-slice"])
            .assert();
        let stderr = assert.get_output().stderr.clone();
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&stderr)
            && let Some(results) = value["results"].as_array()
        {
            for r in results {
                let rule_id = r["rule-id"].as_str().unwrap_or("");
                assert_ne!(
                    rule_id, "specs.file-location",
                    "file-location gate must not fire when no root spec.md exists"
                );
            }
        }
    }
}
