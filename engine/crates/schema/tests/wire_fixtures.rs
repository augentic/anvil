//! Accept / reject fixture tests for the workflow wire schemas:
//! `adapter`, `source`, `target`, `evidence`, `discovery/lead`, and
//! `plan/plan`. Each schema gets a "valid" fixture that must validate
//! cleanly plus a small set of "invalid" fixtures (missing required
//! field, wrong enum value, wrong type) that the schema must reject.
//!
//! This crate is the single compile/parity/fixture home for the
//! embedded schemas; workflow keeps only its behavior edges (wrapper
//! error codes).

use std::path::PathBuf;

use jsonschema::Validator;
use serde_json::Value as JsonValue;
use specify_schema::{
    ADAPTER_JSON_SCHEMA, EVIDENCE_JSON_SCHEMA, LEAD_JSON_SCHEMA, PLAN_JSON_SCHEMA,
    SOURCE_JSON_SCHEMA, TARGET_JSON_SCHEMA, compile_schema,
};

fn load(source: &str) -> Validator {
    compile_schema(source).expect("embedded schema compiles")
}

fn yaml(input: &str) -> JsonValue {
    serde_saphyr::from_str(input).expect("fixture parses as YAML")
}

fn assert_valid(validator: &Validator, instance: &JsonValue, ctx: &str) {
    let errors: Vec<String> =
        validator.iter_errors(instance).map(|e| format!("{}: {e}", e.instance_path())).collect();
    assert!(errors.is_empty(), "{ctx}: should validate cleanly; got {errors:#?}");
}

fn assert_invalid(validator: &Validator, instance: &JsonValue, ctx: &str) {
    let count = validator.iter_errors(instance).count();
    assert!(count > 0, "{ctx}: schema should reject the fixture but did not");
}

// --- adapter.schema.json --------------------------------------------

// The post-cutover manifest is prose identity only: `name` / `version`
// / `axis` / `description`, plus target-only `platforms` / `inputs` and
// the optional `specify` host-CLI floor. Operation sets derive from the
// closed WIT contract, never from the wire.
const PLUGIN_VALID_SOURCE: &str = r"
name: typescript
version: 1.0.0
axis: source
description: Extracts behavioural evidence from TypeScript codebases.
";

const PLUGIN_VALID_TARGET: &str = r"
name: omnia
version: 1.0.0
axis: target
description: Omnia Rust WASM target adapter.
";

const PLUGIN_VALID_TARGET_WITH_PLATFORMS: &str = r"
name: vectis
version: 1.0.0
axis: target
description: Target manifest carrying the declarative platforms capability.
platforms:
  required: true
  allowed: [core, ios, android]
  default: [core, ios, android]
";

const PLUGIN_VALID_TARGET_WITH_INPUTS: &str = r"
name: vectis
version: 1.0.0
axis: target
description: Target manifest declaring build inputs.
inputs:
  - path: tokens.yaml
    required: true
";

const PLUGIN_INVALID_NO_AXIS: &str = r"
name: typescript
version: 1.0.0
description: Missing the axis discriminator.
";

const PLUGIN_INVALID_BAD_AXIS: &str = r"
name: typescript
version: 1.0.0
axis: lens
description: Unknown axis value.
";

const PLUGIN_INVALID_NAME_NOT_KEBAB: &str = r"
name: CodeTypeScript
version: 1.0.0
axis: source
description: Name is not kebab-case.
";

const PLUGIN_INVALID_VERSION_NOT_SEMVER: &str = r"
name: typescript
version: '1'
axis: source
description: Version is not semver.
";

const PLUGIN_INVALID_MISSING_DESCRIPTION: &str = r"
name: typescript
version: 1.0.0
axis: source
";

#[test]
fn plugin_accepts_source_and_target_shapes() {
    let v = load(ADAPTER_JSON_SCHEMA);
    assert_valid(&v, &yaml(PLUGIN_VALID_SOURCE), "plugin/source");
    assert_valid(&v, &yaml(PLUGIN_VALID_TARGET), "plugin/target");
    assert_valid(&v, &yaml(PLUGIN_VALID_TARGET_WITH_PLATFORMS), "plugin/target-with-platforms");
    assert_valid(&v, &yaml(PLUGIN_VALID_TARGET_WITH_INPUTS), "plugin/target-with-inputs");
}

#[test]
fn plugin_rejects_axis_and_primitives() {
    let v = load(ADAPTER_JSON_SCHEMA);
    assert_invalid(&v, &yaml(PLUGIN_INVALID_NO_AXIS), "plugin/no-axis");
    assert_invalid(&v, &yaml(PLUGIN_INVALID_BAD_AXIS), "plugin/bad-axis");
    assert_invalid(&v, &yaml(PLUGIN_INVALID_NAME_NOT_KEBAB), "plugin/name-not-kebab");
    assert_invalid(&v, &yaml(PLUGIN_INVALID_VERSION_NOT_SEMVER), "plugin/version-not-semver");
    assert_invalid(&v, &yaml(PLUGIN_INVALID_MISSING_DESCRIPTION), "plugin/missing-description");
}

