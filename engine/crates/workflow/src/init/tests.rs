use tempfile::tempdir;

use super::*;
use crate::adapter::PlatformsCapability;

#[test]
fn rejects_missing_adapter() {
    let tmp = tempdir().unwrap();
    let err = init(
        InitOptions {
            project_dir: tmp.path(),
            adapter: None,
            name: Some("demo"),
            description: None,
            workspace: false,
            include_framework: false,
            platforms: None,
            upgrade: false,
        },
        fixed_now(),
    )
    .expect_err("missing adapter must error");
    assert!(
        matches!(
            &err,
            Error::Diag {
                code: "init-requires-adapter-or-workspace",
                ..
            }
        ),
        "got: {err:?}"
    );
}

#[test]
fn workspace_rejects_adapter_argument() {
    // `--workspace` and `<adapter>` are mutually exclusive; the
    // orchestrator re-checks even when the CLI layer already
    // filtered.
    let tmp = tempdir().unwrap();
    let err = init(
        InitOptions {
            project_dir: tmp.path(),
            adapter: Some("omnia"),
            name: Some("platform-workspace"),
            description: None,
            workspace: true,
            include_framework: false,
            platforms: None,
            upgrade: false,
        },
        fixed_now(),
    )
    .expect_err("workspace + adapter must error");
    assert!(
        matches!(
            &err,
            Error::Diag {
                code: "init-requires-adapter-or-workspace",
                ..
            }
        ),
        "got: {err:?}"
    );
}

#[test]
fn validate_no_capability_passthrough() {
    let result = validate_platforms(Some(&[Platform::Core, Platform::Ios]), None, "test");
    assert_eq!(result.unwrap(), vec![Platform::Core, Platform::Ios]);
}

#[test]
fn validate_no_cap_no_op_empty() {
    let result = validate_platforms(None, None, "test");
    assert!(result.unwrap().is_empty());
}

#[test]
fn validate_required_none_fails() {
    let cap = PlatformsCapability {
        required: true,
        allowed: vec![Platform::Core, Platform::Ios],
        default: vec![Platform::Core, Platform::Ios],
    };
    let err = validate_platforms(None, Some(&cap), "vectis").unwrap_err();
    let Error::Validation { code, .. } = err else {
        panic!("expected Validation, got: {err:?}");
    };
    assert_eq!(code, "project-platforms-required");
}

#[test]
fn validate_platforms_missing_core_fails() {
    let cap = PlatformsCapability {
        required: true,
        allowed: vec![Platform::Core, Platform::Ios, Platform::Android],
        default: vec![Platform::Core, Platform::Ios, Platform::Android],
    };
    let err = validate_platforms(Some(&[Platform::Ios, Platform::Android]), Some(&cap), "vectis")
        .unwrap_err();
    let Error::Validation { code, .. } = err else {
        panic!("expected Validation, got: {err:?}");
    };
    assert_eq!(code, "project-platforms-must-include-core");
}

#[test]
fn validate_platforms_not_allowed_fails() {
    let cap = PlatformsCapability {
        required: true,
        allowed: vec![Platform::Core, Platform::Ios],
        default: vec![Platform::Core, Platform::Ios],
    };
    let err = validate_platforms(
        Some(&[Platform::Core, Platform::Ios, Platform::Android]),
        Some(&cap),
        "vectis",
    )
    .unwrap_err();
    let Error::Validation { code, .. } = err else {
        panic!("expected Validation, got: {err:?}");
    };
    assert_eq!(code, "project-platforms-not-allowed");
}

#[test]
fn validate_platforms_valid_set_succeeds() {
    let cap = PlatformsCapability {
        required: true,
        allowed: vec![Platform::Core, Platform::Ios, Platform::Android, Platform::Web],
        default: vec![Platform::Core, Platform::Ios, Platform::Android],
    };
    let result = validate_platforms(
        Some(&[Platform::Core, Platform::Ios, Platform::Android]),
        Some(&cap),
        "vectis",
    );
    assert_eq!(result.unwrap(), vec![Platform::Core, Platform::Ios, Platform::Android]);
}
