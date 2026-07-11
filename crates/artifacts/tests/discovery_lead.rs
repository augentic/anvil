//! Lead wire-shape round-trip matrix (`artifacts::discovery::lead`).

use artifacts::discovery::lead::Lead;

#[test]
fn optional_topics_round_trip() {
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
}

#[test]
fn absent_topics_default_empty() {
    let yaml = r"
lead: user-registration
source: legacy-monolith
synopsis: Registration endpoint.
";
    let parsed: Lead = serde_saphyr::from_str(yaml).expect("parse");
    assert!(parsed.topics.is_empty());
    let rendered = serde_saphyr::to_string(&parsed).expect("render");
    assert!(!rendered.contains("topics"), "empty topics must stay off the wire: {rendered}");
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
