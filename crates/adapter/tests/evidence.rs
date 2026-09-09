//! The evidence call
//!
//! What an adapter can rely on from `evidence`: the request it builds (the
//! `Evidence` schema with the claim-id pattern, `check` set, the reference
//! tools and workspace lend following the context), reference calls answered
//! from the embedded corpus, a candidate the claim gate rejects corrected in
//! place, the backend's spent rounds surfacing as `Internal` with the last
//! findings, and a host refusal passing through typed.

use std::path::Path;

use emery_adapter::types::{Authority, Backing, ClaimKind, Context, Error, Evidence, SourceInput};
use emery_adapter::{Error as ModelError, ToolCall, content_note, evidence};
use emery_prose::registry::Doc;
use omnia_test::SeenFormat;
use omnia_test::guest::Scripted;

const DOCS: &[Doc] = &[Doc {
    path: "references/greeting.md",
    body: "Greet warmly.",
}];

const VALID: &str = r#"{"authority":"documentation","claims":[
    {"kind":"requirement","id":"password-reset.request","statement":"Users reset by email."},
    {"kind":"decision"}
]}"#;

fn context(docs: &'static [Doc], lend: Option<&str>) -> Context<'static> {
    Context {
        adapter_id: "source:probe",
        project_root: Path::new("."),
        docs,
        lend: lend.map(str::to_string),
    }
}

async fn ask(model: &Scripted, ctx: &Context<'_>) -> Result<Evidence, Error> {
    evidence(model, ctx, "SYSTEM".to_string(), "USER".to_string()).await
}

// The request carries the system prose, the user turn, the derived
// `Evidence` schema under `evidence` with the claim-id grammar as a steering
// pattern, and the check the backend loops on.
#[tokio::test]
async fn request_shape() {
    let model = Scripted::answering([VALID]);
    let ctx = context(DOCS, Some("/lend/docs"));

    let accepted = ask(&model, &ctx).await.expect("a valid answer is accepted first time");
    assert_eq!(accepted.claims.len(), 2);

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.system.as_deref(), Some("SYSTEM"));
    assert_eq!(request.messages, ["USER"]);
    assert!(request.check, "acceptance is the check, not the reply text");
    assert_eq!(request.tools, ["list_docs", "read_doc"], "a docs-carrying call offers the tools");
    assert_eq!(request.workspace.as_deref(), Some("/lend/docs"), "the lend follows the context");

    let SeenFormat::Schema { name, schema } = &request.format else {
        panic!("evidence is steered by schema");
    };
    assert_eq!(name, "evidence");
    let schema: serde_json::Value = serde_json::from_str(schema).expect("generated schema parses");
    let claim = schema.pointer("/$defs/Claim").expect("Claim definition");
    assert_eq!(
        claim.pointer("/properties/id/pattern").and_then(serde_json::Value::as_str),
        Some("^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$")
    );
    assert!(claim.pointer("/properties/backing").is_some(), "schema tracks the DTO");
    assert_ne!(
        claim.get("additionalProperties"),
        Some(&serde_json::Value::Bool(false)),
        "open claim extras stay admitted"
    );
    model.assert_exhausted();
}

// Without embedded docs no tools are declared; an inline value lends nothing.
#[tokio::test]
async fn bare_context() {
    let model = Scripted::answering([VALID]);
    let ctx = context(&[], None);

    ask(&model, &ctx).await.expect("accepted");
    let request = &model.seen()[0];
    assert!(request.tools.is_empty(), "no docs, no tools");
    assert!(request.workspace.is_none(), "no lend for an inline value");
}

// Reference calls are answered in-process from the corpus before the
// candidate is checked.
#[tokio::test]
async fn references_answered() {
    let model = Scripted::answering([VALID]).calling(
        0,
        [
            ToolCall {
                id: "1".to_string(),
                name: "list_docs".to_string(),
                arguments: "{}".to_string(),
            },
            ToolCall {
                id: "2".to_string(),
                name: "read_doc".to_string(),
                arguments: r#"{"path":"references/greeting.md"}"#.to_string(),
            },
        ],
    );
    let ctx = context(DOCS, None);

    ask(&model, &ctx).await.expect("accepted");
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 3, "two reference calls, then the check");
    assert_eq!(exchanges[0].outcome.as_deref(), Ok(r#"{"paths":["references/greeting.md"]}"#));
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"body":"Greet warmly.","path":"references/greeting.md"}"#)
    );
    assert_eq!(exchanges[2].tool, "check");
    assert_eq!(exchanges[2].outcome, Ok(String::new()));
}

// A candidate the claim gate rejects — a missing id, a missing extra — is
// sent back as the correction and the next candidate is checked again, so
// the engine never sees the claim it would otherwise refuse.
#[tokio::test]
async fn gate_findings_corrected() {
    let model = Scripted::answering([
        r#"{"authority":"documentation","claims":[{"kind":"requirement"}]}"#,
        VALID,
    ]);
    let ctx = context(&[], None);

    let accepted = ask(&model, &ctx).await.expect("the second candidate passes the gate");
    assert_eq!(accepted.claims.len(), 2);

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 2, "one rejection, one acceptance");
    assert_eq!(exchanges[0].tool, "check");
    let correction = exchanges[0].outcome.as_ref().expect_err("the first candidate is rejected");
    assert!(correction.contains("## Previous answer (rejected)"), "{correction}");
    assert!(correction.contains("## Findings"), "{correction}");
    assert!(correction.contains("`requirement` claims require an id"), "{correction}");
    assert!(correction.contains("missing extra `statement`"), "{correction}");
    assert_eq!(exchanges[1].outcome, Ok(String::new()));
    assert_eq!(model.requests().len(), 2, "one scripted answer per attempt");
}

// When the backend spends its rounds on a rejected candidate the last
// findings surface as `Internal`, so the host's own error is never the
// adapter's answer.
#[tokio::test]
async fn rounds_exhausted() {
    let model = Scripted::answering([
        r#"{"authority":"documentation","claims":[{"kind":"criterion","id":"Not.Valid","criterion":"x"}]}"#,
    ]);
    let ctx = context(&[], None);

    let error = ask(&model, &ctx).await.expect_err("the only candidate fails the gate");
    let Error::Internal(detail) = error else {
        panic!("spent rounds are internal: {error}");
    };
    assert!(detail.contains("budget exhausted"), "{detail}");
    assert!(detail.contains("id `Not.Valid` does not match"), "{detail}");
    assert_eq!(model.exchanges().len(), 1, "one check, rejected");
}

// A request the host refuses is the adapter's `InvalidRequest`, untouched.
#[tokio::test]
async fn invalid_request_passes_through() {
    let model = Scripted::new([Err(ModelError::InvalidRequest("no such model".to_string()))]);
    let ctx = context(&[], None);

    let error = ask(&model, &ctx).await.expect_err("the host refused");
    assert!(
        matches!(&error, Error::InvalidRequest(detail) if detail == "no such model"),
        "{error}"
    );
    assert!(model.exchanges().is_empty(), "nothing to check");
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

#[test]
fn content_note_names_the_binding() {
    let workspace = content_note(&SourceInput::workspace("docs", "/lend/docs"), "the docs tree");
    assert!(workspace.contains("`/lend/docs` — the docs tree the prompt walks"), "{workspace}");

    let value = content_note(&SourceInput::value("brief", "Ship it."), "unused");
    assert!(value.contains("no `$SOURCE_DIR` is lent:\n\nShip it."), "{value}");
}
