use super::*;
use crate::Platform;

// These tests exercise the generic adapter loader/parser. Manifest names
// are deliberately contrived (`demo-source` / `demo-target` / …) — the
// core is adapter-agnostic, so no real adapter identity is load-bearing
// here (RFC-50). Adapter-specific manifest invariants belong in the
// adapter's own suite under `specify-adapters`.
//
// Three collapsed tests: the manifest serde matrix, the typed
// `pub(super)` gate functions (which cannot re-home), and the
// cache-path router. Every former assertion over the post-cutover
// manifest shape is preserved.

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
fn manifest_serde_shapes() {
    // The operation set derives from the closed WIT contract:
    // extract < survey for sources, build < merge < shape for targets.
    let source: SourceAdapter = serde_saphyr::from_str(
        r"name: demo-source
version: 1.0.0
axis: source
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

    let target: TargetAdapter = serde_saphyr::from_str(
        r"name: demo-target
version: 1.0.0
axis: target
",
    )
    .expect("parse target");
    assert_eq!(
        target.operations().copied().collect::<Vec<_>>(),
        vec![TargetOperation::Build, TargetOperation::Merge, TargetOperation::Shape]
    );

    // The retired old-stack manifest keys no longer parse: the typed
    // structs are `deny_unknown_fields`, so `briefs` / `execution` /
    // `extension` / `prepare` / `catalog` all fail the serde boundary.
    for retired in [
        "briefs:\n  survey: briefs/survey.md\n  extract: briefs/extract.md",
        "execution: agent",
        "extension: {}",
    ] {
        let yaml = format!("name: demo-source\nversion: 1.0.0\naxis: source\n{retired}\n");
        assert!(
            serde_saphyr::from_str::<SourceAdapter>(&yaml).is_err(),
            "source manifest must reject retired key block `{retired}`",
        );
    }
    for retired in [
        "briefs:\n  shape: briefs/shape.md\n  build: briefs/build.md\n  merge: briefs/merge.md",
        "execution: tool",
        "extension: {}",
        "prepare:\n  argv: [prepare, build]",
        "host_prereq:\n  script: scripts/build-host-prereq.sh",
        "finalize_verify:\n  script: scripts/build-finalize-verify.sh",
        "catalog:\n  infer: true",
    ] {
        let yaml = format!("name: demo-target\nversion: 1.0.0\naxis: target\n{retired}\n");
        assert!(
            serde_saphyr::from_str::<TargetAdapter>(&yaml).is_err(),
            "target manifest must reject retired key block `{retired}`",
        );
    }

    manifest_version_fields();
    manifest_inputs();
    manifest_platforms();
}

fn manifest_version_fields() {
    // RFC-47 D1: `version` is a semver string on the wire, typed in memory.
    let manifest: SourceAdapter = serde_saphyr::from_str(
        r"name: demo-source
version: 2.3.4
axis: source
",
    )
    .expect("parse");
    assert_eq!(manifest.version, semver::Version::new(2, 3, 4));

    // RFC-47 D3: the optional `specify` floor parses into the typed version
    // and round-trips; an absent floor is `None`.
    let manifest: TargetAdapter = serde_saphyr::from_str(
        r#"name: demo-target
version: 1.0.0
specify: "0.28.0"
axis: target
"#,
    )
    .expect("parse");
    assert_eq!(manifest.requires_specify, Some(semver::Version::new(0, 28, 0)));
    let rendered = serde_saphyr::to_string(&manifest).expect("serialise");
    assert!(rendered.contains("specify: 0.28.0"), "specify floor round-trips:\n{rendered}");
    assert_eq!(manifest, serde_saphyr::from_str::<TargetAdapter>(&rendered).expect("reparse"));
    let source: SourceAdapter = serde_saphyr::from_str(
        r"name: demo-source
version: 1.0.0
axis: source
",
    )
    .expect("parse");
    assert_eq!(source.requires_specify, None);
}

fn manifest_inputs() {
    // Target-only `inputs` round-trip; absent inputs default to empty and
    // elide on write.
    let with_inputs: TargetAdapter = serde_saphyr::from_str(
        r"name: demo-target
version: 1.0.0
axis: target
inputs:
  - path: tokens.yaml
    required: true
  - path: assets.yaml
    required: false
",
    )
    .expect("parse inputs");
    assert_eq!(with_inputs.inputs.len(), 2);
    assert_eq!(with_inputs.inputs[0].path, "tokens.yaml");
    assert!(with_inputs.inputs[0].required);
    assert!(!with_inputs.inputs[1].required);
    assert_eq!(
        with_inputs,
        serde_saphyr::from_str::<TargetAdapter>(
            &serde_saphyr::to_string(&with_inputs).expect("serialise")
        )
        .expect("reparse")
    );
    let target: TargetAdapter = serde_saphyr::from_str(
        r"name: demo-target
version: 1.0.0
axis: target
",
    )
    .expect("parse target");
    assert!(target.inputs.is_empty(), "absent inputs default to empty");
    let rendered = serde_saphyr::to_string(&target).expect("serialise");
    assert!(!rendered.contains("inputs"), "absent inputs must elide on write:\n{rendered}");
}

fn manifest_platforms() {
    // Absent platforms default to None and elide on write.
    let target: TargetAdapter = serde_saphyr::from_str(
        r"name: demo-target
version: 1.0.0
axis: target
",
    )
    .expect("parse target");
    let rendered = serde_saphyr::to_string(&target).expect("serialise");
    assert_eq!(target.platforms, None, "absent platforms must default to None");
    assert!(!rendered.contains("platforms"), "absent platforms must elide on write:\n{rendered}");

    // A required capability carries allowed + default platform sets.
    let required: TargetAdapter = serde_saphyr::from_str(
        r"name: demo-target
version: 1.0.0
axis: target
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

#[test]
fn typed_gates() {
    // The belt-and-suspenders version gate: a non-semver version is `adapter-version-malformed`.
    let Error::Validation { code, .. } =
        check_version(&serde_json::json!({ "version": "1" }), Path::new("adapter.yaml"))
            .expect_err("integer-shaped version must be rejected")
    else {
        panic!("expected Error::Validation");
    };
    assert_eq!(code, "adapter-version-malformed");
    check_version(&serde_json::json!({ "version": "1.2.3" }), Path::new("adapter.yaml"))
        .expect("exact semver passes");

    // RFC-47 D2: a `None` pin always picks the installed identity; a matching
    // `Some(_)` pin passes; a mismatched pin cannot resolve (`adapter-version-required`).
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

    // RFC-47 D3: an absent floor never gates; a binary at/above the floor
    // passes; below the floor aborts on the exit-3 path; an unparseable
    // current version is permissive (mirrors config::version_is_older).
    check_requires_specify(None, "0.1.0", "demo-source", Path::new("adapter.yaml"))
        .expect("absent floor never gates");
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
    check_requires_specify(Some(&two), "not-a-version", "demo-target", Path::new("adapter.yaml"))
        .expect("unparseable current version is permissive");
}
