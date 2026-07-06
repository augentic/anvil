//! Integration tests for the the axis-aware adapter loader
//! (`specify_workflow::adapter`).
//!
//! Covers:
//! - axis routing — `(source, foo)` and `(target, foo)` resolve to
//!   distinct manifests even when the directory names collide.
//! - cache-vs-local probe order — the agent-populated manifest cache
//!   wins.
//! - cache placement — a load of `(source, …)` populates the out-of-tree
//!   `<project-cache>/manifests/sources/<name>/`; `(target, …)`
//!   mirrors under `manifests/targets/`.
//! - schema validation — both the shared shape and the axis-specific
//!   refinements (axis literal, retired old-stack keys) reject
//!   hand-rolled inputs.

use std::fs;
use std::path::{Path, PathBuf};

use specify_error::Error;
use specify_workflow::adapter::{
    AdapterLocation, AdapterRef, Axis, SourceAdapter, SourceOperation, TargetAdapter,
    TargetOperation, cache_dir, check_axis_unique_for_name,
};

use crate::common;

fn fixtures_root() -> PathBuf {
    // `crates/workflow/tests/` -> `tests/fixtures/plugins/`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plugins")
}

/// Build a temporary project layout by copying the in-tree fixture
/// directory into a fresh tempdir. The resulting `project_dir` carries
/// `sources/` and `targets/` (local axis) but no manifest-cache
/// entries — cache fixtures are populated by individual tests below.
fn local_project() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().to_path_buf();
    common::copy_dir(&fixtures_root(), &project);
    (tmp, project)
}

#[test]
fn resolves_source_from_local_dir() {
    let (_tmp, project) = local_project();
    let resolved = SourceAdapter::resolve(&AdapterRef::bare("typescript"), &project)
        .expect("resolve source adapter from adapters/sources/<name>/adapter.yaml");
    assert_eq!(resolved.manifest.name, "typescript");
    assert_eq!(resolved.manifest.axis, Axis::Source);
    assert_eq!(
        resolved.manifest.operations().copied().collect::<Vec<_>>(),
        vec![SourceOperation::Extract, SourceOperation::Survey]
    );
    assert!(matches!(resolved.location, AdapterLocation::Local(_)));
    assert!(resolved.location.path().ends_with("adapters/sources/typescript"));
}

#[test]
fn resolves_target_from_local_dir() {
    let (_tmp, project) = local_project();
    let resolved = TargetAdapter::resolve(&AdapterRef::bare("omnia"), &project)
        .expect("resolve target adapter from adapters/targets/<name>/adapter.yaml");
    assert_eq!(resolved.manifest.name, "omnia");
    assert_eq!(resolved.manifest.axis, Axis::Target);
    // `operations()` yields the closed WIT set in ascending kebab-name
    // order: build < merge < shape.
    assert_eq!(
        resolved.manifest.operations().copied().collect::<Vec<_>>(),
        vec![TargetOperation::Build, TargetOperation::Merge, TargetOperation::Shape]
    );
    assert!(resolved.location.path().ends_with("adapters/targets/omnia"));
}

#[test]
fn resolves_shrunk_source_manifest() {
    // The post-cutover manifest carries only `name` / `version` /
    // `axis` / `description`. It must resolve, and the operation set
    // derives from the closed WIT contract.
    let (_tmp, project) = local_project();
    let manifest_dir = project.join("adapters").join("sources").join("shrunk");
    fs::create_dir_all(&manifest_dir).expect("create shrunk source dir");
    fs::write(
        manifest_dir.join("adapter.yaml"),
        r"name: shrunk
version: 1.0.0
axis: source
description: Shrunk post-cutover source manifest.
",
    )
    .expect("write shrunk source manifest");

    let resolved = SourceAdapter::resolve(&AdapterRef::bare("shrunk"), &project)
        .expect("shrunk source manifest resolves");
    assert_eq!(
        resolved.manifest.operations().copied().collect::<Vec<_>>(),
        vec![SourceOperation::Extract, SourceOperation::Survey],
        "operation set derives from the closed WIT contract"
    );
}

#[test]
fn resolves_shrunk_target_manifest() {
    // Target-axis counterpart, keeping the optional `platforms`
    // capability the shrunk vectis manifest retains.
    let (_tmp, project) = local_project();
    let manifest_dir = project.join("adapters").join("targets").join("shrunk-target");
    fs::create_dir_all(&manifest_dir).expect("create shrunk target dir");
    fs::write(
        manifest_dir.join("adapter.yaml"),
        r"name: shrunk-target
version: 1.0.0
axis: target
description: Shrunk post-cutover target manifest.
platforms:
  required: true
  allowed: [core, ios, android]
  default: [core, ios, android]
",
    )
    .expect("write shrunk target manifest");

    let resolved = TargetAdapter::resolve(&AdapterRef::bare("shrunk-target"), &project)
        .expect("shrunk target manifest resolves");
    assert!(resolved.manifest.platforms.is_some(), "retained platforms capability survives");
    assert_eq!(
        resolved.manifest.operations().copied().collect::<Vec<_>>(),
        vec![TargetOperation::Build, TargetOperation::Merge, TargetOperation::Shape],
        "operation set derives from the closed WIT contract"
    );
}