#[test]
fn plugin_rejects_retired_old_stack_keys() {
    // The old-stack manifest surface (`briefs`, `execution`,
    // `extension`, `prepare`, `host_prereq`, `finalize_verify`,
    // `catalog`) is closed out; `additionalProperties: false` rejects
    // every retired key.
    let v = load(ADAPTER_JSON_SCHEMA);
    for retired in [
        "briefs:\n  survey: briefs/survey.md\n  extract: briefs/extract.md",
        "execution: agent",
        "extension:\n  name: demo-extension",
        "prepare:\n  argv: [prepare, build]",
        "host_prereq:\n  script: scripts/host-prereq.sh",
        "finalize_verify:\n  script: scripts/finalize-verify.sh",
        "catalog:\n  infer: true",
    ] {
        let manifest = format!(
            "name: demo\nversion: 1.0.0\naxis: target\ndescription: Retired key.\n{retired}\n"
        );
        assert_invalid(&v, &yaml(&manifest), &format!("plugin/retired-key `{retired}`"));
    }
}

// --- source.schema.json --------------------------------------------

const SOURCE_INVALID_AXIS_TARGET: &str = r"
name: typescript
version: 1.0.0
axis: target
description: Wrong axis for the source schema.
";

const SOURCE_INVALID_PLATFORMS: &str = r"
name: typescript
version: 1.0.0
axis: source
description: Source adapter carrying the target-only platforms capability.
platforms:
  required: true
  allowed: [core]
  default: [core]
";

#[test]
fn source_accepts_canonical_shape() {
    let v = load(SOURCE_JSON_SCHEMA);
    assert_valid(&v, &yaml(PLUGIN_VALID_SOURCE), "source/valid");
}

#[test]
fn source_rejects_axis_and_target_only_keys() {
    let v = load(SOURCE_JSON_SCHEMA);
    assert_invalid(&v, &yaml(SOURCE_INVALID_AXIS_TARGET), "source/axis-target");
    assert_invalid(&v, &yaml(SOURCE_INVALID_PLATFORMS), "source/platforms");
}

// --- target.schema.json --------------------------------------------

const TARGET_INVALID_AXIS_SOURCE: &str = r"
name: omnia
version: 1.0.0
axis: source
description: Wrong axis for the target schema.
";

#[test]
fn target_accepts_canonical_shape() {
    let v = load(TARGET_JSON_SCHEMA);
    assert_valid(&v, &yaml(PLUGIN_VALID_TARGET), "target/valid");
    assert_valid(&v, &yaml(PLUGIN_VALID_TARGET_WITH_PLATFORMS), "target/platforms");
    assert_valid(&v, &yaml(PLUGIN_VALID_TARGET_WITH_INPUTS), "target/inputs");
}

#[test]
fn target_rejects_axis_mismatch() {
    let v = load(TARGET_JSON_SCHEMA);
    assert_invalid(&v, &yaml(TARGET_INVALID_AXIS_SOURCE), "target/axis-source");
}

// --- evidence.schema.json ------------------------------------------

const EVIDENCE_VALID_REQUIREMENT: &str = r"
authority: behaviour
lead: user-registration
claims:
  - kind: requirement
    id: users.register.email-validation
    path: src/users/register.ts#L12-L87
    statement: The system accepts registrations with RFC 5322 emails.
";

const EVIDENCE_VALID_SPATIAL: &str = r"
authority: documentation
lead: home-screen
claims:
  - kind: region
    path: screenshots/home.png
  - kind: container
    path: screenshots/home.png
  - kind: leaf
    path: screenshots/home.png
";

const EVIDENCE_VALID_EMPTY_CLAIMS: &str = r"
authority: intent
lead: add-search-filter
claims: []
";

const EVIDENCE_INVALID_MISSING_AUTHORITY: &str = r"
lead: user-registration
claims: []
";

const EVIDENCE_INVALID_BAD_AUTHORITY: &str = r"
authority: unknown
lead: user-registration
claims: []
";

const EVIDENCE_INVALID_BAD_KIND: &str = r"
authority: behaviour
lead: user-registration
claims:
  - kind: hunch
    id: users.register.maybe
";

const EVIDENCE_INVALID_REQUIREMENT_NO_CLAIM_ID: &str = r"
authority: documentation
lead: password-reset
claims:
  - kind: requirement
    statement: Reset links expire after 30 minutes.
";

const EVIDENCE_INVALID_LEAD_NOT_KEBAB: &str = r"
authority: behaviour
lead: User_Registration
claims: []
";

#[test]
fn evidence_accepts_doc_legacy_and_spatial() {
    let v = load(EVIDENCE_JSON_SCHEMA);
    assert_valid(&v, &yaml(EVIDENCE_VALID_REQUIREMENT), "evidence/requirement");
    assert_valid(&v, &yaml(EVIDENCE_VALID_SPATIAL), "evidence/spatial-region-container-leaf");
    assert_valid(&v, &yaml(EVIDENCE_VALID_EMPTY_CLAIMS), "evidence/empty-claims");
}

