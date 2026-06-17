use serde_json::json;

use super::*;

fn fixture() -> ExampleClaim {
    ExampleClaim {
        kind: ExampleKind::Example,
        id: "users.register.happy-path".to_string(),
        path: Some("tests/data/replays/users-register/happy.json".to_string()),
        replay_digest: "sha256:7a2b".to_string(),
        input: Some(json!({
            "method": "POST",
            "route": "/users",
        })),
        output: Some(json!({
            "status": 201,
        })),
        statement: Some(
            "Registering with a fresh email returns 201 and publishes user.created.".to_string(),
        ),
    }
}

#[test]
fn round_trips_full_body() {
    let claim = fixture();
    let yaml = serde_saphyr::to_string(&claim).expect("serialise");
    assert!(yaml.contains("kind: example"));
    assert!(yaml.contains("id: users.register.happy-path"));
    assert!(yaml.contains("replay-digest:"));
    let reparsed: ExampleClaim = serde_saphyr::from_str(&yaml).expect("reparse");
    assert_eq!(claim, reparsed);
}

#[test]
fn elides_optional_fields_when_absent() {
    let claim = ExampleClaim {
        kind: ExampleKind::Example,
        id: "users.register.minimal".to_string(),
        path: None,
        replay_digest: "sha256:deadbeef".to_string(),
        input: None,
        output: None,
        statement: None,
    };
    let yaml = serde_saphyr::to_string(&claim).expect("serialise");
    for absent in ["path:", "input:", "output:", "statement:"] {
        assert!(!yaml.contains(absent), "{absent} must elide when None, got:\n{yaml}");
    }
}

#[test]
fn rejects_unknown_fields() {
    let raw = r#"{
            "kind": "example",
            "id": "x",
            "replay-digest": "sha256:a",
            "rogue": true
        }"#;
    let err =
        serde_json::from_str::<ExampleClaim>(raw).expect_err("unknown field must be rejected");
    assert!(
        err.to_string().contains("unknown field"),
        "expected deny_unknown_fields error, got: {err}"
    );
}
