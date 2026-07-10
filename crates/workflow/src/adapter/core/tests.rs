use std::path::Path;

use error::Error;

use super::*;
use crate::Platform;

// These tests exercise the `pub(super)` post-resolve gates that cannot
// move to an integration suite (identity comes from the
// package reference, metadata from the component's `metadata` answer —
// there is no manifest to parse). Adapter-specific invariants belong in
// the adapter's own suite under `specify-adapters`.

#[test]
fn axis_routing() {
    assert_eq!(Axis::Source.dir_segment(), "sources");
    assert_eq!(Axis::Target.dir_segment(), "targets");
    assert_eq!(Axis::Source.interface(), "specify:adapter/source@0.1.0");
    assert_eq!(Axis::Target.interface(), "specify:adapter/target@0.1.0");

    // The operation sets derive from the closed WIT contract:
    // extract < survey for sources, build < merge < shape for targets.
    let source = SourceAdapter {
        name: "demo-source".into(),
        version: semver::Version::new(1, 0, 0),
        requires_specify: None,
    };
    assert_eq!(
        source.operations().copied().collect::<Vec<_>>(),
        vec![SourceOperation::Extract, SourceOperation::Survey]
    );
    let target = TargetAdapter {
        name: "demo-target".into(),
        version: semver::Version::new(1, 0, 0),
        requires_specify: None,
        inputs: Vec::new(),
        platforms: None,
    };
    assert_eq!(
        target.operations().copied().collect::<Vec<_>>(),
        vec![TargetOperation::Build, TargetOperation::Guidance, TargetOperation::Merge]
    );

    // Identity: a pin resolves as itself, a bare name as the
    // `0.0.0` development placeholder.
    assert_eq!(
        AdapterRef::pinned("demo", semver::Version::new(1, 2, 3)).resolved_version(),
        semver::Version::new(1, 2, 3)
    );
    assert_eq!(AdapterRef::bare("demo").resolved_version(), dev_version());
}

#[test]
fn typed_gates() {
    let component = Path::new("/store/demo@1.0.0.wasm");

    // The `specify-floor` string from a describe answer parses into a
    // typed semver; a non-semver floor is `adapter-floor-malformed`;
    // an absent floor is `None`.
    assert_eq!(parse_floor(None, "demo", component).expect("absent floor passes"), None);
    assert_eq!(
        parse_floor(Some("0.28.0"), "demo", component).expect("exact semver parses"),
        Some(semver::Version::new(0, 28, 0))
    );
    let Error::Validation { code, .. } = parse_floor(Some("1"), "demo", component)
        .expect_err("integer-shaped floor must be rejected")
    else {
        panic!("expected Error::Validation");
    };
    assert_eq!(code, "adapter-floor-malformed");

    // An absent floor never gates; a binary at/above the floor
    // passes; below the floor aborts on the exit-3 path; an unparseable
    // current version is permissive (mirrors config::version_is_older).
    check_requires_specify(None, "0.1.0", "demo-source", component)
        .expect("absent floor never gates");
    let floor = semver::Version::new(0, 28, 0);
    check_requires_specify(Some(&floor), "0.28.0", "demo-target", component)
        .expect("exact floor passes");
    check_requires_specify(Some(&floor), "0.29.1", "demo-target", component).expect("newer passes");
    let two = semver::Version::new(2, 0, 0);
    let err = check_requires_specify(Some(&two), "1.5.0", "demo-target", component)
        .expect_err("a binary below the floor must be rejected");
    assert_eq!(err.variant_str(), "adapter-cli-too-old");
    let Error::AdapterCliTooOld { required, found, .. } = err else {
        panic!("expected Error::AdapterCliTooOld, got: {err:?}");
    };
    assert_eq!(required, "2.0.0");
    assert_eq!(found, "1.5.0");
    check_requires_specify(Some(&two), "not-a-version", "demo-target", component)
        .expect("unparseable current version is permissive");
}

#[test]
fn platforms_capability_check() {
    let capability = PlatformsCapability {
        required: true,
        allowed: vec![Platform::Core, Platform::Ios, Platform::Android],
        default: vec![Platform::Core, Platform::Ios],
    };

    // Required + empty set: the violation carries the display defaults.
    let Err(PlatformsViolation::RequiredButMissing { defaults }) = capability.check(&[]) else {
        panic!("required capability must refuse an empty set");
    };
    assert_eq!(defaults, vec!["core".to_string(), "ios".to_string()]);

    // Non-empty set without core.
    assert_eq!(capability.check(&[Platform::Ios]), Err(PlatformsViolation::MissingCore));

    // A platform outside `allowed`.
    let Err(PlatformsViolation::NotAllowed { platform, allowed }) =
        capability.check(&[Platform::Core, Platform::Web])
    else {
        panic!("web must be outside the allowed set");
    };
    assert_eq!(platform, Platform::Web);
    assert_eq!(allowed, vec!["core".to_string(), "ios".to_string(), "android".to_string()]);

    // A conforming set passes; an optional capability allows empty.
    capability.check(&[Platform::Core, Platform::Ios]).expect("conforming set passes");
    let optional = PlatformsCapability {
        required: false,
        allowed: vec![Platform::Core],
        default: vec![Platform::Core],
    };
    optional.check(&[]).expect("optional capability allows an empty set");
}
