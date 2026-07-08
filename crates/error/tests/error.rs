//! Error discriminant and Display contract.

use specify_error::Error;

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
fn cli_too_old_discriminant_display() {
    let err = Error::CliTooOld {
        required: "1.0.0".to_string(),
        found: "0.9.0".to_string(),
    };
    assert_eq!(err.variant_str(), "specify-version-too-old");
    let msg = err.to_string();
    assert!(msg.contains("0.9.0") && msg.contains("1.0.0"), "both versions in display: {msg}");
    assert!(err.hint().is_none(), "CliTooOld has no recovery hint");
}

#[test]
fn adapter_cli_too_old_discriminant_display() {
    // The adapter floor reuses the exit-3 family but carries
    // a distinct discriminant naming the adapter that outran the binary.
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
    assert!(err.hint().is_none(), "AdapterCliTooOld has no recovery hint");
}

#[test]
fn validation_static_code_and_display() {
    // The common path borrows a `&'static str` code, and
    // `validation_failed` folds `rule` + `detail` into one message.
    let err = Error::validation_failed("bad-thing", "rule", "detail");
    assert_eq!(err.variant_str(), "bad-thing");
    assert_eq!(err.to_string(), "bad-thing: rule: detail");
}

#[test]
fn validation_empty_rule_omits_prefix() {
    // Edge: an empty `rule` must not leave a dangling `": "` prefix.
    let err = Error::validation_failed("code", "", "just detail");
    assert_eq!(err.to_string(), "code: just detail");
    assert_eq!(err.variant_str(), "code");
}
