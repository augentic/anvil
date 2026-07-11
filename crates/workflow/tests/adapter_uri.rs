//! Integration coverage for the public `<adapter>` argument
//! projections (`workflow::init`): recorded-value identity recovery
//! and the package-reference recognition matrix. The private parse
//! branches (GitHub refusal, dev shorthand, store resolution) stay
//! with the `src` unit layer — they have no public entry point short
//! of a full `specify init`.

use workflow::adapter::AdapterRef;
use workflow::init::{adapter_name_from_value, recognize_package};

#[test]
fn value_identity() {
    // `adapter_name_from_value` extracts the kebab name across every shape.
    assert_eq!(adapter_name_from_value("demo-target"), "demo-target");
    assert_eq!(adapter_name_from_value("specify:demo-target@1.2.0"), "demo-target");
    assert_eq!(adapter_name_from_value("acme:demo-target@1.2.0"), "demo-target");
    assert_eq!(adapter_name_from_value("file:///abs/components/demo-target.wasm"), "demo-target");
    assert_eq!(
        adapter_name_from_value("file:///abs/release/specify_demo_target.wasm"),
        "demo-target"
    );
    assert_eq!(adapter_name_from_value("/abs/components/demo-target.wasm"), "demo-target");

    // `AdapterRef::from_value` recovers a semver pin; a bare name, a
    // `file://` path, and a non-semver suffix yield a bare ref; a package
    // reference recovers the bare `(name, version)` identity, stripping
    // `<namespace>:`.
    assert_eq!(AdapterRef::from_value("demo-target"), AdapterRef::bare("demo-target"));
    assert_eq!(
        AdapterRef::from_value("demo-target@1.0.0"),
        AdapterRef::pinned("demo-target", semver::Version::new(1, 0, 0))
    );
    assert_eq!(AdapterRef::from_value("demo-target@v1"), AdapterRef::bare("demo-target"));
    assert_eq!(
        AdapterRef::from_value("file:///abs/components/demo-target.wasm"),
        AdapterRef::bare("demo-target")
    );
    assert_eq!(
        AdapterRef::from_value("specify:demo-target@1.2.0"),
        AdapterRef::pinned("demo-target", semver::Version::new(1, 2, 0))
    );
}

#[test]
fn package_recognition() {
    // `recognize_package` exposes the `(namespace, name, version)`
    // identity of a valid package reference.
    let package = recognize_package("specify:demo-target@1.2.0")
        .expect("package shape")
        .expect("valid package reference");
    assert_eq!(package.namespace, "specify");
    assert_eq!(package.name, "demo-target");
    assert_eq!(package.version, semver::Version::new(1, 2, 0));

    // An immutable locator pins an exact SemVer version — a missing
    // version, a git-style tag, and `latest` are all rejected.
    for malformed in [
        "specify:demo-target",
        "specify:demo-target@v1",
        "specify:demo-target@1",
        "specify:demo-target@latest",
    ] {
        let result = recognize_package(malformed)
            .unwrap_or_else(|| panic!("`{malformed}` is a package-ref shape"));
        assert!(
            matches!(
                result,
                Err(error::Error::Diag {
                    code: "adapter-package-ref-version-required",
                    ..
                })
            ),
            "`{malformed}` must demand an exact SemVer pin",
        );
    }

    // URL schemes, drive paths, bare names, and local paths are not package
    // references — they keep flowing through the other branches.
    for non_package in [
        "demo-target",
        "./demo-target.wasm",
        "/abs/demo-target.wasm",
        "file:///abs/demo-target.wasm",
        r"C:\adapters\demo-target.wasm",
        "C:/adapters/demo-target.wasm",
    ] {
        assert!(
            recognize_package(non_package).is_none(),
            "`{non_package}` must not be treated as a package reference",
        );
    }

    // The versioned first-party shorthand is sugar for the `specify:`
    // package reference (specify: naming cut).
    let sugar = recognize_package("demo-target@1.0.0")
        .expect("versioned shorthand is a package shape")
        .expect("valid shorthand");
    assert_eq!(sugar.namespace, "specify");
    assert_eq!(sugar.name, "demo-target");
    assert_eq!(sugar.version, semver::Version::new(1, 0, 0));
}
