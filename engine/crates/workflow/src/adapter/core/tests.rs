use super::*;
use crate::Platform;

// These tests exercise the generic adapter loader/parser. Manifest names
// are deliberately contrived (`demo-source` / `demo-target` / …) — the
// core is adapter-agnostic, so no real adapter identity is load-bearing
// here (RFC-50). Adapter-specific manifest invariants belong in the
// adapter's own suite under `specify-adapters`.

#[test]
fn axis_routing() {
    assert_eq!(Axis::Source.dir_segment(), "sources");
    assert_eq!(Axis::Target.dir_segment(), "targets");
    // The manifest mirror lives out-of-tree under the per-project OS cache;
    // `cache_dir` routes `manifests/<axis>/<name>` beneath it.
    let project = Path::new("/proj");
    let base = crate::config::Layout::new(project).cache_dir();
    assert_eq!(
        cache_dir(project, Axis::Source, "demo-source"),
        base.join("manifests/sources/demo-source")
    );
    assert_eq!(
        cache_dir(project, Axis::Target, "demo-target"),
        base.join("manifests/targets/demo-target")
    );
}

#[test]
fn briefs_typed_at_parse_boundary() {
    // Source operations are a closed enum; BTreeMap key order is extract < survey.
    let source: SourceAdapter = serde_saphyr::from_str(
        r"name: demo-source
version: 1.0.0
axis: source
briefs:
  survey: briefs/survey.md
  extract: briefs/extract.md
",
    )
    .expect("parse source");
    assert_eq!(
        source.operations().copied().collect::<Vec<_>>(),
        vec![SourceOperation::Extract, SourceOperation::Survey]
    );
    let reparsed: SourceAdapter =
        serde_saphyr::from_str(&serde_saphyr::to_string(&source).expect("serialise"))
            .expect("reparse");
    assert_eq!(source, reparsed);

    // Target operations order build < merge < shape (kebab-case BTreeMap keys).
    let target: TargetAdapter = serde_saphyr::from_str(
        r"name: demo-target
version: 1.0.0
axis: target
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
",
    )
    .expect("parse target");
    assert_eq!(
        target.operations().copied().collect::<Vec<_>>(),
        vec![TargetOperation::Build, TargetOperation::Merge, TargetOperation::Shape]
    );
}

#[test]
fn execution_mode_and_gate() {
    // `agent` round-trips kebab-case...
    let manifest: SourceAdapter = serde_saphyr::from_str(
        r"name: demo-source
version: 1.0.0
axis: source
execution: agent
briefs:
  survey: briefs/survey.md
  extract: briefs/extract.md
",
    )
    .expect("parse agent");
    assert_eq!(manifest.execution, Some(Execution::Agent));
    let rendered = serde_saphyr::to_string(&manifest).expect("serialise");
    assert!(rendered.contains("execution: agent"), "execution round-trips kebab-case:\n{rendered}");
    assert_eq!(manifest, serde_saphyr::from_str::<SourceAdapter>(&rendered).expect("reparse"));

    // ...and `tool` parses on the target axis.
    let target: TargetAdapter = serde_saphyr::from_str(
        r"name: demo-target
version: 1.0.0
axis: target
execution: tool
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
",
    )
    .expect("parse tool");
    assert_eq!(target.execution, Some(Execution::Tool));

    // The typed gate refuses a missing mode (exit 2) and accepts a declared one.
    let Error::Validation { code, .. } = check_execution(None, Path::new("adapter.yaml"))
        .expect_err("missing execution must be rejected")
    else {
        panic!("expected Error::Validation");
    };
    assert_eq!(code, "adapter-execution-mode-required");
    check_execution(Some(Execution::Agent), Path::new("adapter.yaml")).expect("agent passes");
    check_execution(Some(Execution::Tool), Path::new("adapter.yaml")).expect("tool passes");
}

