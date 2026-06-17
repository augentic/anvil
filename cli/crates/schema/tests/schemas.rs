//! Smoke tests that prove every embedded schema parses and compiles.
//! A corrupted `include_str!` import or a malformed schema source
//! surfaces here long before any production caller touches the
//! validator.

use jsonschema::{Registry, Resource};
use serde_json::{Value, json};
use specify_schema::{
    BUILD_REPORT_JSON_SCHEMA, BUILD_REQUEST_JSON_SCHEMA, DECISION_JSON_SCHEMA,
    DIAGNOSTIC_JSON_SCHEMA, DIAGNOSTIC_REPORT_JSON_SCHEMA, EMBEDDED_SCHEMAS, PARTS_JSON_SCHEMA,
    RESOLVED_RULES_JSON_SCHEMA, RULE_JSON_SCHEMA, SLICE_MODEL_JSON_SCHEMA, SYNTHESIS_JSON_SCHEMA,
    ValidationStatus, WORKSPACE_MODEL_JSON_SCHEMA, compile_schema, validate_value,
};

/// Every embedded schema compiles, table-driven over the canonical
/// [`EMBEDDED_SCHEMAS`] inventory so a new constant is covered the
/// moment it is registered there. The three envelope schemas whose
/// relative `$ref`s cannot resolve standalone compile through their
/// registry-backed validators instead. The two on-disk-only schemas
/// (`manifest-meta`, `context-lock`) are appended explicitly — they
/// ship without an embedded constant.
#[test]
fn every_schema_compiles() {
    let needs_registry =
        ["SYNTHESIS_JSON_SCHEMA", "DIAGNOSTIC_REPORT_JSON_SCHEMA", "BUILD_REPORT_JSON_SCHEMA"];
    for (name, relative, source) in EMBEDDED_SCHEMAS {
        if needs_registry.contains(name) {
            continue;
        }
        compile_schema(source).unwrap_or_else(|err| panic!("{name} ({relative}) compiles: {err}"));
    }
    let _synthesis = synthesis_validator();
    let _diagnostic_report = diagnostic_report_validator();
    let _build_report = build_report_validator();

    let on_disk_only = [
        ("manifest-meta", include_str!("../../../schemas/manifest-meta.schema.json")),
        ("context-lock", include_str!("../../../schemas/context-lock.schema.json")),
    ];
    for (name, source) in on_disk_only {
        compile_schema(source).unwrap_or_else(|err| panic!("{name} schema compiles: {err}"));
    }
}

/// Every embedded schema constant must byte-match its on-disk source.
/// `include_str!` binds at compile time, so this guards
/// against a constant pointing at a stale or duplicated copy: each
/// [`EMBEDDED_SCHEMAS`] entry re-reads the canonical workspace file at
/// runtime and asserts equality. The inventory itself lives in the
/// crate so `specify contract dump` publishes the same list.
#[test]
fn embedded_schemas_match_on_disk_sources() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crates/schema has a repo root two levels up");
    assert!(!EMBEDDED_SCHEMAS.is_empty(), "EMBEDDED_SCHEMAS must list every embedded constant");
    for (name, relative, embedded) in EMBEDDED_SCHEMAS {
        let on_disk = std::fs::read_to_string(repo_root.join(relative))
            .unwrap_or_else(|err| panic!("read {relative} for {name}: {err}"));
        assert_eq!(
            *embedded, on_disk,
            "{name} embed diverges from on-disk {relative}; re-run the embed or fix the file"
        );
    }
}

/// The single slice-model schema validates an agent synthesis response
/// `model` (kernel-owned + header fields omitted) — proving the
/// optional-field design that lets one schema serve both the response
/// and the persisted file.
#[test]
fn slice_model_accepts_no_kernel() {
    let instance = json!({
        "requirements": [{
            "title": "Request password reset",
            "agreement": "agreed",
            "claims": [
                { "source": "docs", "id": "password-reset.request", "kind": "requirement" }
            ],
            "statement": "The system lets a user request a reset link."
        }],
        "tasks": []
    });
    let summaries = validate_value(
        &instance,
        SLICE_MODEL_JSON_SCHEMA,
        "slice-model",
        "agent response model omits kernel-owned and header fields",
    );
    assert!(
        summaries.iter().all(|s| matches!(s.status, ValidationStatus::Pass)),
        "agent response without kernel fields must validate; got {summaries:?}"
    );
}

