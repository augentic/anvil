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
    let project = Project::init().with_schemas();
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
