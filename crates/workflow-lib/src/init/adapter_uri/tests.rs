use std::fs;
use std::path::Path;

use specify_error::Error;
use specify_schema::cache::adapter_store_entry;

use super::*;

// Pure parse/identity matrices for the `<adapter>` argument shapes
// (package references, first-party shorthand, value identity), plus the
// store-resolve branches driven against a real content-addressed store.
// GitHub URIs are a refusal branch, covered inline.

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

    // `adapter_ref_from_value` recovers a semver pin; a bare name, a
    // `file://` path, and a non-semver suffix yield a bare ref; a package
    // reference recovers the bare `(name, version)` identity, stripping
    // `<namespace>:`.
    assert_eq!(adapter_ref_from_value("demo-target"), AdapterRef::bare("demo-target"));
    assert_eq!(
        adapter_ref_from_value("demo-target@1.0.0"),
        AdapterRef::pinned("demo-target", semver::Version::new(1, 0, 0))
    );
    assert_eq!(adapter_ref_from_value("demo-target@v1"), AdapterRef::bare("demo-target"));
    assert_eq!(
        adapter_ref_from_value("file:///abs/components/demo-target.wasm"),
        AdapterRef::bare("demo-target")
    );
    assert_eq!(
        adapter_ref_from_value("specify:demo-target@1.2.0"),
        AdapterRef::pinned("demo-target", semver::Version::new(1, 2, 0))
    );

    // GitHub URIs are refused with a typed error: a source checkout no
    // longer yields a usable adapter artifact.
    let err = AdapterUri::parse(
        "https://github.com/augentic/specify/adapters/targets/demo-target",
        Path::new("/tmp"),
    )
    .expect_err("GitHub adapter URIs must be refused");
    assert!(matches!(
        err,
        Error::Diag {
            code: "adapter-github-uri-unsupported",
            ..
        }
    ));
}