/// Compile the synthesis schema through a registry that pins the model
/// schema under its `$id`, so the response's relative `model` `$ref`
/// resolves (mirrors `lint_result_schema_compiles`).
fn synthesis_validator() -> jsonschema::Validator {
    let synthesis: Value =
        serde_json::from_str(SYNTHESIS_JSON_SCHEMA).expect("synthesis schema parses");
    let model: Value = serde_json::from_str(SLICE_MODEL_JSON_SCHEMA).expect("model schema parses");
    let registry = Registry::new()
        .add(
            "https://github.com/augentic/specify-cli/schemas/slice/model.schema.json",
            Resource::from_contents(model),
        )
        .and_then(jsonschema::RegistryBuilder::prepare)
        .expect("registry prepares");
    jsonschema::options()
        .with_registry(&registry)
        .build(&synthesis)
        .expect("synthesis schema compiles with model $ref resolved")
}

/// The synthesis-response worked example validates against the
/// synthesis schema with the model `$ref` resolved.
#[test]
fn synthesis_accepts_example() {
    let validator = synthesis_validator();
    let instance = json!({
        "kind": "response",
        "version": 1,
        "slice": "identity-service",
        "model": {
            "requirements": [{
                "title": "Request password reset",
                "domain": "password-reset",
                "agreement": "agreed",
                "claims": [
                    { "source": "docs", "id": "password-reset.request", "kind": "requirement" },
                    { "source": "legacy", "id": "users.password-reset.request", "kind": "example" }
                ],
                "statement": "The system lets a registered user request a password reset link by email.",
                "scenarios": [
                    "Given a registered email, when the user requests a reset, then the system accepts it."
                ]
            }],
            "tasks": [{
                "id": "TASK-001",
                "text": "Implement password reset request handling.",
                "satisfies": ["REQ-001"]
            }]
        },
        "artifacts": {
            "proposal": "# Password reset\n…",
            "design": "# Design\n…",
            "tasks": "# Tasks\n- [ ] TASK-001 …",
            "specs": [{
                "domain": "password-reset",
                "content": "## Request password reset\nThe system lets a registered user…"
            }]
        }
    });
    let errors: Vec<String> = validator.iter_errors(&instance).map(|err| err.to_string()).collect();
    assert!(errors.is_empty(), "RFC synthesis response must validate; errors: {errors:?}");
}

/// The `parts.yaml` worked example validates: a kebab-case
/// part slug carrying a composition `group` fragment (with a `*-when`
/// key and `items`) plus an optional description.
#[test]
fn parts_schema_accepts_rfc_example() {
    let instance = json!({
        "version": 1,
        "parts": {
            "tab-bar": {
                "description": "Bottom navigation across primary sections.",
                "group": {
                    "active-when": "$route",
                    "items": [
                        { "icon-button": { "bind": "home", "event": "Navigate(Home)" } },
                        { "icon-button": { "bind": "search", "event": "Navigate(Search)" } },
                        { "icon-button": { "bind": "settings", "event": "Navigate(Settings)" } }
                    ]
                }
            }
        }
    });
    let summaries =
        validate_value(&instance, PARTS_JSON_SCHEMA, "parts", "parts.yaml worked example");
    assert!(
        summaries.iter().all(|s| matches!(s.status, ValidationStatus::Pass)),
        "RFC parts example must validate; got {summaries:?}"
    );
}

/// A part missing its required `group`, and a non-kebab slug, are both
/// rejected.
#[test]
fn parts_schema_rejects_malformed() {
    let missing_group = json!({
        "version": 1,
        "parts": { "tab-bar": { "description": "no group" } }
    });
    let summaries = validate_value(
        &missing_group,
        PARTS_JSON_SCHEMA,
        "parts",
        "part requires a group fragment",
    );
    assert!(
        summaries.iter().any(|s| matches!(s.status, ValidationStatus::Fail)),
        "a part without `group` must be rejected"
    );

    let bad_slug = json!({
        "version": 1,
        "parts": { "TabBar": { "group": { "items": [{ "text": {} }] } } }
    });
    let summaries =
        validate_value(&bad_slug, PARTS_JSON_SCHEMA, "parts", "part slug is kebab-case");
    assert!(
        summaries.iter().any(|s| matches!(s.status, ValidationStatus::Fail)),
        "a non-kebab part slug must be rejected"
    );
}

