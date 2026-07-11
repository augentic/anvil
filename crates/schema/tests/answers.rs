//! Parity and behavior tests for the derived judgment-answer schemas.
//!
//! The generated documents under `schemas/answers/` must byte-match the
//! `answers` derivation (regenerate via
//! `REGENERATE_GOLDENS=1 cargo nextest run -p schema`), every
//! derived document must compile standalone (the derivation inlines
//! cross-file refs), and worked answer examples must validate — proving
//! the envelope fields are gone and the canonical constraints survive.

use schema::{ValidationStatus, answers, compile_schema, validate_value};
use serde_json::{Value, json};

fn schemas_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/schema has the repo root two levels up")
        .join("schemas/answers")
}

// Compare a derived answer schema against its generated on-disk copy, or
// rewrite the copy when `REGENERATE_GOLDENS` is set. `include_str!` binds
// at compile time, so a regeneration needs one rebuild before the
// embedded-constant byte-match in `schemas.rs` sees the new content.
fn assert_generated(name: &str, derived: &Value) {
    let path = schemas_dir().join(name);
    let rendered = answers::render(derived);

    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        std::fs::create_dir_all(schemas_dir()).expect("mkdir schemas/answers");
        std::fs::write(&path, &rendered).expect("write generated answer schema");
        return;
    }

    let on_disk = std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("generated answer schema {} missing ({err}); regenerate", path.display())
    });
    assert_eq!(
        rendered, on_disk,
        "{name} diverges from its derivation; regenerate via REGENERATE_GOLDENS=1"
    );
}

mod schemas {
    use super::*;

    /// Each generated `schemas/answers/` document byte-matches its
    /// derivation from the embedded canonical schemas.
    #[test]
    fn generated_files_match() {
        assert_generated("leads.schema.json", &answers::leads());
        assert_generated("evidence.schema.json", &answers::evidence());
        assert_generated("report.schema.json", &answers::report());
        assert_generated("proposal.schema.json", &answers::proposal());
        assert_generated("synthesis.schema.json", &answers::synthesis());
    }

    /// Every derived answer schema compiles standalone — the derivation
    /// inlined the cross-file diagnostic `$ref`, so no registry is needed.
    #[test]
    fn compile_standalone() {
        for (name, derived) in [
            ("leads", answers::leads()),
            ("evidence", answers::evidence()),
            ("report", answers::report()),
            ("proposal", answers::proposal()),
            ("synthesis", answers::synthesis()),
        ] {
            compile_schema(&answers::render(&derived))
                .unwrap_or_else(|err| panic!("{name} answer schema compiles standalone: {err}"));
        }
    }

    /// The closed enums the derived answers carry must match the sets
    /// `wit/specify.wit` declares (`authority`, `claim-kind`, `severity`,
    /// `platform`, `status`) — the WIT records deserialize schema-gated
    /// answers, so an enum widened on one side only is a contract break.
    /// Update the WIT and this mirror together.
    #[test]
    fn enums_match_wit_contract() {
        let evidence = answers::evidence();
        assert_eq!(
            enum_values(&evidence, "/$defs/authorityClass/enum"),
            ["intent", "documentation", "behaviour"],
            "evidence authority mirrors wit source.authority"
        );
        assert_eq!(
            enum_values(&evidence, "/$defs/claimKind/enum"),
            [
                "intent",
                "requirement",
                "criterion",
                "decision",
                "section",
                "diagram",
                "contract",
                "example",
                "excerpt",
                "type",
                "call",
                "region",
                "container",
                "leaf"
            ],
            "evidence claim kinds mirror wit source.claim-kind"
        );

        let synthesis = answers::synthesis();
        assert_eq!(
            enum_values(&synthesis, "/$defs/model-claimKind/enum"),
            enum_values(&answers::evidence(), "/$defs/claimKind/enum"),
            "synthesis model claim kinds mirror the evidence claim-kind set"
        );

        let report = answers::report();
        assert_eq!(
            enum_values(&report, "/$defs/diagnostic-severity/enum"),
            ["critical", "important", "suggestion", "optional"],
            "diagnostic severity mirrors wit target.severity"
        );
        assert_eq!(
            enum_values(&report, "/$defs/platform/enum"),
            ["core", "ios", "android", "web", "desktop"],
            "platform mirrors wit target.platform"
        );
        assert_eq!(
            enum_values(&report, "/properties/status/enum"),
            ["success", "failure"],
            "report status mirrors wit target.status"
        );
    }
}