#[test]
fn evidence_rejects_bad_authority_and_kinds() {
    let v = load(EVIDENCE_JSON_SCHEMA);
    assert_invalid(&v, &yaml(EVIDENCE_INVALID_MISSING_AUTHORITY), "evidence/missing-authority");
    assert_invalid(&v, &yaml(EVIDENCE_INVALID_BAD_AUTHORITY), "evidence/bad-authority");
    assert_invalid(&v, &yaml(EVIDENCE_INVALID_BAD_KIND), "evidence/bad-kind");
    assert_invalid(
        &v,
        &yaml(EVIDENCE_INVALID_REQUIREMENT_NO_CLAIM_ID),
        "evidence/requirement-missing-id",
    );
    assert_invalid(&v, &yaml(EVIDENCE_INVALID_LEAD_NOT_KEBAB), "evidence/lead-not-kebab");
}

// --- discovery/lead.schema.json --------------------------------

const LEAD_VALID: &str = r"
lead: user-registration
source: legacy-monolith
synopsis: Registration endpoint accepting email + password with RFC 5322 validation.
";

const LEAD_INVALID_MISSING_SOURCE_KEY: &str = r"
lead: user-registration
synopsis: bad — source is required.
";

const LEAD_INVALID_BAD_ID: &str = r"
lead: User_Registration
source: legacy-monolith
synopsis: Bad id.
";

const LEAD_INVALID_TENTATIVE_REMOVED: &str = r"
lead: user-registration
source: legacy-monolith
synopsis: A lead carrying the retired tentative field.
tentative: true
";

const LEAD_INVALID_ALIASES_REMOVED: &str = r"
lead: user-registration
source: legacy-monolith
synopsis: A lead carrying the retired aliases field.
aliases:
  - account-registration
";

#[test]
fn lead_accepts_minimal_shape() {
    let v = load(LEAD_JSON_SCHEMA);
    assert_valid(&v, &yaml(LEAD_VALID), "lead/minimal");
}

#[test]
fn lead_rejects_source_id_tentative() {
    let v = load(LEAD_JSON_SCHEMA);
    assert_invalid(&v, &yaml(LEAD_INVALID_MISSING_SOURCE_KEY), "lead/missing-source");
    assert_invalid(&v, &yaml(LEAD_INVALID_BAD_ID), "lead/bad-id");
    // `tentative` is not a lead field (DECISIONS §Lead reconciliation D2.3); the schema
    // is `additionalProperties: false`, so a lead carrying it fails.
    assert_invalid(&v, &yaml(LEAD_INVALID_TENTATIVE_REMOVED), "lead/retired-tentative");
    // `aliases` is not a lead field; the schema is `additionalProperties: false`, so a
    // lead carrying it fails.
    assert_invalid(&v, &yaml(LEAD_INVALID_ALIASES_REMOVED), "lead/retired-aliases");
}

// --- plan/plan.schema.json -------------------------------------------

fn plan_v2_fixture_path(name: &str) -> PathBuf {
    // `crates/schema/` -> `crates/` -> repo root -> `tests/fixtures/plan/v2/`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/fixtures/plan/v2")
        .join(name)
}

#[test]
fn plan_schema_accepts_workflow_intent_n1() {
    let v = load(PLAN_JSON_SCHEMA);
    let raw = std::fs::read_to_string(plan_v2_fixture_path("intent-n1.yaml")).expect("read");
    assert_valid(&v, &yaml(&raw), "plan/v2/intent-n1");
}

#[test]
fn plan_accepts_multi_source() {
    let v = load(PLAN_JSON_SCHEMA);
    let raw = std::fs::read_to_string(plan_v2_fixture_path("multi-source.yaml")).expect("read");
    assert_valid(&v, &yaml(&raw), "plan/v2/multi-source");
}

#[test]
fn plan_accepts_divergence_likely() {
    let v = load(PLAN_JSON_SCHEMA);
    let raw =
        std::fs::read_to_string(plan_v2_fixture_path("divergence-likely.yaml")).expect("read");
    assert_valid(&v, &yaml(&raw), "plan/v2/divergence-likely");
}

#[test]
fn plan_rejects_unknown_divergence() {
    let v = load(PLAN_JSON_SCHEMA);
    let raw =
        std::fs::read_to_string(plan_v2_fixture_path("divergence-likely.yaml")).expect("read");
    let mutated = raw.replace("divergence: likely", "divergence: maybe");
    assert_invalid(&v, &yaml(&mutated), "plan/v2/divergence-bad-value");
}

#[test]
fn plan_rejects_slice_missing_lead() {
    let v = load(PLAN_JSON_SCHEMA);
    let bad = r"
name: bad
slices:
  - name: only
    project: app
    sources:
      - source: docs
    status: pending
";
    assert_invalid(&v, &yaml(bad), "plan/v2/source-missing-lead");
}