/// The slice-authored Decision Record example validates without
/// the engine-stamped `id` / `slice` / `date` fields — proving the
/// optional-field design that lets one schema serve both the slice form
/// and the persisted baseline form.
#[test]
fn decision_accepts_slice_form() {
    let instance = json!({
        "slug": "identity-store-postgres",
        "status": "accepted",
        "supersedes": ["DEC-0003"],
        "related": ["REQ-001", "REQ-014"]
    });
    let summaries = validate_value(
        &instance,
        DECISION_JSON_SCHEMA,
        "decision",
        "slice-authored decision omits engine-stamped fields",
    );
    assert!(
        summaries.iter().all(|s| matches!(s.status, ValidationStatus::Pass)),
        "slice-authored decision must validate; got {summaries:?}"
    );
}

/// The promoted baseline Decision Record example validates with
/// the engine-stamped header fields present.
#[test]
fn decision_accepts_baseline_form() {
    let instance = json!({
        "id": "DEC-0007",
        "slug": "identity-store-postgres",
        "status": "accepted",
        "slice": "identity-service",
        "date": "2026-06-02",
        "supersedes": ["DEC-0003"],
        "related": ["REQ-001", "REQ-014"]
    });
    let summaries = validate_value(
        &instance,
        DECISION_JSON_SCHEMA,
        "decision",
        "promoted baseline decision carries engine-stamped fields",
    );
    assert!(
        summaries.iter().all(|s| matches!(s.status, ValidationStatus::Pass)),
        "promoted baseline decision must validate; got {summaries:?}"
    );
}

#[test]
fn workspace_model_accepts_minimal() {
    let instance = json!({
        "version": 1,
        "project_dir": ".",
        "scan_profile": "project",
        "artifact_paths": [],
        "languages": [],
        "files": [],
        "frontmatter": [],
        "markdown_sections": [],
        "markdown_links": [],
        "symlinks": [],
        "skills": [],
        "adapter_manifests": []
    });
    let summaries = validate_value(
        &instance,
        WORKSPACE_MODEL_JSON_SCHEMA,
        "workspace-model",
        "workspace-model minimal envelope",
    );
    assert!(
        summaries.iter().all(|s| matches!(s.status, ValidationStatus::Pass)),
        "minimal envelope must validate; got {summaries:?}"
    );
}

/// Compile the diagnostic-report envelope through a registry that pins
/// the finding schema under the directory its relative
/// `diagnostic.schema.json` `$ref` resolves to — the standalone
/// `compile_schema` helper has no resource registry.
fn diagnostic_report_validator() -> jsonschema::Validator {
    let envelope: Value =
        serde_json::from_str(DIAGNOSTIC_REPORT_JSON_SCHEMA).expect("lint-result schema parses");
    let finding: Value =
        serde_json::from_str(DIAGNOSTIC_JSON_SCHEMA).expect("finding schema parses");
    let registry = Registry::new()
        .add(
            "https://github.com/augentic/specify-cli/schemas/diagnostics/diagnostic.schema.json",
            Resource::from_contents(finding),
        )
        .and_then(jsonschema::RegistryBuilder::prepare)
        .expect("registry prepares");
    jsonschema::options()
        .with_registry(&registry)
        .build(&envelope)
        .expect("lint-result schema compiles with finding $ref resolved")
}