fn enum_values(schema: &Value, pointer: &str) -> Vec<String> {
    schema
        .pointer(pointer)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("answer schema resolves `{pointer}` to an enum"))
        .iter()
        .map(|value| value.as_str().expect("enum values are strings").to_string())
        .collect()
}

fn assert_accepts(schema: &Value, instance: &Value, ctx: &str) {
    let summaries =
        validate_value(instance, &answers::render(schema), ctx, "worked answer example");
    assert!(
        summaries.iter().all(|s| matches!(s.status, ValidationStatus::Pass)),
        "{ctx}: worked example must validate; got {summaries:?}"
    );
}

fn assert_rejects(schema: &Value, instance: &Value, ctx: &str) {
    let summaries = validate_value(instance, &answers::render(schema), ctx, "rejected example");
    assert!(
        summaries.iter().any(|s| matches!(s.status, ValidationStatus::Fail)),
        "{ctx}: example must be rejected"
    );
}

mod leads {
    use super::*;

    /// The survey answer accepts a lead list without `source` keys and
    /// rejects an item that still carries the stripped envelope key.
    #[test]
    fn strips_source() {
        let schema = answers::leads();
        assert_accepts(
            &schema,
            &json!({ "leads": [{
            "lead": "user-registration",
            "synopsis": "Registration endpoint accepting email + password.",
            "topics": ["identity"]
        }] }),
            "answers/leads",
        );
        assert_rejects(
            &schema,
            &json!({ "leads": [{
            "lead": "user-registration",
            "source": "legacy-monolith",
            "synopsis": "The envelope source key must not survive derivation."
        }] }),
            "answers/leads-envelope-source",
        );
        assert_rejects(&schema, &json!([]), "answers/leads-bare-array");
    }
}

mod evidence {
    use super::*;

    /// The extract answer accepts Evidence without the envelope `lead` key,
    /// still enforces the canonical claim-kind enum and per-kind `id`
    /// requirement, and rejects a document that repeats `lead`.
    #[test]
    fn strips_lead() {
        let schema = answers::evidence();
        assert_accepts(
            &schema,
            &json!({
                "authority": "documentation",
                "claims": [{
                    "kind": "requirement",
                    "id": "users.register.email-validation",
                    "path": "src/users/register.ts#L12-L87",
                    "statement": "The system accepts registrations with RFC 5322 emails."
                }]
            }),
            "answers/evidence",
        );
        assert_rejects(
            &schema,
            &json!({ "authority": "behaviour", "lead": "user-registration", "claims": [] }),
            "answers/evidence-envelope-lead",
        );
        assert_rejects(
            &schema,
            &json!({ "authority": "documentation", "claims": [{ "kind": "hunch" }] }),
            "answers/evidence-bad-kind",
        );
        assert_rejects(
            &schema,
            &json!({ "authority": "documentation", "claims": [{ "kind": "requirement" }] }),
            "answers/evidence-requirement-missing-id",
        );
    }
}

mod proposal {
    use super::*;

    /// The proposal answer accepts a `kind: response` grouping carrying
    /// the Gate 1 prose, rejects a `kind: request` document (the request
    /// arm is trimmed), rejects a slice with no sources, and rejects an
    /// answer without the `gate` object (canonically optional, but the
    /// derivation makes it required on the judgment answer).
    #[test]
    fn response_only() {
        let schema = answers::proposal();
        let gate = json!({
            "change": "## Intent\n\nRefresh registration.",
            "discovery-summary": "Sources: 2. Leads: 1.",
            "discovery-source-inventory": "| key | adapter | path |\n|---|---|---|"
        });
        assert_accepts(
            &schema,
            &json!({
                "version": 1,
                "kind": "response",
                "slices": [{
                    "name": "user-registration",
                    "sources": [{ "source": "legacy", "lead": "user-registration" }],
                    "divergence": "likely",
                    "disagreements": [{
                        "field": "password-min-length",
                        "values": [
                            { "source": "legacy", "value": "8" },
                            { "source": "docs", "value": "12" }
                        ]
                    }]
                }],
                "gate": gate
            }),
            "answers/proposal",
        );
        assert_rejects(
            &schema,
            &json!({ "version": 1, "kind": "request", "projects": [], "leads": [] }),
            "answers/proposal-request-arm",
        );
        assert_rejects(
            &schema,
            &json!({
                "version": 1,
                "kind": "response",
                "slices": [{ "name": "x", "sources": [] }],
                "gate": gate
            }),
            "answers/proposal-empty-sources",
        );
        assert_rejects(
            &schema,
            &json!({
                "version": 1,
                "kind": "response",
                "slices": [{
                    "name": "user-registration",
                    "sources": [{ "source": "legacy", "lead": "user-registration" }]
                }]
            }),
            "answers/proposal-gate-required",
        );
    }
}

