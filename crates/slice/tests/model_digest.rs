//! RFC-86a D2: requirement body digest over the typed `model.yaml` —
//! kernel-owned fields (id, status, sources) are digest-neutral.

use slice::SliceModel;

fn parse(yaml: &str) -> SliceModel {
    SliceModel::parse_yaml(yaml).expect("model.yaml parses")
}

const ORIGINAL: &str = r"requirements:
  - id: REQ-003
    title: password reset
    status: unknown
    sources: [docs]
    statement: the reset path is not evidenced
    scenarios:
      - GIVEN a registered user
      - WHEN the reset link is requested
    notes: intent is silent on this flow
";

// Same body after a re-refine renumbered the id and re-derived the
// kernel-owned fields.
const RENUMBERED: &str = r"requirements:
  - id: REQ-007
    title: password reset
    status: conflict
    sources: [docs, intent]
    statement: the reset path is not evidenced
    scenarios:
      - GIVEN a registered user
      - WHEN the reset link is requested
    notes: intent is silent on this flow
";

const RESHAPED: &str = r"requirements:
  - id: REQ-003
    title: password reset
    status: unknown
    sources: [docs]
    statement: the reset path sends a signed link over email
    scenarios:
      - GIVEN a registered user
      - WHEN the reset link is requested
    notes: intent is silent on this flow
";

#[test]
fn renumbering_neutral() {
    let original = parse(ORIGINAL).requirements[0].body_digest();
    let renumbered = parse(RENUMBERED).requirements[0].body_digest();
    assert_eq!(original, renumbered);
}

#[test]
fn body_change_sensitive() {
    let original = parse(ORIGINAL).requirements[0].body_digest();
    let reshaped = parse(RESHAPED).requirements[0].body_digest();
    assert_ne!(original, reshaped);
}

// Identical bodies in one model share one digest (degenerate but legal
// — same content, same decision).
#[test]
fn identical_bodies_share_digest() {
    let twice = format!(
        "{ORIGINAL}  - id: REQ-004\n    title: password reset\n    status: unknown\n    \
         sources: [docs]\n    statement: the reset path is not evidenced\n    scenarios:\n      - \
         GIVEN a registered user\n      - WHEN the reset link is requested\n    notes: intent is \
         silent on this flow\n"
    );
    let model = parse(&twice);
    assert_eq!(model.requirements[0].body_digest(), model.requirements[1].body_digest());
}
