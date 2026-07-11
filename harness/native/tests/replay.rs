//! Omnia model replay semantics: answers are keyed on the reduced
//! request (`ModelDefault`'s replay convention), a miss is a typed
//! backend failure, and a missing fixture directory is an empty store.

use omnia_guest::Model as _;
use omnia_guest::model::{Format, Message, Request, Role};
use omnia_testkit::model::Replay;
use serde_json::json;
use tempfile::TempDir;

fn request(content: &str) -> Request {
    Request {
        system: Some("SYSTEM".to_string()),
        messages: vec![Message {
            role: Role::User,
            content: content.to_string(),
        }],
        format: Format::Json,
        ..Request::default()
    }
}

/// The reduced-request key for [`request`], mirroring the recorder.
fn key(content: &str) -> serde_json::Value {
    json!({
        "model": null,
        "system": "SYSTEM",
        "messages": [{ "role": "user", "content": content }],
        "generation": null,
        "format": { "kind": "json" },
        "tools": [],
        "grants": { "references": null, "verify": [] },
    })
}

#[tokio::test]
async fn replays_recorded_answer() {
    let tmp = TempDir::new().expect("tempdir");
    let fixture = json!({
        "key_request": key("What is the plan?"),
        "answer": { "leads": [] },
        "usage": { "input_tokens": 3, "output_tokens": 5 },
    });
    std::fs::write(tmp.path().join("survey.json"), fixture.to_string()).expect("write fixture");

    let model = Replay::from_dir(tmp.path()).expect("load fixtures");
    let reply = model.create(request("What is the plan?")).await.expect("replayed answer");

    assert_eq!(reply.answer, r#"{"leads":[]}"#);
    assert_eq!(reply.usage.expect("usage carried").output_tokens, 5);
}

#[tokio::test]
async fn unmatched_request_refused() {
    let tmp = TempDir::new().expect("tempdir");
    let model = Replay::from_dir(tmp.path()).expect("load empty dir");

    let err = model.create(request("anything")).await.expect_err("no fixture");
    assert!(err.to_string().contains("no replay fixture"), "{err}");
}

#[test]
fn missing_dir_is_empty_store() {
    let tmp = TempDir::new().expect("tempdir");
    Replay::from_dir(tmp.path().join("absent")).expect("missing dir loads empty");
}

#[test]
fn malformed_fixture_refused() {
    let tmp = TempDir::new().expect("tempdir");
    std::fs::write(tmp.path().join("bad.json"), "not json").expect("write fixture");

    let err = Replay::from_dir(tmp.path()).expect_err("malformed fixture");
    assert!(err.to_string().contains("bad.json"), "{err:#}");
}
