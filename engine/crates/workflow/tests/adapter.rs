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
//!   refinements (axis literal, closed `briefs.<operation>` keys) reject
//!   hand-rolled inputs.

use std::fs;
use std::path::{Path, PathBuf};

use specify_error::Error;
use specify_workflow::adapter::{
    AdapterLocation, AdapterRef, Axis, SourceAdapter, SourceOperation, TargetAdapter,
    TargetOperation, cache_dir, check_axis_unique_for_name,
};

mod common;

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
    assert_eq!(
        resolved.manifest.briefs.get(&SourceOperation::Extract).map(String::as_str),
        Some("briefs/extract.md")
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
    // `briefs` is a BTreeMap, so `operations()` yields keys in
    // ascending kebab-name order: build < merge < shape.
    assert_eq!(
        resolved.manifest.operations().copied().collect::<Vec<_>>(),
        vec![TargetOperation::Build, TargetOperation::Merge, TargetOperation::Shape]
    );
    assert!(resolved.location.path().ends_with("adapters/targets/omnia"));
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
execution: agent
briefs:
  survey: briefs/survey.md
  extract: briefs/extract.md
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
execution: agent
briefs:
  survey: briefs/survey.md
  extract: briefs/extract.md
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
fn schema_violations_reject_at_load_time() {
    // Source-axis adapter with the wrong brief key set — `shape` is
    // not a source operation, and `extract` is required by
    // `source.schema.json#/properties/briefs`.
    let (_tmp, project) = local_project();
    let bad_root = project.join("adapters").join("sources").join("wrong-ops");
    fs::create_dir_all(&bad_root).expect("create bad source dir");
    fs::write(
        bad_root.join("adapter.yaml"),
        r"name: wrong-ops
version: 1.0.0
axis: source
briefs:
  survey: briefs/survey.md
  shape: briefs/shape.md
",
    )
    .expect("write bad manifest");

    let err = SourceAdapter::resolve(&AdapterRef::bare("wrong-ops"), &project)
        .expect_err("source-axis adapter with wrong brief keys must fail");
    let detail = err.to_string();
    assert!(
        detail.contains("adapter-schema-violation")
            || detail.contains("adapter-manifest-malformed"),
        "expected schema violation, got: {detail}"
    );
}

#[test]
fn resolves_captures_with_extension() {
    // workflow §Acceptance scenario #26-1 (release blocker, D1):
    // pin the loader against the live `adapters/sources/captures/`
    // adapter shape. The manifest carries a singular `extension`
    // declaration (RFC-48 D11) — an optional run `name` plus structured
    // `{read, write}` permissions — and a free-form `description:`
    // field; both must round-trip through the axis-aware loader.
    //
    // This test is the cli-side complement to the deno harness
    // assertions in `augentic/specify` at
    // `tests/cross_repo/sources_test.ts` — the harness pins the
    // golden-fixture data shape (Evidence + provenance.yaml +
    // discovery.md) while this test pins the loader behaviour.
    let (_tmp, project) = local_project();
    let manifest_dir = project.join("adapters").join("sources").join("captures");
    fs::create_dir_all(manifest_dir.join("briefs")).expect("create captures adapter dir");
    fs::write(
        manifest_dir.join("adapter.yaml"),
        r#"name: captures
version: 1.0.0
axis: source
execution: agent
briefs:
  survey: briefs/survey.md
  extract: briefs/extract.md
extension:
  name: replay-index
  permissions:
    read: ["$PROJECT_DIR/.specify"]
    write: []
description: >-
  Runtime capture source adapter. Walks a read-only capture tree under
  `$SOURCE_DIR` and emits one lead per observed handler entry point.
"#,
    )
    .expect("write captures manifest");
    fs::write(manifest_dir.join("briefs/survey.md"), "# survey\n").expect("survey brief stub");
    fs::write(manifest_dir.join("briefs/extract.md"), "# extract\n").expect("extract brief stub");

    let resolved = SourceAdapter::resolve(&AdapterRef::bare("captures"), &project)
        .expect("captures adapter loads via SourceAdapter::resolve");
    assert_eq!(resolved.manifest.name, "captures");
    assert_eq!(resolved.manifest.axis, Axis::Source);
    let extension = resolved.manifest.extension.as_ref().expect("singular extension declared");
    assert_eq!(extension.name.as_deref(), Some("replay-index"));
    assert_eq!(extension.permissions.read, vec!["$PROJECT_DIR/.specify".to_string()]);
    assert!(extension.permissions.write.is_empty());
    assert_eq!(
        resolved.manifest.operations().copied().collect::<Vec<_>>(),
        vec![SourceOperation::Extract, SourceOperation::Survey],
        "captures declares survey + extract per workflow §Runtime source adapter"
    );
    assert_eq!(
        resolved.manifest.briefs.get(&SourceOperation::Extract).map(String::as_str),
        Some("briefs/extract.md")
    );
    assert!(
        matches!(resolved.location, AdapterLocation::Local(_)),
        "live plg manifest resolves under adapters/sources/<name>/ (local axis)"
    );
    assert!(
        resolved.location.path().ends_with("adapters/sources/captures"),
        "resolver root must land on the plg-tree adapter directory, got: {}",
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
execution: tool
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
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
execution: tool
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
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
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
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