#[test]
fn retired_manifest_keys_rejected_at_load() {
    // The S4 schema close: the old-stack `briefs` / `execution` /
    // `extension` / `prepare` / hook keys are no longer legal manifest
    // properties on either axis.
    let (_tmp, project) = local_project();
    let source_cases = [
        ("with-briefs", "briefs:\n  survey: briefs/survey.md\n  extract: briefs/extract.md"),
        ("with-execution", "execution: agent"),
        ("with-extension", "extension:\n  name: demo-tool"),
    ];
    for (name, block) in source_cases {
        let manifest_dir = project.join("adapters").join("sources").join(name);
        fs::create_dir_all(&manifest_dir).expect("create retired-key source dir");
        fs::write(
            manifest_dir.join("adapter.yaml"),
            format!(
                "name: {name}\nversion: 1.0.0\naxis: source\ndescription: Retired key.\n{block}\n"
            ),
        )
        .expect("write retired-key source manifest");
        let err = SourceAdapter::resolve(&AdapterRef::bare(name), &project)
            .expect_err("a retired manifest key must fail");
        let detail = err.to_string();
        assert!(
            detail.contains("adapter-schema-violation"),
            "expected schema violation for `{name}`: {detail}"
        );
    }

    let target_cases = [
        ("with-prepare", "prepare:\n  argv: [prepare, build]"),
        ("with-hooks", "host_prereq:\n  script: scripts/host-prereq.sh"),
        ("with-catalog", "catalog:\n  infer: true"),
    ];
    for (name, block) in target_cases {
        let manifest_dir = project.join("adapters").join("targets").join(name);
        fs::create_dir_all(&manifest_dir).expect("create retired-key target dir");
        fs::write(
            manifest_dir.join("adapter.yaml"),
            format!(
                "name: {name}\nversion: 1.0.0\naxis: target\ndescription: Retired key.\n{block}\n"
            ),
        )
        .expect("write retired-key target manifest");
        let err = TargetAdapter::resolve(&AdapterRef::bare(name), &project)
            .expect_err("a retired manifest key must fail");
        let detail = err.to_string();
        assert!(
            detail.contains("adapter-schema-violation"),
            "expected schema violation for `{name}`: {detail}"
        );
    }
}

#[test]
fn axis_collision_rejected_at_resolve_time() {
    // Both `adapters/sources/foo/` and `adapters/targets/foo/` exist
    // in the fixture. Per DECISIONS.md §"Adapter name uniqueness"
    // the loader must reject this configuration on either axis with
    // the kebab-case `adapter-name-axis-collision` discriminant.
    let (_tmp, project) = local_project();
    for err in [
        SourceAdapter::resolve(&AdapterRef::bare("foo"), &project)
            .expect_err("source-axis resolve must reject the collision"),
        TargetAdapter::resolve(&AdapterRef::bare("foo"), &project)
            .expect_err("target-axis resolve must reject the collision"),
    ] {
        let Error::Validation { code, detail } = err else {
            panic!("expected Error::Validation, got: {err:?}");
        };
        assert_eq!(code, "adapter-name-axis-collision");
        assert!(
            detail.contains("adapters/sources/") && detail.contains("adapters/targets/"),
            "error body must name both axes, got: {detail}"
        );
    }
}

#[test]
fn axis_unique_passes_distinct() {
    // The fixture declares `typescript` only on the source axis
    // and `omnia` only on the target axis. Installing each on its
    // declared axis (or any brand-new name on either axis) must not
    // collide.
    let (_tmp, project) = local_project();
    check_axis_unique_for_name(Axis::Source, "typescript", &project)
        .expect("source-only adapter name is unique on the source axis");
    check_axis_unique_for_name(Axis::Target, "omnia", &project)
        .expect("target-only adapter name is unique on the target axis");
    check_axis_unique_for_name(Axis::Source, "brand-new-name", &project)
        .expect("absent adapter name is unique on the source axis");
    check_axis_unique_for_name(Axis::Target, "brand-new-name", &project)
        .expect("absent adapter name is unique on the target axis");
}

