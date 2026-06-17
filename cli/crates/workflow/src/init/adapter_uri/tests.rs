use super::*;

#[test]
fn github_adapter_uri_parses_default_main() {
    let parsed = GithubAdapterUri::parse("https://github.com/owner/repo/schemas/omnia")
        .expect("parse GitHub URI");
    assert_eq!(
        parsed,
        GithubAdapterUri {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            checkout_ref: None,
            adapter_path: "schemas/omnia".to_string(),
            adapter_name: "omnia".to_string(),
        }
    );
}

#[test]
fn github_adapter_uri_parses_suffix_ref() {
    let parsed = GithubAdapterUri::parse("https://github.com/owner/repo/schemas/omnia@v1")
        .expect("parse GitHub URI");
    assert_eq!(parsed.checkout_ref.as_deref(), Some("v1"));
    assert_eq!(parsed.adapter_path, "schemas/omnia");
    assert_eq!(parsed.adapter_name, "omnia");
}

#[test]
fn github_adapter_uri_parses_tree_ref() {
    let parsed = GithubAdapterUri::parse("https://github.com/owner/repo/tree/main/schemas/omnia")
        .expect("parse GitHub URI");
    assert_eq!(parsed.checkout_ref.as_deref(), Some("main"));
    assert_eq!(parsed.adapter_path, "schemas/omnia");
    assert_eq!(parsed.adapter_name, "omnia");
}

#[test]
fn name_from_value_handles_shapes() {
    assert_eq!(adapter_name_from_value("omnia"), "omnia");
    assert_eq!(adapter_name_from_value("specify:omnia@1.2.0"), "omnia");
    assert_eq!(adapter_name_from_value("file:///abs/adapters/targets/omnia"), "omnia");
    assert_eq!(adapter_name_from_value("file:///abs/adapters/targets/omnia/"), "omnia");
    assert_eq!(
        adapter_name_from_value("https://github.com/augentic/specify/adapters/targets/omnia"),
        "omnia"
    );
    assert_eq!(
        adapter_name_from_value("https://github.com/augentic/specify/adapters/targets/omnia@v1"),
        "omnia"
    );
    assert_eq!(adapter_name_from_value("/abs/targets/omnia"), "omnia");
}

#[test]
fn package_ref_parses_namespace_semver() {
    let parsed = AdapterPackageRef::recognize("specify:omnia@1.2.0")
        .expect("recognised as a package reference")
        .expect("valid package reference");
    assert_eq!(
        parsed,
        AdapterPackageRef {
            namespace: "specify".to_string(),
            name: "omnia".to_string(),
            version: semver::Version::new(1, 2, 0),
        }
    );
    assert_eq!(parsed.wire_value(), "specify:omnia@1.2.0");
}

