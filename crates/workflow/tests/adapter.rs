//! Integration tests for the adapter resolver
//! (`workflow::adapter`).
//!
//! Covers:
//! - pinned identities resolving the single-file store entry
//!   (`<store-root>/<name>@<version>.wasm`), verify-on-read included.
//! - bare names resolving the project component cache.
//! - metadata-driven resolution: floor gate, malformed floor, target
//!   inputs + platforms, and the digest-keyed metadata sidecar cache.
//!
//! Metadata dispatch is stubbed with a registered runner that parses
//! the component file's bytes as a JSON `Metadata` — each test
//! controls its adapter's answer by writing the fixture component.
//! (nextest runs each test in its own process, so the process-global
//! runner registration is per-test.)

use std::fs;
use std::path::Path;

use error::Error;
use workflow::adapter::metadata::metadata_cache_path;
use workflow::adapter::{
    AdapterRef, Resolver as _, SourceOperation, TargetOperation, component_cache_entry,
};

mod common;

/// Explicit component resolver over the shared metadata fixture.
fn resolver() -> workflow::adapter::resolver::Component {
    common::resolver()
}

/// Stage a store entry for `(name, version)` plus its verify-on-read
/// sidecar. The bytes are opaque to the explicit metadata runner.
fn stage_store_entry(name: &str, version: &str, answer: &str) -> std::path::PathBuf {
    let entry = schema::cache::adapter_store_entry(name, version);
    fs::create_dir_all(entry.parent().expect("store root")).expect("create store root");
    fs::write(&entry, answer).expect("write store component");
    let digest = schema::cache::file_content_digest(&entry);
    schema::cache::write_store_meta(name, version, &digest, None).expect("write sidecar");
    entry
}

#[test]
fn pinned_resolves_from_store() {
    let components = resolver();
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).expect("project dir");

    let version = semver::Version::new(2, 3, 4);
    let entry = stage_store_entry("typescript", "2.3.4", "{}");

    let resolved = components
        .resolve_source(&AdapterRef::pinned("typescript", version.clone()), &project)
        .expect("resolve pinned identity from the store");
    assert_eq!(resolved.manifest.name, "typescript");
    assert_eq!(resolved.manifest.version, version, "version comes from the package identity");
    assert_eq!(resolved.manifest.requires_specify, None);
    assert_eq!(resolved.origin.label, "store");
    assert_eq!(resolved.origin.reference, entry.display().to_string());
    assert_eq!(
        resolved.manifest.operations().copied().collect::<Vec<_>>(),
        vec![SourceOperation::Extract, SourceOperation::Survey],
        "operation set derives from the closed WIT contract"
    );

    // The describe answer is cached against the component digest as a
    // sidecar beside the entry.
    assert!(metadata_cache_path(&entry).is_file(), "metadata sidecar recorded beside the entry");
}

#[test]
fn metadata_cache_short_circuits() {
    let components = resolver();
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).expect("project dir");

    let entry = stage_store_entry("typescript", "1.0.0", "{}");
    let adapter_ref = AdapterRef::pinned("typescript", semver::Version::new(1, 0, 0));
    components.resolve_source(&adapter_ref, &project).expect("first resolve dispatches");

    // Rewrite the cached sidecar with a different answer under the same
    // digest: a second resolve must return the sidecar answer without
    // re-dispatching (the stub would have returned an empty answer).
    let sidecar = metadata_cache_path(&entry);
    let digest = schema::cache::file_content_digest(&entry);
    fs::write(
        &sidecar,
        format!(r#"{{ "digest": "{digest}", "metadata": {{ "specify-floor": "0.1.0" }} }}"#),
    )
    .expect("rewrite sidecar");

    let resolved = components.resolve_source(&adapter_ref, &project).expect("second resolve");
    assert_eq!(
        resolved.manifest.requires_specify,
        Some(semver::Version::new(0, 1, 0)),
        "digest-valid sidecar answer wins without a re-dispatch"
    );
}

#[test]
fn bare_resolves_from_component_cache() {
    let components = resolver();
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).expect("project dir");
    let _cache = common::scoped_cache(tmp.path());

    let entry = component_cache_entry(&project, "captures");
    fs::create_dir_all(entry.parent().expect("cache dir")).expect("create component cache");
    fs::write(&entry, "{}").expect("write cached component");

    let resolved = components
        .resolve_source(&AdapterRef::bare("captures"), &project)
        .expect("bare name resolves the project component cache");
    assert_eq!(resolved.manifest.name, "captures");
    assert_eq!(
        resolved.manifest.version,
        workflow::adapter::dev_version(),
        "a development artifact resolves as the 0.0.0 placeholder"
    );
    assert_eq!(resolved.origin.label, "dev");
    assert_eq!(resolved.origin.reference, entry.display().to_string());
}