#[test]
fn lint_result_accepts_one_finding() {
    let validator = diagnostic_report_validator();
    let instance = json!({
        "version": 1,
        "summary": { "critical": 0, "important": 1, "suggestion": 0, "optional": 0 },
        "findings": [{
            "id": "FIND-0001",
            "rule-id": "UNI-014",
            "title": "Literal deployment URL in generated handler",
            "severity": "important",
            "source": "hybrid",
            "target-adapter": "omnia",
            "slice": "billing-invoice-export",
            "artifact": "code",
            "location": {
                "path": "crates/invoice_export/src/config.rs",
                "line": 18
            },
            "evidence": {
                "kind": "snippet",
                "value": "const BASE_URL: &str = \"https://api.example.com\";"
            },
            "impact": "Generated code will point every deployment at the same external endpoint.",
            "remediation": "Read the endpoint from Omnia configuration and add a required config key to the design.",
            "confidence": "high",
            "fingerprint": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
        }]
    });
    let errors: Vec<String> = validator.iter_errors(&instance).map(|err| err.to_string()).collect();
    assert!(errors.is_empty(), "FIND-0001 envelope must validate; errors: {errors:?}");
}

/// Pins the relative-ref form: integrations downstream rely on
/// `diagnostic.schema.json` resolving against the envelope schema's
/// directory rather than an absolute URL.
#[test]
fn diagnostic_report_relative_ref() {
    let envelope: Value = serde_json::from_str(DIAGNOSTIC_REPORT_JSON_SCHEMA)
        .expect("diagnostic-report schema parses");
    let items_ref = envelope
        .pointer("/properties/findings/items/$ref")
        .and_then(Value::as_str)
        .expect("findings.items.$ref is a string");
    assert_eq!(items_ref, "diagnostic.schema.json");
}

/// The build-request worked example validates.
#[test]
fn build_request_schema_accepts_rfc_example() {
    let instance = json!({
        "version": 1,
        "slice": "identity-service",
        "project-dir": "/workspace/workspace/identity-service",
        "inputs": {
            "root": "/workspace/.specify/slices/identity-service",
            "artifacts": {
                "proposal": "proposal.md",
                "specs": ["specs/identity/spec.md"],
                "design": "design.md",
                "tasks": "tasks.md",
                "additional": ["tokens.yaml"]
            }
        }
    });
    let summaries = validate_value(
        &instance,
        BUILD_REQUEST_JSON_SCHEMA,
        "build-request",
        "build request example",
    );
    assert!(
        summaries.iter().all(|s| matches!(s.status, ValidationStatus::Pass)),
        "RFC build request must validate; got {summaries:?}"
    );
}

/// Compile the build-report schema through a registry that pins the
/// diagnostic schema under its `$id`, so the report's relative
/// `../diagnostics/diagnostic.schema.json` `findings[]` `$ref` resolves
/// (mirrors `lint_result_schema_compiles`).
fn build_report_validator() -> jsonschema::Validator {
    let report: Value =
        serde_json::from_str(BUILD_REPORT_JSON_SCHEMA).expect("build-report schema parses");
    let diagnostic: Value =
        serde_json::from_str(DIAGNOSTIC_JSON_SCHEMA).expect("diagnostic schema parses");
    let registry = Registry::new()
        .add(
            "https://github.com/augentic/specify-cli/schemas/diagnostics/diagnostic.schema.json",
            Resource::from_contents(diagnostic),
        )
        .and_then(jsonschema::RegistryBuilder::prepare)
        .expect("registry prepares");
    jsonschema::options()
        .with_registry(&registry)
        .build(&report)
        .expect("build-report schema compiles with diagnostic $ref resolved")
}

/// The build-report success example validates.
#[test]
fn build_report_schema_accepts_success() {
    let validator = build_report_validator();
    let instance = json!({
        "version": 1,
        "slice": "identity-service",
        "target": "omnia@1.0.0",
        "status": "success",
        "findings": []
    });
    let errors: Vec<String> = validator.iter_errors(&instance).map(|err| err.to_string()).collect();
    assert!(errors.is_empty(), "success report must validate; errors: {errors:?}");
}

/// The build-report failure (no findings) example validates.
#[test]
fn build_report_failure_no_findings() {
    let validator = build_report_validator();
    let instance = json!({
        "version": 1,
        "slice": "identity-service",
        "target": "omnia@1.0.0",
        "status": "failure",
        "findings": []
    });
    let errors: Vec<String> = validator.iter_errors(&instance).map(|err| err.to_string()).collect();
    assert!(errors.is_empty(), "failure report must validate; errors: {errors:?}");
}