#[test]
fn axis_unique_rejects_opposite_axis() {
    // The init-time helper for the cross-axis uniqueness invariant.
    // Asking to install `foo` on either axis must fail because the
    // fixture already declares `foo` on both.
    let (_tmp, project) = local_project();
    for axis in [Axis::Source, Axis::Target] {
        let err = check_axis_unique_for_name(axis, "foo", &project)
            .expect_err("colliding adapter name must fail");
        let Error::Validation { code, detail } = err else {
            panic!("expected Error::Validation, got: {err:?}");
        };
        assert_eq!(code, "adapter-name-axis-collision");
        assert!(
            detail.contains("adapters/sources/") && detail.contains("adapters/targets/"),
            "error body must name both axes, got: {detail}"
        );
    }
}

#[test]
fn cache_dir_resolves_under_axis_segment() {
    // The manifest mirror is regenerable state that lives out-of-tree
    // under the per-project OS cache; `cache_dir` routes
    // `manifests/<axis>/<name>` beneath that root.
    let project = Path::new("/proj");
    let base = specify_workflow::config::Layout::new(project).cache_dir();
    assert_eq!(
        cache_dir(project, Axis::Source, "documentation"),
        base.join("manifests/sources/documentation"),
        "per-axis manifest cache root for source adapters lives under <cache>/manifests/sources/",
    );
    assert_eq!(
        cache_dir(project, Axis::Target, "omnia"),
        base.join("manifests/targets/omnia"),
        "per-axis manifest cache root for target adapters lives under <cache>/manifests/targets/",
    );
}

#[test]
fn cache_wins_over_local() {
    // Stage a manifest under the out-of-tree `<project-cache>/manifests/sources/typescript/`
    // alongside the in-tree `adapters/sources/typescript/`; assert the
    // cached copy wins per workflow §Resolver and cache.
    let (_tmp, project) = local_project();
    let _cache = common::scoped_cache(&project);
    let cached_root = cache_dir(&project, Axis::Source, "typescript");
    fs::create_dir_all(&cached_root).expect("create cache dir");
    fs::write(
        cached_root.join("adapter.yaml"),
        r"name: typescript
version: 7.0.0
axis: source
description: Cached source adapter fixture.
",
    )
    .expect("stage cache manifest");

    let resolved = SourceAdapter::resolve(&AdapterRef::bare("typescript"), &project)
        .expect("resolve from cache");
    assert_eq!(resolved.manifest.version, semver::Version::new(7, 0, 0), "cache wins over local");
    assert!(matches!(resolved.location, AdapterLocation::Cached(_)));
}

#[test]
fn pinned_resolves_from_store() {
    // RFC-48 D5: a pinned `(name, version)` resolves first against the
    // global content-addressed store entry `<store-root>/<name>@<version>/`,
    // ahead of both the manifest cache and the in-repo tree.
    let tmp = tempfile::tempdir().expect("tempdir");
    let store_root = tmp.path().join("store");
    fs::create_dir_all(&store_root).expect("create store root");
    let _store = common::scoped_store(&store_root);

    let version = semver::Version::new(2, 3, 4);
    let entry = store_root.join(format!("typescript@{version}"));
    fs::create_dir_all(&entry).expect("create store entry");
    fs::write(
        entry.join("adapter.yaml"),
        r"name: typescript
version: 2.3.4
axis: source
description: Store-resident source adapter fixture.
",
    )
    .expect("stage store manifest");

    let (_tmp, project) = local_project();
    let resolved =
        SourceAdapter::resolve(&AdapterRef::pinned("typescript", version.clone()), &project)
            .expect("resolve from store");
    assert_eq!(resolved.manifest.version, version, "pinned store entry wins over in-repo local");
    assert!(matches!(resolved.location, AdapterLocation::Store(_)));
}

#[test]
fn missing_adapter_reports_not_found() {
    let (_tmp, project) = local_project();
    let err = SourceAdapter::resolve(&AdapterRef::bare("nonexistent"), &project)
        .expect_err("missing adapter must fail");
    let detail = err.to_string();
    assert!(detail.contains("adapter-not-found"), "{detail}");
}