#[test]
fn missing_adapter_reports_not_found() {
    let components = resolver();
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));
    let _cache = common::scoped_cache(tmp.path());
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).expect("project dir");

    for err in [
        components
            .resolve_source(&AdapterRef::bare("nonexistent"), &project)
            .expect_err("missing development artifact must fail"),
        components
            .resolve_source(
                &AdapterRef::pinned("nonexistent", semver::Version::new(1, 0, 0)),
                &project,
            )
            .expect_err("missing store entry must fail"),
    ] {
        let detail = err.to_string();
        assert!(
            matches!(
                err,
                Error::Diag {
                    code: "adapter-not-found",
                    ..
                }
            ),
            "{detail}"
        );
        assert!(detail.contains("nonexistent"), "error names the identity: {detail}");
    }
}

#[test]
fn store_entry_digest_mismatch_refused() {
    let components = resolver();
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).expect("project dir");

    // Record the sidecar, then mutate the entry bytes underneath it:
    // Verify-on-read must refuse the drifted artifact.
    let entry = stage_store_entry("typescript", "1.0.0", "{}");
    fs::write(&entry, r#"{"specify-floor":"0.1.0"}"#).expect("drift the entry");

    let err = components
        .resolve_source(&AdapterRef::pinned("typescript", semver::Version::new(1, 0, 0)), &project)
        .expect_err("drifted store entry must fail verify-on-read");
    let detail = err.to_string();
    assert!(
        matches!(
            err,
            Error::Diag {
                code: "adapter-digest-mismatch",
                ..
            }
        ),
        "{detail}"
    );
}

#[test]
fn floor_gate_from_metadata() {
    let components = resolver();
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).expect("project dir");

    // A floor above the running binary aborts on the exit-3 path,
    // naming the identity.
    stage_store_entry("demo-target", "1.0.0", r#"{"specify-floor":"999.0.0"}"#);
    let err = components
        .resolve_target(&AdapterRef::pinned("demo-target", semver::Version::new(1, 0, 0)), &project)
        .expect_err("a binary below the adapter floor must be rejected");
    assert_eq!(err.variant_str(), "adapter-cli-too-old");

    // A non-semver floor is the typed `adapter-floor-malformed`.
    stage_store_entry("bad-floor", "1.0.0", r#"{"specify-floor":"v1"}"#);
    let err = components
        .resolve_target(&AdapterRef::pinned("bad-floor", semver::Version::new(1, 0, 0)), &project)
        .expect_err("a non-semver floor must be rejected");
    let Error::Validation { code, .. } = err else {
        panic!("expected Error::Validation, got: {err:?}");
    };
    assert_eq!(code, "adapter-floor-malformed");
}

#[test]
fn target_metadata_from_metadata_answer() {
    let components = resolver();
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).expect("project dir");

    stage_store_entry(
        "vectis",
        "1.0.4",
        r#"{
            "inputs": [
                { "path": "tokens.yaml", "required": true },
                { "path": "assets.yaml", "required": false }
            ],
            "platforms": {
                "required": true,
                "allowed": ["core", "ios", "android"],
                "default": ["core", "ios", "android"]
            }
        }"#,
    );

    let resolved = components
        .resolve_target(&AdapterRef::pinned("vectis", semver::Version::new(1, 0, 4)), &project)
        .expect("target adapter resolves with metadata");
    assert_eq!(resolved.manifest.inputs.len(), 2, "both declared inputs survive");
    assert_eq!(resolved.manifest.inputs[0].path, "tokens.yaml");
    assert!(resolved.manifest.inputs[0].required);
    assert!(!resolved.manifest.inputs[1].required);
    let platforms = resolved.manifest.platforms.as_ref().expect("platforms capability present");
    assert!(platforms.required);
    assert_eq!(platforms.allowed.len(), 3);
    assert_eq!(
        resolved.manifest.operations().copied().collect::<Vec<_>>(),
        vec![TargetOperation::Build, TargetOperation::Guidance, TargetOperation::Merge],
        "operation set derives from the closed WIT contract"
    );

    // A source answer never carries the target-only fields; a target
    // resolve over an empty answer defaults them.
    stage_store_entry("omnia", "1.0.0", "{}");
    let resolved = components
        .resolve_target(&AdapterRef::pinned("omnia", semver::Version::new(1, 0, 0)), &project)
        .expect("target adapter with empty metadata answer resolves");
    assert!(resolved.manifest.inputs.is_empty(), "absent inputs default to empty");
    assert!(resolved.manifest.platforms.is_none(), "absent platforms default to None");
}

#[test]
fn dev_component_paths_shape() {
    // The development probe: `target/wasm32-wasip2/release/<name>.wasm`
    // under the project, then the sibling `specify-adapters` checkout.
    let project = Path::new("/repos/consumer");
    let paths = workflow::adapter::dev_component_paths(project, "demo-target");
    assert_eq!(
        paths[0],
        Path::new("/repos/consumer/target/wasm32-wasip2/release/demo_target.wasm")
    );
    assert_eq!(
        paths[1],
        Path::new("/repos/specify-adapters/target/wasm32-wasip2/release/demo_target.wasm")
    );
}

// ---------------------------------------------------------------------------
// Identity model and platform capability (moved from the src unit layer)
// ---------------------------------------------------------------------------

#[test]
fn axis_routing() {
    use workflow::adapter::{Axis, SourceAdapter, TargetAdapter, dev_version};

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
fn platforms_capability_check() {
    use workflow::Platform;
    use workflow::adapter::{PlatformsCapability, PlatformsViolation};

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