#[test]
fn package_refs() {
    // `recognize` parses `<namespace>:<name>@<semver>` and round-trips `wire_value`.
    let parsed = AdapterPackageRef::recognize("specify:demo-target@1.2.0")
        .expect("recognised as a package reference")
        .expect("valid package reference");
    assert_eq!(
        parsed,
        AdapterPackageRef {
            namespace: "specify".to_string(),
            name: "demo-target".to_string(),
            version: semver::Version::new(1, 2, 0),
        }
    );
    assert_eq!(parsed.wire_value(), "specify:demo-target@1.2.0");

    // An immutable locator pins an exact SemVer version — a missing
    // version, a git-style tag, and `latest` are all rejected.
    for malformed in [
        "specify:demo-target",
        "specify:demo-target@v1",
        "specify:demo-target@1",
        "specify:demo-target@latest",
    ] {
        let result = AdapterPackageRef::recognize(malformed)
            .unwrap_or_else(|| panic!("`{malformed}` is a package-ref shape"));
        assert!(
            matches!(
                result,
                Err(Error::Diag {
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
        "demo-target@1.0.0",
        "./demo-target.wasm",
        "/abs/demo-target.wasm",
        "file:///abs/demo-target.wasm",
        r"C:\adapters\demo-target.wasm",
        "C:/adapters/demo-target.wasm",
    ] {
        assert!(
            AdapterPackageRef::recognize(non_package).is_none(),
            "`{non_package}` must not be treated as a package reference",
        );
    }

    // The public `recognize_package` projection exposes `(namespace, name,
    // version)`; non-package shapes pass through (None); a malformed pin errors.
    let package = recognize_package("specify:demo-target@1.2.0")
        .expect("package shape")
        .expect("valid package reference");
    assert_eq!(package.namespace, "specify");
    assert_eq!(package.name, "demo-target");
    assert_eq!(package.version, semver::Version::new(1, 2, 0));
    assert!(recognize_package("demo-target").is_none());
    assert!(recognize_package("./local.wasm").is_none());
    recognize_package("specify:demo-target").expect("package shape").unwrap_err();

    // The versioned first-party shorthand is sugar for the `specify:`
    // package reference (specify: naming cut).
    let sugar = recognize_package("demo-target@1.0.0")
        .expect("versioned shorthand is a package shape")
        .expect("valid shorthand");
    assert_eq!(sugar.namespace, "specify");
    assert_eq!(sugar.name, "demo-target");
    assert_eq!(sugar.version, semver::Version::new(1, 0, 0));
}

#[test]
fn first_party_shorthand() {
    // A bare name carries no pin (resolves the development release build); a
    // `name@<semver>` carries the version pin.
    assert_eq!(parse_first_party_shorthand("demo-target"), Some(("demo-target", None)));
    assert_eq!(
        parse_first_party_shorthand("demo-target@1.0.0"),
        Some(("demo-target", Some(semver::Version::new(1, 0, 0))))
    );
    assert_eq!(parse_first_party_shorthand("demo-source"), Some(("demo-source", None)));
    assert_eq!(
        parse_first_party_shorthand("demo-source@2.3.1"),
        Some(("demo-source", Some(semver::Version::new(2, 3, 1))))
    );

    // Paths flow through from_local; a non-kebab name or a `@suffix` that is
    // not exact semver is not shorthand.
    assert_eq!(parse_first_party_shorthand("./demo-target"), None);
    assert_eq!(parse_first_party_shorthand("/abs/demo-target"), None);
    assert_eq!(parse_first_party_shorthand("file:///abs/demo-target"), None);
    assert_eq!(
        parse_first_party_shorthand(
            "https://github.com/augentic/specify/adapters/targets/demo-target"
        ),
        None
    );
    assert_eq!(parse_first_party_shorthand("Demo-target"), None);
    assert_eq!(parse_first_party_shorthand("-demo-target"), None);
    assert_eq!(parse_first_party_shorthand("demo-target@v1"), None);
    assert_eq!(parse_first_party_shorthand("demo-target@1"), None);
    assert_eq!(parse_first_party_shorthand("demo-target@latest"), None);
    assert_eq!(parse_first_party_shorthand("demo-target@"), None);
    assert_eq!(parse_first_party_shorthand(""), None);
}

#[test]
fn package_ref_uninstalled_is_not_installed() {
    // A package reference resolves only from the global content-addressed
    // store; absent an installed entry (the root `specify init` layer
    // installs before scaffolding), it is `adapter-package-not-installed`
    // rather than a silent fallback to a mutable checkout or a local
    // path. Kept: no CLI fixture seeds an empty store for this branch.
    let store = tempfile::tempdir().expect("store root");
    let _guard = crate::test_cache::scoped_store(store.path());

    let err = AdapterUri::parse("specify:demo-target@1.2.0", Path::new("/tmp"))
        .expect_err("uninstalled package reference must not resolve");
    assert!(matches!(
        err,
        Error::Diag {
            code: "adapter-package-not-installed",
            ..
        }
    ));
}

#[test]
fn package_ref_resolves_from_store_entry() {
    // With the pinned `(name, version)` present in the global store, the
    // package reference resolves the single-file component entry and records
    // the canonical wire value as `adapter_value`. Kept: store-resolve has no
    // in-loop CLI fixture.
    let store = tempfile::tempdir().expect("store root");
    let _guard = crate::test_cache::scoped_store(store.path());

    let entry = adapter_store_entry("demo-target", "1.2.0");
    fs::create_dir_all(entry.parent().expect("store root")).expect("create store root");
    fs::write(&entry, b"\0asm").expect("write component");

    let parsed = AdapterUri::parse("specify:demo-target@1.2.0", Path::new("/tmp"))
        .expect("resolve from store");
    assert_eq!(parsed.adapter_name, "demo-target");
    assert_eq!(parsed.adapter_value, "specify:demo-target@1.2.0");
    assert_eq!(parsed.component, entry);
    assert_eq!(parsed.origin, AdapterOrigin::Store);
}
