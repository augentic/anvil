//! Evidence answer contract
//!
//! What an adapter can rely on from the evidence answer path: the schema
//! tracks the `Evidence` DTO, a well formed answer deserializes, unknown
//! fields in open bodies are tolerated, and a failing answer is repaired
//! in-adapter with the gate's findings.

use std::path::Path;

use emery_adapter::answers::evidence_schema;
use emery_adapter::types::{Authority, Backing, ClaimKind, Context, Evidence, SourceInput};
use emery_adapter::{content_note, evidence};
use omnia_test::guest::Scripted;

#[test]
fn schema_tracks_dto() {
    let schema: serde_json::Value =
        serde_json::from_str(&evidence_schema()).expect("generated schema parses");
    let claim = schema.pointer("/$defs/Claim").expect("Claim definition");

    assert!(claim.pointer("/properties/backing").is_some(), "schema carries claim backing");
    assert!(
        claim.pointer("/properties/payload").is_none()
            && claim.pointer("/properties/backing-path").is_none(),
        "removed flattened backing is absent"
    );
    assert_ne!(
        claim.get("additionalProperties"),
        Some(&serde_json::Value::Bool(false)),
        "open claim extras stay admitted"
    );
    assert_eq!(
        claim.pointer("/properties/id/pattern").and_then(serde_json::Value::as_str),
        Some("^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$")
    );
    assert_eq!(
        claim.pointer("/if/properties/kind/enum"),
        Some(&serde_json::json!(["requirement", "criterion", "example"]))
    );
    assert_eq!(claim.pointer("/then/required"), Some(&serde_json::json!(["id"])));
}

#[test]
fn evidence_deserializes() {
    let evidence: Evidence = serde_json::from_str(
        r#"{
            "authority": "behaviour",
            "claims": [
                {
                    "kind": "example",
                    "id": "password-reset.expiry",
                    "path": "captures/reset.json#L3-L9",
                    "synopsis": "Expired token is rejected.",
                    "backing": {"path": "captures/reset.json"},
                    "replay-digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                    "input": {"token": "stale"},
                    "output": {"status": 410}
                },
                {"kind": "type", "backing": {"payload": "struct ResetToken { expiry: Instant }"}}
            ]
        }"#,
    )
    .expect("evidence body parses");

    assert_eq!(evidence.authority, Authority::Behaviour);
    assert_eq!(evidence.claims.len(), 2);
    let example = &evidence.claims[0];
    assert_eq!(example.kind, ClaimKind::Example);
    assert_eq!(example.id.as_deref(), Some("password-reset.expiry"));
    assert_eq!(example.path.as_deref(), Some("captures/reset.json#L3-L9"));
    assert_eq!(example.backing, Some(Backing::Path("captures/reset.json".to_string())));
    // Open per-kind fields are preserved (A8).
    assert_eq!(
        example.extras.get("replay-digest").and_then(serde_json::Value::as_str),
        Some(concat!(
            "sha256:",
            "0000000000000000000000000000000000000000000000000000000000000000"
        )),
    );
    assert_eq!(example.extras["input"], serde_json::json!({"token": "stale"}));
    assert_eq!(example.extras["output"], serde_json::json!({"status": 410}));
    assert!(!example.extras.contains_key("synopsis"), "modeled keys stay typed");
    let claim = &evidence.claims[1];
    assert_eq!(claim.kind, ClaimKind::Type, "`type` deserializes despite being a keyword");
    assert_eq!(
        claim.backing,
        Some(Backing::Payload("struct ResetToken { expiry: Instant }".to_string()))
    );
    assert!(claim.id.is_none() && claim.path.is_none() && claim.synopsis.is_none());
    assert!(claim.extras.is_empty(), "no unmodeled keys, no extras");
}

// Unpinned `synopsis` and `backing` shapes become absent, not fatal.
#[test]
fn open_body_fields_lenient() {
    let evidence: Evidence = serde_json::from_str(
        r#"{
            "authority": "documentation",
            "claims": [
                {"kind": "section", "synopsis": {"headline": "structured"}, "backing": "bare string"},
                {"kind": "decision", "synopsis": "kept", "backing": {"payload": "ADR-7"}}
            ]
        }"#,
    )
    .expect("unpinned body shapes never fail the answer");

    let odd = &evidence.claims[0];
    assert!(odd.synopsis.is_none(), "non-string synopsis is dropped");
    assert!(odd.backing.is_none(), "non-variant backing is dropped");
    let clean = &evidence.claims[1];
    assert_eq!(clean.synopsis.as_deref(), Some("kept"), "modeled shapes still parse");
    assert_eq!(clean.backing, Some(Backing::Payload("ADR-7".to_string())));
}

// Parse failures and gate findings — id grammar and required extras
// alike — are repaired in-adapter, so the engine never sees the claim it
// would otherwise refuse.
#[tokio::test]
async fn evidence_repairs_gate_findings() {
    let model = Scripted::answering([
        r#"{"authority":"documentation"}"#,
        r#"{"authority":"documentation","claims":[{"kind":"requirement"}]}"#,
        r#"{"authority":"documentation","claims":[
            {"kind":"requirement","id":"password-reset.request","statement":"Users reset by email."},
            {"kind":"decision"}
        ]}"#,
    ]);
    let ctx = Context {
        adapter_id: "source:probe",
        project_root: Path::new("."),
        docs: &[],
        lend: None,
    };

    let accepted = evidence(&model, &ctx, "SYSTEM".to_string(), "USER".to_string())
        .await
        .expect("the third answer passes the gate");
    assert_eq!(accepted.claims.len(), 2);

    let requests = model.requests();
    assert_eq!(requests.len(), 3, "two repairs");
    let first = &requests[1].messages[0].content;
    assert!(first.contains("did not deserialize"), "{first}");
    let second = &requests[2].messages[0].content;
    assert!(second.contains("claims require an id"), "finding names the missing id: {second}");
    assert!(second.contains("missing extra `statement`"), "and the extra: {second}");
}

#[test]
fn content_note_names_the_binding() {
    let workspace = content_note(&SourceInput::workspace("docs", "/lend/docs"), "the docs tree");
    assert!(workspace.contains("`/lend/docs` — the docs tree the prompt walks"), "{workspace}");

    let value = content_note(&SourceInput::value("brief", "Ship it."), "unused");
    assert!(value.contains("no `$SOURCE_DIR` is lent:\n\nShip it."), "{value}");
}
