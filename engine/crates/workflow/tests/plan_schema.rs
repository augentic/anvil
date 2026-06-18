//! Pure-library tests for `schemas/plan/plan.schema.json` (embedded as
//! `specify_schema::PLAN_JSON_SCHEMA`) plus the kebab-name regex shared
//! with `specify_workflow::slice::actions::validate_name`. CLI
//! integration tests for the `specify plan *` group live in the binary
//! harness under `tests/workflow/`.

use jsonschema::Validator;
use serde_json::Value as JsonValue;
use specify_schema::PLAN_JSON_SCHEMA;

/// `platform-v2` plan example, inline.
///
/// Kept inline (rather than loaded from a fixture) so the test is pinned to
/// the exact shape the schema ships; subsequent changes that touch the schema must
/// also touch this constant.
const PLAN_EXAMPLE: &str = r"
name: platform-v2

sources:
  monolith:
    adapter: typescript
    path: /path/to/legacy-codebase
  orders:
    adapter: typescript
    path: git@github.com:org/orders-service.git
  payments:
    adapter: typescript
    path: git@github.com:org/payments-service.git
  frontend:
    adapter: typescript
    path: git@github.com:org/web-app.git

slices:
  - name: user-registration
    project: platform
    sources: [monolith]
    status: done

  - name: email-verification
    project: platform
    sources: [monolith]
    depends-on: [user-registration]
    status: in-progress

  - name: registration-duplicate-email-crash
    project: platform
    description: >
      Duplicate email submission returns 500 instead of 409.
      Discovered during email-verification extraction.
    status: pending

  - name: notification-preferences
    project: platform
    depends-on: [user-registration]
    description: >
      Greenfield — user-facing notification channel and frequency settings.
    status: pending

  - name: extract-shared-validation
    project: platform
    description: >
      Pull duplicated input validation into a shared validation crate
      before building checkout-flow.
    depends-on: [email-verification]
    status: pending

  - name: product-catalog
    project: platform
    sources: [monolith]
    depends-on: [extract-shared-validation]
    status: pending

  - name: shopping-cart
    project: platform
    sources: [orders]
    depends-on: [product-catalog, user-registration]
    status: pending

  - name: checkout-api
    project: platform
    sources: [payments]
    depends-on: [shopping-cart]
    status: pending

  - name: checkout-ui
    project: platform
    sources: [frontend]
    depends-on: [checkout-api]
    status: pending
";

fn load_validator() -> Validator {
    let schema: JsonValue =
        serde_json::from_str(PLAN_JSON_SCHEMA).expect("plan.schema.json is valid JSON");
    jsonschema::validator_for(&schema).expect("plan.schema.json compiles as a JSON Schema")
}

fn yaml_to_json(yaml: &str) -> JsonValue {
    serde_saphyr::from_str(yaml).expect("fixture parses as YAML")
}

#[test]
fn schema_validates_plan_example() {
    let validator = load_validator();
    let instance = yaml_to_json(PLAN_EXAMPLE);
    let errors: Vec<String> =
        validator.iter_errors(&instance).map(|e| format!("{}: {}", e.instance_path(), e)).collect();
    assert!(errors.is_empty(), "plan example should validate cleanly; errors: {errors:#?}");
}

#[test]
fn schema_rejects_unknown_status() {
    let validator = load_validator();
    let mutated = PLAN_EXAMPLE.replacen("status: in-progress", "status: maybe", 1);
    let instance = yaml_to_json(&mutated);

    let offending_paths: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| e.instance_path().to_string())
        .filter(|p| p.starts_with("/slices/") && p.ends_with("/status"))
        .collect();

    assert!(
        !offending_paths.is_empty(),
        "unknown status should produce at least one error on /slices/*/status; got none"
    );
}

#[test]
fn schema_rejects_non_kebab_name() {
    let validator = load_validator();
    let mutated = PLAN_EXAMPLE.replacen("name: platform-v2", "name: Platform V2", 1);
    let instance = yaml_to_json(&mutated);

    let name_errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| e.instance_path().to_string())
        .filter(|p| p == "/name")
        .collect();

    assert!(
        !name_errors.is_empty(),
        "non-kebab-case name should produce at least one error on /name; got none"
    );
}

/// The JSON Schema regex and `validate_name` must agree on every name,
/// in both directions. The cases below are the plan schema alias cases
/// out; keep them in sync with the doc-comment on
/// `specify_workflow::slice::actions::validate_name`.
#[test]
fn kebab_name_regex_matches_validate_name() {
    use regex::Regex;
    use specify_workflow::slice::actions as slice_actions;

    // Extract the pattern from the embedded schema to keep this test
    // honest against drift — the schema is the source of truth.
    let schema: JsonValue =
        serde_json::from_str(PLAN_JSON_SCHEMA).expect("plan.schema.json is valid JSON");
    let pattern = schema["$defs"]["kebabName"]["pattern"]
        .as_str()
        .expect("$defs.kebabName.pattern is a string");
    let regex = Regex::new(pattern).expect("$defs.kebabName.pattern compiles as a regex");

    let accept = ["a", "ab", "a-b", "a1", "user-registration"];
    let reject = ["", "-a", "a-", "a--b", "A", "a_b"];

    for name in accept {
        assert!(regex.is_match(name), "regex `{pattern}` should accept `{name}` but did not");
        slice_actions::validate_name(name).unwrap_or_else(|err| {
            panic!("validate_name should accept `{name}`, got error: {err}");
        });
    }

    for name in reject {
        assert!(!regex.is_match(name), "regex `{pattern}` should reject `{name}` but accepted it");
        slice_actions::validate_name(name)
            .err()
            .unwrap_or_else(|| panic!("validate_name should reject `{name}`"));
    }
}