mod synthesis {
    use super::*;

    /// The synthesis answer accepts a response with the inlined model shape
    /// and rejects a model claim with a malformed kind — proving the
    /// cross-file inline preserved the canonical constraints.
    #[test]
    fn inlines_model() {
        let schema = answers::synthesis();
        assert_accepts(
            &schema,
            &json!({
                "version": 1,
                "kind": "response",
                "slice": "user-registration",
                "model": {
                    "requirements": [{
                        "title": "Register with email",
                        "statement": "The system accepts registrations with RFC 5322 emails.",
                        "domain": "identity",
                        "claims": [{ "source": "legacy", "id": "users.register", "kind": "requirement" }]
                    }],
                    "tasks": [{ "id": "TASK-001", "text": "Implement registration" }]
                },
                "artifacts": {
                    "proposal": "## Proposal",
                    "design": "## Design",
                    "tasks": "## Tasks",
                    "specs": [{ "domain": "identity", "content": "## Identity" }]
                }
            }),
            "answers/synthesis",
        );
        assert_rejects(
            &schema,
            &json!({
                "version": 1,
                "kind": "response",
                "slice": "user-registration",
                "model": {
                    "requirements": [{
                        "title": "x",
                        "statement": "y",
                        "domain": "identity",
                        "claims": [{ "source": "legacy", "id": "users.register", "kind": "hunch" }]
                    }],
                    "tasks": []
                },
                "artifacts": { "proposal": "p", "design": "d", "tasks": "t", "specs": [] }
            }),
            "answers/synthesis-bad-claim-kind",
        );
    }
}

mod report {
    use super::*;

    /// The report answer accepts a report without the envelope keys, with the
    /// inlined diagnostic shape governing `findings[]`, and rejects a report
    /// that still carries `version` / `slice` / `target` or a malformed
    /// finding.
    #[test]
    fn strips_envelope() {
        let schema = answers::report();
        assert_accepts(
            &schema,
            &json!({ "status": "success", "findings": [], "ui-surface": { "screens": 0 } }),
            "answers/report-success",
        );
        assert_accepts(
            &schema,
            &json!({
                "status": "failure",
                "findings": [{
                    "id": "DIAG-0001",
                    "rule-id": "contract.id-unique",
                    "title": "Duplicate info.x-specify-id across baseline",
                    "severity": "critical",
                    "source": "tool",
                    "artifact": "contracts",
                    "evidence": {
                        "kind": "structured",
                        "summary": "x-specify-id user-api collides with legacy-api.yaml",
                        "data": { "detail": "duplicate id" }
                    },
                    "impact": "Downstream consumers cannot resolve a unique contract id.",
                    "remediation": "Rename or remove the duplicate id before merge.",
                    "fingerprint": "sha256:a2e95674f838eb042eba78e16239f32199def3ca976e29499f8275beb30225e4"
                }],
                "outputs": [{ "platform": "core", "path": "contracts/http/user-api.yaml" }]
            }),
            "answers/report-failure-with-finding",
        );
        assert_rejects(
            &schema,
            &json!({
                "version": 1,
                "slice": "identity-contracts",
                "target": "contracts@1.0.0",
                "status": "success",
                "findings": []
            }),
            "answers/report-envelope-keys",
        );
        assert_rejects(
            &schema,
            &json!({ "status": "success", "findings": [{ "detail": "not a diagnostic" }] }),
            "answers/report-malformed-finding",
        );
    }
}
