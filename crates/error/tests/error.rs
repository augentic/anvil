//! Error discriminant and Display contract.

use emery_error::Error;

#[test]
fn diag_round_trip() {
    let err = Error::Diag {
        code: "kebab-prefix",
        detail: "specific detail".to_string(),
    };
    assert_eq!(err.variant_str(), "kebab-prefix");
    assert_eq!(err.to_string(), "kebab-prefix: specific detail");
}

#[test]
fn adapter_too_old_display() {
    let err = Error::AdapterCliTooOld {
        adapter: "omnia (omnia@1.0.0.wasm)".to_string(),
        required: "2.0.0".to_string(),
        found: "1.0.0".to_string(),
    };
    assert_eq!(err.variant_str(), "adapter-cli-too-old");
    let msg = err.to_string();
    assert!(
        msg.contains("1.0.0") && msg.contains("2.0.0") && msg.contains("omnia"),
        "versions and adapter in display: {msg}"
    );
    let hint = err.hint().expect("exit-3 errors carry a recovery hint");
    assert!(
        hint.contains("brew upgrade emery")
            && hint.contains("cargo install --git https://github.com/augentic/emery --locked")
            && !hint.contains("emery adapter"),
        "the hint names live install channels only: {hint}"
    );
}

#[test]
fn live_codes_hint() {
    let specify = Error::validation_failed("specify-source-required", "", "detail");
    let hint = specify.hint().expect("specify-source-required carries a hint");
    assert!(hint.contains("emery specify"), "{hint}");

    let show = Error::Diag {
        code: "spec-not-generated",
        detail: "detail".into(),
    };
    let hint = show.hint().expect("spec-not-generated carries a hint");
    assert!(hint.contains("emery specify"), "{hint}");

    let deleted = Error::validation_failed("plan-epoch-stale", "rule", "detail");
    assert!(deleted.hint().is_none(), "deleted verbs must not keep recovery hints");
}

#[test]
fn validation_code_display() {
    let err = Error::validation_failed("bad-thing", "rule", "detail");
    assert_eq!(err.variant_str(), "bad-thing");
    assert_eq!(err.to_string(), "bad-thing: rule: detail");
}

#[test]
fn empty_rule_omits_prefix() {
    // An empty rule must not leave a dangling separator.
    let err = Error::validation_failed("code", "", "just detail");
    assert_eq!(err.to_string(), "code: just detail");
    assert_eq!(err.variant_str(), "code");
}
