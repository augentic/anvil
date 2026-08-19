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
fn cli_too_old_display() {
    let err = Error::CliTooOld {
        required: "1.0.0".to_string(),
        found: "0.9.0".to_string(),
    };
    assert_eq!(err.variant_str(), "emery-version-too-old");
    let msg = err.to_string();
    assert!(msg.contains("0.9.0") && msg.contains("1.0.0"), "both versions in display: {msg}");
    let hint = err.hint().expect("exit-3 errors carry a recovery hint");
    assert!(hint.contains("install.sh"), "the hint names the install channel: {hint}");
}

#[test]
fn adapter_too_old_display() {
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
    let hint = err.hint().expect("exit-3 errors carry a recovery hint");
    assert!(
        hint.contains("install.sh") && hint.contains("emery adapter upgrade"),
        "the hint names the install channel and the adapter-upgrade fallback: {hint}"
    );
}

#[test]
fn validation_code_display() {
    // The common path borrows a `&'static str` code, and
    // `validation_failed` folds `rule` + `detail` into one message.
    let err = Error::validation_failed("bad-thing", "rule", "detail");
    assert_eq!(err.variant_str(), "bad-thing");
    assert_eq!(err.to_string(), "bad-thing: rule: detail");
}

#[test]
fn execute_codes_hint() {
    // The RFC-86 execute refusals are the operator's main exit-2
    // surface; each carries a recovery hint mirroring
    // docs/reference/diagnostics.md.
    for (code, expect) in [
        ("plan-epoch-stale", "emery plan execute"),
        ("guest-marker-held", ".emery/change/guest.lock"),
    ] {
        let err = Error::validation_failed(code, "rule", "detail");
        let hint = err.hint().unwrap_or_else(|| panic!("{code} carries a hint"));
        assert!(hint.contains(expect), "{code} hint names the recovery gesture: {hint}");
    }
}

#[test]
fn closed_plan_codes_hint() {
    for (code, expect) in [
        ("plan-discovery-mismatch", "emery plan author"),
        ("plan-definition-stale", "emery plan author"),
        ("plan-epoch-required", "emery plan execute"),
        ("target-base-freeze-detached", "accepted CID"),
    ] {
        let err = Error::Diag {
            code,
            detail: "detail".into(),
        };
        let hint = err.hint().unwrap_or_else(|| panic!("{code} carries a hint"));
        assert!(hint.contains(expect), "{code} hint names the recovery gesture: {hint}");
    }
}

#[test]
fn empty_rule_omits_prefix() {
    // Edge: an empty `rule` must not leave a dangling `": "` prefix.
    let err = Error::validation_failed("code", "", "just detail");
    assert_eq!(err.to_string(), "code: just detail");
    assert_eq!(err.variant_str(), "code");
}
