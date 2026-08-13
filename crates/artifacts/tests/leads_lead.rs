//! Lead wire-shape round-trip matrix (`artifacts::leads::lead`).

use artifacts::leads::lead::Lead;

#[test]
fn optional_topics_round() {
    let yaml = r"
lead: user-registration
source: legacy-monolith
synopsis: Registration endpoint accepting email + password.
topics:
  - identity
  - account-creation
";
    let parsed: Lead = serde_saphyr::from_str(yaml).expect("parse");
    assert_eq!(parsed.topics, ["identity", "account-creation"]);
    assert!(parsed.parent.is_none());
    assert!(parsed.focus.is_none());
}

#[test]
fn parent_and_focus_round() {
    let yaml = r"
lead: orders-create
source: code
synopsis: Create order.
parent: orders-api
focus: POST /orders
";
    let parsed: Lead = serde_saphyr::from_str(yaml).expect("parse");
    assert_eq!(parsed.parent.as_deref(), Some("orders-api"));
    assert_eq!(parsed.focus.as_deref(), Some("POST /orders"));
    let rendered = serde_saphyr::to_string(&parsed).expect("render");
    assert!(rendered.contains("parent:"), "{rendered}");
    assert!(rendered.contains("focus:"), "{rendered}");
}

#[test]
fn absent_topics_default() {
    let yaml = r"
lead: user-registration
source: legacy-monolith
synopsis: Registration endpoint.
";
    let parsed: Lead = serde_saphyr::from_str(yaml).expect("parse");
    assert!(parsed.topics.is_empty());
    let rendered = serde_saphyr::to_string(&parsed).expect("render");
    assert!(!rendered.contains("topics"), "empty topics must stay off the wire: {rendered}");
    assert!(!rendered.contains("parent"), "{rendered}");
    assert!(!rendered.contains("focus"), "{rendered}");
}

#[test]
fn retired_aliases_rejected() {
    let yaml = r"
lead: user-registration
source: legacy-monolith
synopsis: Registration endpoint.
aliases:
  - account-registration
";
    let err = serde_saphyr::from_str::<Lead>(yaml).expect_err("aliases field must fail");
    let msg = err.to_string();
    assert!(msg.contains("unknown field") || msg.contains("aliases"), "{msg}");
}