#[test]
fn package_ref_requires_exact_semver() {
    // RFC-48 D2: an immutable locator pins an exact SemVer version. A
    // missing version, a git-style tag, or `latest` are all rejected —
    // there is no branch or tag defaulting.
    for malformed in
        ["specify:omnia", "specify:omnia@v1", "specify:omnia@1", "specify:omnia@latest"]
    {
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
}

#[test]
fn package_ref_rejects_non_package() {
    // URL schemes, drive paths, bare names, and local paths are not
    // package references — they keep flowing through the other branches.
    for non_package in [
        "omnia",
        "omnia@1.0.0",
        "./omnia",
        "/abs/omnia",
        "file:///abs/omnia",
        "https://github.com/augentic/specify/adapters/targets/omnia",
        r"C:\adapters\omnia",
        "C:/adapters/omnia",
    ] {
        assert!(
            AdapterPackageRef::recognize(non_package).is_none(),
            "`{non_package}` must not be treated as a package reference",
        );
    }
}

#[test]
fn package_ref_uninstalled_is_not_installed() {
    // A package reference resolves only from the global content-addressed
    // store; absent an installed entry (the root `specify init` layer
    // installs before scaffolding), it is `adapter-package-not-installed`
    // rather than a silent fallback to a mutable git checkout or a local
    // path.
    let store = tempfile::tempdir().expect("store root");
    let _guard = crate::test_cache::scoped_store(store.path());

    let err = AdapterUri::parse("specify:omnia@1.2.0", Path::new("/tmp"))
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
    // package reference resolves it as a local source and records the
    // canonical wire value as `adapter_value`.
    let store = tempfile::tempdir().expect("store root");
    let _guard = crate::test_cache::scoped_store(store.path());

    let entry = adapter_store_entry("omnia", "1.2.0");
    fs::create_dir_all(&entry).expect("create store entry");
    fs::write(entry.join(crate::adapter::ADAPTER_FILENAME), "name: omnia\n")
        .expect("write manifest");

    let parsed =
        AdapterUri::parse("specify:omnia@1.2.0", Path::new("/tmp")).expect("resolve from store");
    assert_eq!(parsed.adapter_name, "omnia");
    assert_eq!(parsed.adapter_value, "specify:omnia@1.2.0");
    assert_eq!(parsed.source_dir, entry);
}

#[test]
fn recognize_package_projects_identity() {
    // The public projection exposes the `(namespace, name, version)` the
    // root install layer derives an OCI reference + store key from.
    let package = recognize_package("specify:omnia@1.2.0")
        .expect("package shape")
        .expect("valid package reference");
    assert_eq!(package.namespace, "specify");
    assert_eq!(package.name, "omnia");
    assert_eq!(package.version, semver::Version::new(1, 2, 0));
    // Non-package shapes are passed through (None) for the git / local
    // branches; a malformed pin surfaces as an error.
    assert!(recognize_package("omnia").is_none());
    assert!(recognize_package("./local").is_none());
    recognize_package("specify:omnia").expect("package shape").unwrap_err();
}

#[test]
fn shorthand_splits_name_and_semver_pin() {
    // A bare name carries no pin (resolves the single installed
    // identity); a `name@<semver>` carries the RFC-47 version pin.
    assert_eq!(parse_first_party_shorthand("omnia"), Some(("omnia", None)));
    assert_eq!(
        parse_first_party_shorthand("omnia@1.0.0"),
        Some(("omnia", Some(semver::Version::new(1, 0, 0))))
    );
    assert_eq!(parse_first_party_shorthand("typescript"), Some(("typescript", None)));
    assert_eq!(
        parse_first_party_shorthand("typescript@2.3.1"),
        Some(("typescript", Some(semver::Version::new(2, 3, 1))))
    );
}

#[test]
fn first_party_repo_routes_extracted() {
    // Bundled adapters (a WASI extension) have extracted to
    // specify-adapters; prose-only adapters still resolve from the
    // platform repo during the topology transition (RFC-48 / RFC-49).
    assert_eq!(first_party_repo("contracts"), "specify-adapters");
    assert_eq!(first_party_repo("vectis"), "specify-adapters");
    assert_eq!(first_party_repo("omnia"), "specify");
    assert_eq!(first_party_repo("typescript"), "specify");
}

#[test]
fn shorthand_rejects_non_shorthand() {
    // Paths and URLs flow through from_local / from_github instead.
    assert_eq!(parse_first_party_shorthand("./omnia"), None);
    assert_eq!(parse_first_party_shorthand("/abs/omnia"), None);
    assert_eq!(parse_first_party_shorthand("file:///abs/omnia"), None);
    assert_eq!(
        parse_first_party_shorthand("https://github.com/augentic/specify/adapters/targets/omnia"),
        None
    );
    // Not kebab-case, or a `@suffix` that is not exact semver.
    assert_eq!(parse_first_party_shorthand("Omnia"), None);
    assert_eq!(parse_first_party_shorthand("-omnia"), None);
    assert_eq!(parse_first_party_shorthand("omnia@v1"), None);
    assert_eq!(parse_first_party_shorthand("omnia@1"), None);
    assert_eq!(parse_first_party_shorthand("omnia@latest"), None);
    assert_eq!(parse_first_party_shorthand("omnia@"), None);
    assert_eq!(parse_first_party_shorthand(""), None);
}

#[test]
fn ref_from_value_recovers_semver_pin() {
    // A semver `@suffix` is recovered as a version pin; a bare name,
    // a `file://` path, and a non-semver git ref all yield `None`.
    assert_eq!(adapter_ref_from_value("omnia"), AdapterRef::bare("omnia"));
    assert_eq!(
        adapter_ref_from_value("omnia@1.0.0"),
        AdapterRef::pinned("omnia", semver::Version::new(1, 0, 0))
    );
    assert_eq!(adapter_ref_from_value("omnia@v1"), AdapterRef::bare("omnia"));
    assert_eq!(
        adapter_ref_from_value("file:///abs/adapters/targets/omnia"),
        AdapterRef::bare("omnia")
    );
    // A package reference recovers the bare `(name, version)` identity,
    // stripping the `<namespace>:` prefix.
    assert_eq!(
        adapter_ref_from_value("specify:omnia@1.2.0"),
        AdapterRef::pinned("omnia", semver::Version::new(1, 2, 0))
    );
}

#[test]
#[ignore = "networked GitHub fetch smoke test"]
fn shorthand_resolves_via_github() {
    // The shorthand resolves the canonical published first-party
    // adapter (a real sparse checkout of augentic/specify@v1).
    // Networked — run with `--ignored`.
    let parsed = AdapterUri::from_shorthand("omnia", Some(&semver::Version::new(1, 0, 0)))
        .expect("resolve shorthand against the published GitHub adapter");
    assert_eq!(parsed.adapter_name, "omnia");
}
