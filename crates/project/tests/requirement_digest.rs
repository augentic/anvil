//! RFC-86a D2: canonical requirement-body digest kernel.

use project::slice::RequirementBody;

fn digest(title: &str, statement: &str, scenarios: &[&str], notes: Option<&str>) -> String {
    let scenarios: Vec<String> = scenarios.iter().map(ToString::to_string).collect();
    RequirementBody {
        title,
        statement,
        scenarios: &scenarios,
        notes,
    }
    .digest()
}

fn base() -> String {
    digest(
        "password reset",
        "the reset path is not evidenced",
        &["GIVEN a registered user", "WHEN the reset link is requested"],
        Some("intent is silent on this flow"),
    )
}

#[test]
fn wire_shape() {
    let d = base();
    let hex = d.strip_prefix("sha256:").expect("sha256: scheme");
    assert_eq!(hex.len(), 64);
    assert!(hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
}

#[test]
fn identical_bodies_match() {
    assert_eq!(base(), base());
}

#[test]
fn title_sensitive() {
    let changed = digest(
        "password reset flow",
        "the reset path is not evidenced",
        &["GIVEN a registered user", "WHEN the reset link is requested"],
        Some("intent is silent on this flow"),
    );
    assert_ne!(base(), changed);
}

#[test]
fn statement_sensitive() {
    let changed = digest(
        "password reset",
        "the reset path is now evidenced",
        &["GIVEN a registered user", "WHEN the reset link is requested"],
        Some("intent is silent on this flow"),
    );
    assert_ne!(base(), changed);
}

mod scenarios {
    use super::{base, digest};

    #[test]
    fn content_sensitive() {
        let changed = digest(
            "password reset",
            "the reset path is not evidenced",
            &["GIVEN an anonymous user", "WHEN the reset link is requested"],
            Some("intent is silent on this flow"),
        );
        assert_ne!(base(), changed);
    }

    #[test]
    fn order_sensitive() {
        let changed = digest(
            "password reset",
            "the reset path is not evidenced",
            &["WHEN the reset link is requested", "GIVEN a registered user"],
            Some("intent is silent on this flow"),
        );
        assert_ne!(base(), changed);
    }

    #[test]
    fn count_sensitive() {
        let changed = digest(
            "password reset",
            "the reset path is not evidenced",
            &["GIVEN a registered user", "WHEN the reset link is requested", ""],
            Some("intent is silent on this flow"),
        );
        assert_ne!(base(), changed);
    }
}

mod notes {
    use super::{base, digest};

    #[test]
    fn content_sensitive() {
        let changed = digest(
            "password reset",
            "the reset path is not evidenced",
            &["GIVEN a registered user", "WHEN the reset link is requested"],
            Some("docs are silent on this flow"),
        );
        assert_ne!(base(), changed);
    }

    #[test]
    fn absence_sensitive() {
        let changed = digest(
            "password reset",
            "the reset path is not evidenced",
            &["GIVEN a registered user", "WHEN the reset link is requested"],
            None,
        );
        assert_ne!(base(), changed);
    }
}

mod framing {
    use super::digest;

    // Equal concatenated bytes split differently across field
    // boundaries must not collide.
    #[test]
    fn field_boundary() {
        assert_ne!(digest("ab", "", &[], None), digest("a", "b", &[], None));
    }

    // A statement embedding text that mimics a scenario record must
    // not collide with an actual scenario.
    #[test]
    fn embedded_record_header() {
        let inlined = digest("t", "s\nscenario 5\nGIVEN", &[], None);
        let structured = digest("t", "s", &["GIVEN"], None);
        assert_ne!(inlined, structured);
    }

    #[test]
    fn scenario_split() {
        assert_ne!(digest("t", "s", &["ab", ""], None), digest("t", "s", &["a", "b"], None));
    }
}