#[test]
fn resolves_captures_manifest() {
    // workflow §Acceptance scenario #26-1 (release blocker, D1): pin
    // the loader against the live `adapters/sources/captures/` adapter
    // shape — the shrunk post-cutover manifest with a free-form
    // `description:`.
    let (_tmp, project) = local_project();
    let manifest_dir = project.join("adapters").join("sources").join("captures");
    fs::create_dir_all(&manifest_dir).expect("create captures adapter dir");
    fs::write(
        manifest_dir.join("adapter.yaml"),
        r"name: captures
version: 1.0.0
axis: source
description: >-
  Runtime capture source adapter. Walks a read-only capture tree under
  `$SOURCE_DIR` and emits one lead per observed handler entry point.
",
    )
    .expect("write captures manifest");

    let resolved = SourceAdapter::resolve(&AdapterRef::bare("captures"), &project)
        .expect("captures adapter loads via SourceAdapter::resolve");
    assert_eq!(resolved.manifest.name, "captures");
    assert_eq!(resolved.manifest.axis, Axis::Source);
    assert_eq!(
        resolved.manifest.operations().copied().collect::<Vec<_>>(),
        vec![SourceOperation::Extract, SourceOperation::Survey],
        "captures serves survey + extract per workflow §Runtime source adapter"
    );
    assert!(
        matches!(resolved.location, AdapterLocation::Local(_)),
        "live manifest resolves under adapters/sources/<name>/ (local axis)"
    );
    assert!(
        resolved.location.path().ends_with("adapters/sources/captures"),
        "resolver root must land on the adapter directory, got: {}",
        resolved.location.path().display()
    );
}

#[test]
fn resolves_target_adapter_with_inputs() {
    // A target manifest declares the extra `build` inputs
    // its operation consumes (paths relative to `inputs.root`, each
    // flagged `required`). The flat list must round-trip through
    // `TargetAdapter::resolve` with fields populated.
    let (_tmp, project) = local_project();
    let manifest_dir = project.join("adapters").join("targets").join("with-inputs");
    fs::create_dir_all(&manifest_dir).expect("create target adapter dir");
    fs::write(
        manifest_dir.join("adapter.yaml"),
        r"name: with-inputs
version: 1.0.0
axis: target
inputs:
  - path: tokens.yaml
    required: true
  - path: assets.yaml
    required: false
description: Target adapter declaring build inputs.
",
    )
    .expect("write manifest with inputs");

    let resolved = TargetAdapter::resolve(&AdapterRef::bare("with-inputs"), &project)
        .expect("target adapter declaring inputs resolves");
    assert_eq!(resolved.manifest.inputs.len(), 2, "both declared inputs survive the round-trip");
    assert_eq!(resolved.manifest.inputs[0].path, "tokens.yaml");
    assert!(resolved.manifest.inputs[0].required, "first input is required");
    assert_eq!(resolved.manifest.inputs[1].path, "assets.yaml");
    assert!(!resolved.manifest.inputs[1].required, "second input is optional");
}

#[test]
fn target_adapter_inputs_default_empty() {
    // The `inputs` field is optional; the in-tree `omnia` fixture omits
    // it, so a resolved manifest must default to an empty list.
    let (_tmp, project) = local_project();
    let resolved = TargetAdapter::resolve(&AdapterRef::bare("omnia"), &project)
        .expect("resolve target adapter without inputs");
    assert!(
        resolved.manifest.inputs.is_empty(),
        "a manifest that omits `inputs` defaults to an empty list"
    );
}

#[test]
fn malformed_input_rejected_at_load() {
    // An `inputs` entry missing the required `required` flag must fail
    // the target-axis schema before the typed manifest materialises —
    // confirming the new field flows through `TargetAdapter::resolve`.
    let (_tmp, project) = local_project();
    let manifest_dir = project.join("adapters").join("targets").join("bad-inputs");
    fs::create_dir_all(&manifest_dir).expect("create target adapter dir");
    fs::write(
        manifest_dir.join("adapter.yaml"),
        r"name: bad-inputs
version: 1.0.0
axis: target
inputs:
  - path: tokens.yaml
description: Target adapter with a malformed input entry.
",
    )
    .expect("write manifest with malformed input entry");

    let err = TargetAdapter::resolve(&AdapterRef::bare("bad-inputs"), &project)
        .expect_err("input entry omitting `required` must fail");
    let detail = err.to_string();
    assert!(
        detail.contains("adapter-schema-violation")
            || detail.contains("adapter-manifest-malformed"),
        "expected schema violation, got: {detail}"
    );
}

#[test]
fn axis_mismatch_reports_diagnostic() {
    // Adapter file lives under `adapters/sources/<name>/` but declares
    // `axis: target` — should fall through to the source schema and
    // ultimately the axis-mismatch check.
    let (_tmp, project) = local_project();
    let bad_root = project.join("adapters").join("sources").join("mislabeled");
    fs::create_dir_all(&bad_root).expect("create dir");
    fs::write(
        bad_root.join("adapter.yaml"),
        r"name: mislabeled
version: 1.0.0
axis: target
description: Mislabeled fixture.
",
    )
    .expect("write manifest");

    let err = SourceAdapter::resolve(&AdapterRef::bare("mislabeled"), &project)
        .expect_err("axis literal must match the requested axis");
    let detail = err.to_string();
    assert!(
        detail.contains("adapter-schema-violation") || detail.contains("adapter-axis-mismatch"),
        "expected axis diagnostic, got: {detail}"
    );
}
