//! Evidence answer tests.

use emery_adapter::answers::{evidence_schema, parse_evidence, validate_evidence};
use emery_adapter::types::{Authority, Backing, ClaimKind, Error};

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
    let evidence = parse_evidence(
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
    let evidence = parse_evidence(
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
fn evidence_tail() {
    let clean = parse_evidence(
        r#"{"authority":"documentation","claims":[
            {"kind":"requirement","id":"password-reset.request"},
            {"kind":"decision"}
        ]}"#,
    )
    .expect("clean evidence parses");
    validate_evidence(&clean).expect("clean evidence passes the tail");

    let malformed = parse_evidence(
        r#"{"authority":"documentation","claims":[
            {"kind":"requirement"},
            {"kind":"criterion","id":"Not.Valid"}
        ]}"#,
    )
    .expect("the tail, not the parser, rejects malformed claims");
    let Err(Error::Internal(detail)) = validate_evidence(&malformed) else {
        panic!("malformed evidence must fail the tail");
    };
    assert!(detail.contains("claims require an id"), "finding names the missing id: {detail}");
    assert!(detail.contains("`Not.Valid`"), "finding names the malformed id: {detail}");
}