/// A success report with `outputs[]` validates, proving the new
/// `buildOutput` `$def` and `platform` enum resolve correctly.
#[test]
fn build_report_success_outputs() {
    let validator = build_report_validator();
    let instance = json!({
        "version": 1,
        "slice": "identity-service",
        "target": "vectis@1.0.0",
        "status": "success",
        "findings": [],
        "outputs": [
            { "platform": "core", "path": "shared/src/app.rs" },
            { "platform": "ios", "path": "iOS/MyApp/ContentView.swift" },
            { "platform": "android", "path": "Android/app/src/main/kotlin/Main.kt" }
        ]
    });
    let errors: Vec<String> = validator.iter_errors(&instance).map(|err| err.to_string()).collect();
    assert!(errors.is_empty(), "success report with outputs must validate; errors: {errors:?}");
}

/// A report without `outputs` validates (backward compatibility).
#[test]
fn build_report_no_outputs() {
    let validator = build_report_validator();
    let instance = json!({
        "version": 1,
        "slice": "identity-service",
        "target": "omnia@1.0.0",
        "status": "success",
        "findings": []
    });
    let errors: Vec<String> = validator.iter_errors(&instance).map(|err| err.to_string()).collect();
    assert!(errors.is_empty(), "report without outputs must validate; errors: {errors:?}");
}

/// A success report carrying the optional `ui-surface` field validates,
/// proving the additive A4 signal resolves against the `uiSurface`
/// `$def`.
#[test]
fn build_report_accepts_ui_surface() {
    let validator = build_report_validator();
    let instance = json!({
        "version": 1,
        "slice": "identity-service",
        "target": "vectis@1.0.0",
        "status": "success",
        "findings": [],
        "ui-surface": { "screens": 3 }
    });
    let errors: Vec<String> = validator.iter_errors(&instance).map(|err| err.to_string()).collect();
    assert!(errors.is_empty(), "report with ui-surface must validate; errors: {errors:?}");
}

/// `ui-surface.screens` is required and must be a non-negative integer;
/// a stray sibling key is rejected by `additionalProperties: false`.
#[test]
fn build_report_rejects_bad_ui_surface() {
    let validator = build_report_validator();
    let missing_screens = json!({
        "version": 1,
        "slice": "identity-service",
        "target": "vectis@1.0.0",
        "status": "success",
        "findings": [],
        "ui-surface": {}
    });
    assert!(
        validator.iter_errors(&missing_screens).next().is_some(),
        "ui-surface without screens must be rejected"
    );
    let stray_key = json!({
        "version": 1,
        "slice": "identity-service",
        "target": "vectis@1.0.0",
        "status": "success",
        "findings": [],
        "ui-surface": { "screens": 1, "stray": true }
    });
    assert!(
        validator.iter_errors(&stray_key).next().is_some(),
        "ui-surface with a stray key must be rejected"
    );
}

/// The build-report failure-with-findings example validates, proving
/// the relative diagnostic `$ref` accepts a full finding.
#[test]
fn build_report_failure_with_findings() {
    let validator = build_report_validator();
    let instance = json!({
        "version": 1,
        "slice": "identity-contracts",
        "target": "contracts@1.0.0",
        "status": "failure",
        "findings": [{
            "id": "DIAG-0001",
            "rule-id": "contract.id-unique",
            "title": "Duplicate info.x-specify-id across baseline",
            "severity": "critical",
            "source": "tool",
            "kind": "violation",
            "target-adapter": "contracts",
            "slice": "identity-contracts",
            "artifact": "contracts",
            "location": {
                "path": "contracts/http/user-api.yaml"
            },
            "evidence": {
                "kind": "structured",
                "summary": "x-specify-id user-api collides with legacy-api.yaml",
                "data": {
                    "detail": "info.x-specify-id user-api also present on contracts/http/legacy-api.yaml"
                }
            },
            "impact": "Downstream consumers cannot resolve a unique contract id.",
            "remediation": "Rename or remove the duplicate id before merge.",
            "fingerprint": "sha256:a2e95674f838eb042eba78e16239f32199def3ca976e29499f8275beb30225e4"
        }]
    });
    let errors: Vec<String> = validator.iter_errors(&instance).map(|err| err.to_string()).collect();
    assert!(errors.is_empty(), "failure-with-findings report must validate; errors: {errors:?}");
}