#[test]
fn extension_parse_and_rejections() {
    // RFC-48 D11: the singular `extension` object carries an optional run
    // `name` plus structured `{read, write}` permissions and round-trips.
    let manifest: TargetAdapter = serde_saphyr::from_str(
        r#"name: demo-target
version: 1.0.0
axis: target
execution: agent
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
extension:
  name: demo-tool
  permissions:
    read: ["$PROJECT_DIR/demo"]
    write: []
"#,
    )
    .expect("parse");
    let extension = manifest.extension.as_ref().expect("extension declared");
    assert_eq!(extension.name.as_deref(), Some("demo-tool"));
    assert_eq!(extension.permissions.read, vec!["$PROJECT_DIR/demo".to_string()]);
    assert!(extension.permissions.write.is_empty());
    assert_eq!(
        manifest,
        serde_saphyr::from_str::<TargetAdapter>(
            &serde_saphyr::to_string(&manifest).expect("serialise")
        )
        .expect("reparse")
    );

    // An empty `extension: {}` defaults the run name to the adapter name.
    let omitted: TargetAdapter = serde_saphyr::from_str(
        r"name: demo-target
version: 1.0.0
axis: target
execution: agent
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
extension: {}
",
    )
    .expect("parse");
    assert!(omitted.extension.as_ref().expect("extension").name.is_none());

    // The retired plural `tools[]` array and per-extension version/source/sha256 are denied.
    serde_saphyr::from_str::<TargetAdapter>(
        r"name: demo-target
version: 1.0.0
axis: target
execution: agent
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
tools:
  - name: demo-tool
    version: 1.0.0
",
    )
    .expect_err("the plural tools[] array no longer parses");
    for retired in ["version: 1.0.0", "source: https://example.com/x.wasm", "sha256: abc"] {
        let yaml = format!(
            "name: demo-target\nversion: 1.0.0\naxis: target\nexecution: agent\nbriefs:\n  shape: briefs/shape.md\n  build: briefs/build.md\n  merge: briefs/merge.md\nextension:\n  name: demo-tool\n  {retired}\n",
        );
        assert!(
            serde_saphyr::from_str::<TargetAdapter>(&yaml).is_err(),
            "extension must reject retired field `{retired}`",
        );
    }
}

#[test]
fn version_semver_parse_and_gate() {
    // RFC-47 D1: `version` is a semver string on the wire, typed in memory.
    let manifest: SourceAdapter = serde_saphyr::from_str(
        r"name: demo-source
version: 2.3.4
axis: source
briefs:
  survey: briefs/survey.md
  extract: briefs/extract.md
",
    )
    .expect("parse");
    assert_eq!(manifest.version, semver::Version::new(2, 3, 4));

    // The belt-and-suspenders gate: a non-semver version is `adapter-version-malformed`.
    let Error::Validation { code, .. } =
        check_version(&serde_json::json!({ "version": "1" }), Path::new("adapter.yaml"))
            .expect_err("integer-shaped version must be rejected")
    else {
        panic!("expected Error::Validation");
    };
    assert_eq!(code, "adapter-version-malformed");
    check_version(&serde_json::json!({ "version": "1.2.3" }), Path::new("adapter.yaml"))
        .expect("exact semver passes");
}

#[test]
fn requested_version_matches_identity() {
    // RFC-47 D2: a `None` pin always picks the installed identity; a
    // matching `Some(_)` pin passes; a mismatched pin cannot resolve a
    // single installed identity (`adapter-version-required`).
    let installed = semver::Version::new(1, 0, 0);
    check_requested_version(None, "demo-target", &installed, Path::new("adapter.yaml"))
        .expect("bare ref resolves the single identity");
    check_requested_version(Some(&installed), "demo-target", &installed, Path::new("adapter.yaml"))
        .expect("matching pin resolves");

    let other = semver::Version::new(2, 0, 0);
    let Error::Validation { code, .. } =
        check_requested_version(Some(&other), "demo-target", &installed, Path::new("adapter.yaml"))
            .expect_err("mismatched pin must be rejected")
    else {
        panic!("expected Error::Validation");
    };
    assert_eq!(code, "adapter-version-required");
}

