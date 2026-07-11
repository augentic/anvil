use error::Error;

use super::*;

// Retained in `src`: `parse_floor` / `check_requires_specify` are
// `pub(super)` post-resolve gates — identity comes from the package
// reference and metadata from the component's `metadata` answer, so no
// integration boundary reaches these branches without staging a full
// resolve. The identity/axis/platform matrices moved to
// `crates/workflow/tests/adapter.rs`.

#[test]
fn typed_gates() {
    let origin = Origin {
        label: "store".to_string(),
        reference: "/store/demo@1.0.0.wasm".to_string(),
    };

    // The `specify-floor` string from a describe answer parses into a
    // typed semver; a non-semver floor is `adapter-floor-malformed`;
    // an absent floor is `None`.
    assert_eq!(parse_floor(None, "demo", &origin).expect("absent floor passes"), None);
    assert_eq!(
        parse_floor(Some("0.28.0"), "demo", &origin).expect("exact semver parses"),
        Some(semver::Version::new(0, 28, 0))
    );
    let Error::Validation { code, .. } =
        parse_floor(Some("1"), "demo", &origin).expect_err("integer-shaped floor must be rejected")
    else {
        panic!("expected Error::Validation");
    };
    assert_eq!(code, "adapter-floor-malformed");

    // An absent floor never gates; a binary at/above the floor
    // passes; below the floor aborts on the exit-3 path; an unparseable
    // current version is permissive (mirrors config::version_is_older).
    check_requires_specify(None, "0.1.0", "demo-source", &origin)
        .expect("absent floor never gates");
    let floor = semver::Version::new(0, 28, 0);
    check_requires_specify(Some(&floor), "0.28.0", "demo-target", &origin)
        .expect("exact floor passes");
    check_requires_specify(Some(&floor), "0.29.1", "demo-target", &origin).expect("newer passes");
    let two = semver::Version::new(2, 0, 0);
    let err = check_requires_specify(Some(&two), "1.5.0", "demo-target", &origin)
        .expect_err("a binary below the floor must be rejected");
    assert_eq!(err.variant_str(), "adapter-cli-too-old");
    let Error::AdapterCliTooOld { required, found, .. } = err else {
        panic!("expected Error::AdapterCliTooOld, got: {err:?}");
    };
    assert_eq!(required, "2.0.0");
    assert_eq!(found, "1.5.0");
    check_requires_specify(Some(&two), "not-a-version", "demo-target", &origin)
        .expect("unparseable current version is permissive");
}
