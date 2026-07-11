use std::ffi::OsString;
use std::fs;
use std::path::Path;

use error::Error;
use schema::cache::adapter_store_entry;

use super::*;

// Retained in `src`: these branches drive the private `AdapterUri::parse`
// and `parse_first_party_shorthand` — no CLI fixture reaches them short
// of a full `specify init`, and the store-resolve branches need a
// hermetic `SPECIFY_ADAPTER_STORE`. The public projections
// (`adapter_name_from_value`, `AdapterRef::from_value`,
// `recognize_package`) are covered in `tests/adapter_uri.rs`.

/// Restores the previous `SPECIFY_ADAPTER_STORE` value on drop.
struct StoreGuard(Option<OsString>);

impl Drop for StoreGuard {
    #[expect(unsafe_code, reason = "restore the store-root env var pinned for the test")]
    fn drop(&mut self) {
        // SAFETY: nextest runs each test in its own process, so no other
        // thread observes the env mutation for the guard's lifetime.
        unsafe {
            match self.0.take() {
                Some(prev) => std::env::set_var("SPECIFY_ADAPTER_STORE", prev),
                None => std::env::remove_var("SPECIFY_ADAPTER_STORE"),
            }
        }
    }
}

/// Pin the global adapter store root at `root` for the test's lifetime so
/// store reads resolve into a hermetic temp directory.
#[expect(unsafe_code, reason = "pin the store-root env var into the test tempdir")]
fn scoped_store(root: &Path) -> StoreGuard {
    let prev = std::env::var_os("SPECIFY_ADAPTER_STORE");
    // SAFETY: see `StoreGuard::drop` — single-process test isolation.
    unsafe { std::env::set_var("SPECIFY_ADAPTER_STORE", root) };
    StoreGuard(prev)
}

#[test]
fn github_uri_refused() {
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
fn package_ref_wire_value_round_trips() {
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
    let _guard = scoped_store(store.path());

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
    let _guard = scoped_store(store.path());

    let entry = adapter_store_entry("demo-target", "1.2.0");
    fs::create_dir_all(entry.parent().expect("store root")).expect("create store root");
    fs::write(&entry, b"\0asm").expect("write component");

    let parsed = AdapterUri::parse("specify:demo-target@1.2.0", Path::new("/tmp"))
        .expect("resolve from store");
    assert_eq!(parsed.adapter_name, "demo-target");
    assert_eq!(parsed.adapter_value, "specify:demo-target@1.2.0");
    assert_eq!(parsed.origin, AdapterOrigin::Store(entry));
}

#[test]
fn bare_name_defers_to_resolver() {
    // A bare development name is an identity, not a file: parse
    // demands no component (the injected `Resolver` locates one —
    // linked crates natively, the sibling release build in the shipped
    // path — and owns the `adapter-not-found` miss).
    let parsed = AdapterUri::parse("demo-target", Path::new("/tmp")).expect("bare name parses");
    assert_eq!(parsed.adapter_name, "demo-target");
    assert_eq!(parsed.adapter_value, "demo-target");
    assert_eq!(parsed.origin, AdapterOrigin::Dev);
}