#[test]
fn requires_specify_floor() {
    // RFC-47 D3: the optional `specify` key parses into the typed floor and round-trips.
    let manifest: TargetAdapter = serde_saphyr::from_str(
        r#"name: demo-target
version: 1.0.0
specify: "0.28.0"
axis: target
execution: agent
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
"#,
    )
    .expect("parse");
    assert_eq!(manifest.requires_specify, Some(semver::Version::new(0, 28, 0)));
    let rendered = serde_saphyr::to_string(&manifest).expect("serialise");
    assert!(rendered.contains("specify: 0.28.0"), "specify floor round-trips:\n{rendered}");
    assert_eq!(manifest, serde_saphyr::from_str::<TargetAdapter>(&rendered).expect("reparse"));

    // An absent floor never gates, even against an ancient binary.
    let source: SourceAdapter = serde_saphyr::from_str(
        r"name: demo-source
version: 1.0.0
axis: source
execution: agent
briefs:
  survey: briefs/survey.md
  extract: briefs/extract.md
",
    )
    .expect("parse");
    assert_eq!(source.requires_specify, None);
    check_requires_specify(
        source.requires_specify.as_ref(),
        "0.1.0",
        "demo-source",
        Path::new("adapter.yaml"),
    )
    .expect("absent floor never gates");

    // A binary at or above the floor passes; below the floor aborts on the exit-3 path.
    let floor = semver::Version::new(0, 28, 0);
    check_requires_specify(Some(&floor), "0.28.0", "demo-target", Path::new("adapter.yaml"))
        .expect("exact floor passes");
    check_requires_specify(Some(&floor), "0.29.1", "demo-target", Path::new("adapter.yaml"))
        .expect("newer passes");

    let two = semver::Version::new(2, 0, 0);
    let err = check_requires_specify(Some(&two), "1.5.0", "demo-target", Path::new("adapter.yaml"))
        .expect_err("a binary below the floor must be rejected");
    assert_eq!(err.variant_str(), "adapter-cli-too-old");
    let Error::AdapterCliTooOld { required, found, .. } = err else {
        panic!("expected Error::AdapterCliTooOld, got: {err:?}");
    };
    assert_eq!(required, "2.0.0");
    assert_eq!(found, "1.5.0");

    // An unparseable running version is permissive (mirrors config::version_is_older).
    check_requires_specify(Some(&two), "not-a-version", "demo-target", Path::new("adapter.yaml"))
        .expect("unparseable current version is permissive");
}

#[test]
fn unknown_brief_key_rejected() {
    // `shape` is a target operation; on a source manifest it must fail at
    // the typed `briefs: BTreeMap<SourceOperation, _>` boundary.
    let err = serde_saphyr::from_str::<SourceAdapter>(
        r"name: bogus
version: 1.0.0
axis: source
briefs:
  survey: briefs/survey.md
  shape: briefs/shape.md
",
    )
    .expect_err("unknown source operation must be rejected");
    let detail = err.to_string();
    assert!(
        detail.contains("shape") || detail.contains("survey"),
        "expected closed-enum diagnostic, got: {detail}"
    );
}

#[test]
fn platforms_capability_round_trip() {
    // Absent platforms default to None and elide on write.
    let bare: TargetAdapter = serde_saphyr::from_str(
        r"name: demo-target
version: 1.0.0
axis: target
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
",
    )
    .expect("parse");
    assert_eq!(bare.platforms, None, "absent platforms must default to None");
    let rendered = serde_saphyr::to_string(&bare).expect("serialise");
    assert!(!rendered.contains("platforms"), "absent platforms must elide on write:\n{rendered}");
    assert_eq!(bare, serde_saphyr::from_str::<TargetAdapter>(&rendered).expect("reparse"));

    // A required capability carries allowed + default platform sets.
    let required: TargetAdapter = serde_saphyr::from_str(
        r"name: demo-target
version: 1.0.0
axis: target
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
platforms:
  required: true
  allowed:
    - core
    - ios
    - android
    - web
    - desktop
  default:
    - core
    - ios
    - android
",
    )
    .expect("parse");
    let cap = required.platforms.as_ref().expect("platforms must be Some");
    assert!(cap.required);
    assert_eq!(
        cap.allowed,
        vec![Platform::Core, Platform::Ios, Platform::Android, Platform::Web, Platform::Desktop]
    );
    assert_eq!(cap.default, vec![Platform::Core, Platform::Ios, Platform::Android]);
    let rendered = serde_saphyr::to_string(&required).expect("serialise");
    assert!(rendered.contains("required: true"), "required must round-trip");
    assert_eq!(required, serde_saphyr::from_str::<TargetAdapter>(&rendered).expect("reparse"));

    // An optional capability round-trips required: false.
    let optional: TargetAdapter = serde_saphyr::from_str(
        r"name: demo-optional
version: 1.0.0
axis: target
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
platforms:
  required: false
  allowed:
    - core
  default:
    - core
",
    )
    .expect("parse");
    let cap = optional.platforms.as_ref().expect("platforms must be Some");
    assert!(!cap.required);
    assert_eq!(cap.allowed, vec![Platform::Core]);
    assert_eq!(cap.default, vec![Platform::Core]);
    assert_eq!(
        optional,
        serde_saphyr::from_str::<TargetAdapter>(&serde_saphyr::to_string(&optional).unwrap())
            .expect("reparse")
    );
}