/// Per the standards-layer contract §"Hint kinds — reserved", reserved kinds are
/// shape-validated by this schema with no execution semantics. A
/// minimal codex-rule frontmatter that declares each reserved kind
/// must round-trip cleanly so rules exporters accept files awaiting
/// implementation.
#[test]
fn codex_rule_accepts_reserved_kinds() {
    let reserved = ["unique", "reference-resolves", "set-coverage", "cardinality", "constant-eq"];
    for kind in reserved {
        let instance = json!({
            "id": "UNI-014",
            "title": "Reserved-kind smoke fixture",
            "severity": "important",
            "trigger": "Reserved hint kind smoke fixture.",
            "rule_hints": [{
                "kind": kind,
                "value": "placeholder"
            }]
        });
        let summaries =
            validate_value(&instance, RULE_JSON_SCHEMA, "rule", "rule fixture per reserved kind");
        assert!(
            summaries.iter().all(|s| matches!(s.status, ValidationStatus::Pass)),
            "kind {kind} must validate; got {summaries:?}"
        );
    }
}

/// The `UNI-014` example for the `ResolvedRules` export validates
/// cleanly against the resolved-codex schema.
#[test]
fn resolved_codex_accepts_example() {
    let instance = json!({
        "version": 1,
        "target-adapter": "omnia",
        "source-adapters": ["typescript"],
        "rules": [
            {
                "rule-id": "UNI-014",
                "title": "Hardcoded Configuration",
                "severity": "important",
                "trigger": "Generated code embeds environment-specific configuration instead of routing it through declared configuration.",
                "lint-mode": "hybrid",
                "origin": "shared",
                "path-root": "rules-root",
                "path": "adapters/shared/rules/universal/hardcoded-configuration.md",
                "applicability": {
                    "adapters": ["omnia"],
                    "languages": ["rust"],
                    "artifacts": ["code"]
                },
                "rule-hints": [
                    {
                        "kind": "regex",
                        "value": "https?://",
                        "description": "Literal URL in generated code."
                    }
                ],
                "references": [
                    {
                        "label": "Omnia guardrails",
                        "path": "adapters/targets/omnia/references/guardrails.md"
                    }
                ],
                "body": "## Rule\n\nConfiguration values that vary between deployments must not be hardcoded in generated code.\n",
                "deprecated": null
            }
        ]
    });
    let validator =
        compile_schema(RESOLVED_RULES_JSON_SCHEMA).expect("resolved codex schema compiles");
    let errors: Vec<String> = validator.iter_errors(&instance).map(|e| e.to_string()).collect();
    assert!(errors.is_empty(), "UNI-014 example must validate; errors: {errors:?}");
}

/// The `FIND-0001` example for structured lint findings validates
/// cleanly against the standalone diagnostic schema. The fingerprint
/// placeholder `sha256:...` from the contract is replaced with a
/// deterministic 64-hex-char digest so the fingerprint pattern check
/// passes.
#[test]
fn review_finding_accepts_example() {
    let instance = json!({
        "id": "FIND-0001",
        "rule-id": "UNI-014",
        "title": "Literal deployment URL in generated handler",
        "severity": "important",
        "source": "hybrid",
        "target-adapter": "omnia",
        "slice": "billing-invoice-export",
        "artifact": "code",
        "location": {
            "path": "crates/invoice_export/src/config.rs",
            "line": 18
        },
        "evidence": {
            "kind": "snippet",
            "value": "const BASE_URL: &str = \"https://api.example.com\";"
        },
        "impact": "Generated code will point every deployment at the same external endpoint.",
        "remediation": "Read the endpoint from Omnia configuration and add a required config key to the design.",
        "confidence": "high",
        "fingerprint": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
    });
    let validator = compile_schema(DIAGNOSTIC_JSON_SCHEMA).expect("review finding schema compiles");
    let errors: Vec<String> = validator.iter_errors(&instance).map(|e| e.to_string()).collect();
    assert!(errors.is_empty(), "FIND-0001 example must validate; errors: {errors:?}");
}
