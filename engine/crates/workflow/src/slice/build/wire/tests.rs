use serde_json::{Value, json};

use super::*;

fn report(status: &str, findings: &[Value]) -> BuildReport {
    serde_json::from_value(json!({
        "version": 1,
        "slice": "identity-service",
        "target": "demo-target@1.0.0",
        "status": status,
        "findings": findings,
    }))
    .expect("report deserialises")
}

fn report_with_outputs(status: &str, outputs: &[Value]) -> BuildReport {
    serde_json::from_value(json!({
        "version": 1,
        "slice": "identity-service",
        "target": "demo-target@1.0.0",
        "status": status,
        "findings": [],
        "outputs": outputs,
    }))
    .expect("report with outputs deserialises")
}

/// A success report carrying an optional `ui-surface.screens` value.
fn report_with_ui_surface(screens: u32) -> BuildReport {
    serde_json::from_value(json!({
        "version": 1,
        "slice": "identity-service",
        "target": "demo-target@1.0.0",
        "status": "success",
        "findings": [],
        "ui-surface": { "screens": screens },
    }))
    .expect("report with ui-surface deserialises")
}

// The success-with-blocking gate is asserted end-to-end by
// `engine/tests/slice.rs` (the build-finalize blocking-finding test), so its
// unit duplicate was deleted. The UI-surface coherence check moved in-guest
// (the vectis core's report gate), so its unit tests died with it. The
// output-existence gate has no CLI fixture (`target-build-output-missing`
// never surfaces e2e), so it stays as the two kept tests below.

/// The `BuildRequest` / `BuildReport` serde envelope: the optional `ui-surface`
/// claim, the `project-dir` request round-trip, `deny_unknown_fields`
/// rejection, and the per-platform `outputs` round-trip. Collapsed from the
/// four former single-shape serde tests; every input is preserved.
#[test]
fn wire_serde() {
    // `ui-surface` present round-trips kebab-case; absent defaults to None
    // and is skipped on the wire.
    let with_surface = report_with_ui_surface(3);
    assert_eq!(with_surface.ui_surface, Some(UiSurface { screens: 3 }));
    let serialised = serde_json::to_string(&with_surface).expect("serialise");
    assert!(serialised.contains("ui-surface"), "ui-surface renders kebab-case: {serialised}");
    assert_eq!(with_surface, serde_json::from_str::<BuildReport>(&serialised).expect("reparse"));
    let plain = report("success", &[]);
    assert!(plain.ui_surface.is_none(), "missing ui-surface defaults to None");
    let serialised = serde_json::to_string(&plain).expect("serialise");
    assert!(!serialised.contains("ui-surface"), "absent ui-surface is skipped: {serialised}");

    // A `BuildRequest` round-trips, rendering `project-dir` kebab-case.
    let req = json!({
        "version": 1,
        "slice": "identity-service",
        "project-dir": "/w/workspace/identity-service",
        "inputs": {
            "root": "/w/.specify/slices/identity-service",
            "artifacts": {
                "proposal": "proposal.md",
                "design": "design.md",
                "tasks": "tasks.md",
                "specs": ["specs/identity/spec.md"],
                "additional": ["tokens.yaml"]
            }
        }
    });
    let parsed: BuildRequest = serde_json::from_value(req).expect("request deserialises");
    assert_eq!(parsed.version, BUILD_VERSION);
    assert_eq!(parsed.slice, "identity-service");
    assert_eq!(parsed.inputs.artifacts.specs, vec!["specs/identity/spec.md".to_string()]);
    assert_eq!(parsed.inputs.artifacts.additional, vec!["tokens.yaml".to_string()]);
    let serialised = serde_json::to_string(&parsed).expect("serialise request");
    assert!(serialised.contains("project-dir"), "project-dir renders kebab-case");
    assert_eq!(parsed, serde_json::from_str::<BuildRequest>(&serialised).expect("reparse"));

    // `deny_unknown_fields` rejects a stray report key.
    serde_json::from_value::<BuildReport>(json!({
        "version": 1,
        "slice": "identity-service",
        "target": "demo-target@1.0.0",
        "status": "success",
        "findings": [],
        "stray": true
    }))
    .expect_err("deny_unknown_fields rejects stray keys");

    // `outputs` default to empty (skipped on the wire) and round-trip when present.
    let report = report("success", &[]);
    assert!(report.outputs.is_empty(), "missing outputs defaults to empty");
    let serialised = serde_json::to_string(&report).expect("serialise");
    assert!(!serialised.contains("outputs"), "empty outputs is skipped in serialisation");
    assert_eq!(report, serde_json::from_str::<BuildReport>(&serialised).expect("reparse"));
    let report = report_with_outputs(
        "success",
        &[
            json!({ "platform": "core", "path": "shared/src/app.rs" }),
            json!({ "platform": "ios", "path": "iOS/MyApp/ContentView.swift" }),
        ],
    );
    assert_eq!(report.outputs.len(), 2);
    assert_eq!(report.outputs[0].platform, Platform::Core);
    assert_eq!(report.outputs[0].path, "shared/src/app.rs");
    assert_eq!(report.outputs[1].platform, Platform::Ios);
    let serialised = serde_json::to_string(&report).expect("serialise");
    assert_eq!(report, serde_json::from_str::<BuildReport>(&serialised).expect("reparse"));
}

/// Outputs that exist (file or non-empty tree), an empty outputs list, and
/// any `failure` report all pass the output-existence gate.
#[test]
fn output_gate_accepts() {
    let dir = tempfile::tempdir().expect("tempdir");
    enforce_report_outputs_exist(&report("success", &[]), dir.path())
        .expect("empty outputs passes");

    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("shared/src")).expect("mkdir");
    std::fs::write(dir.path().join("shared/src/app.rs"), "fn main() {}").expect("write");
    enforce_report_outputs_exist(
        &report_with_outputs(
            "success",
            &[json!({ "platform": "core", "path": "shared/src/app.rs" })],
        ),
        dir.path(),
    )
    .expect("present file output passes");
    // A non-empty tree output (a target declares per-platform `shared/` trees).
    enforce_report_outputs_exist(
        &report_with_outputs("success", &[json!({ "platform": "core", "path": "shared/" })]),
        dir.path(),
    )
    .expect("non-empty tree output passes");

    let dir = tempfile::tempdir().expect("tempdir");
    enforce_report_outputs_exist(
        &report_with_outputs(
            "failure",
            &[json!({ "platform": "ios", "path": "iOS/MyApp/ContentView.swift" })],
        ),
        dir.path(),
    )
    .expect("failure status skips the output check");
}

/// Every output-existence failure maps to `target-build-output-missing`:
/// a missing file, a present-but-empty file, an empty directory, an
/// absolute path, and a parent-traversal path.
#[test]
fn output_gate_rejects() {
    fn assert_missing(report: &BuildReport, root: &Path, detail_contains: &str) {
        match enforce_report_outputs_exist(report, root) {
            Err(Error::Validation { code, detail }) => {
                assert_eq!(code, "target-build-output-missing");
                assert!(detail.contains(detail_contains), "detail: {detail}");
            }
            other => panic!("expected output-missing gate, got {other:?}"),
        }
    }

    let dir = tempfile::tempdir().expect("tempdir");
    assert_missing(
        &report_with_outputs(
            "success",
            &[json!({ "platform": "ios", "path": "iOS/MyApp/ContentView.swift" })],
        ),
        dir.path(),
        "",
    );

    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("shared/src")).expect("mkdir");
    std::fs::write(dir.path().join("shared/src/app.rs"), "").expect("write empty");
    assert_missing(
        &report_with_outputs(
            "success",
            &[json!({ "platform": "core", "path": "shared/src/app.rs" })],
        ),
        dir.path(),
        "",
    );

    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("shared")).expect("mkdir");
    assert_missing(
        &report_with_outputs("success", &[json!({ "platform": "core", "path": "shared" })]),
        dir.path(),
        "exists but is empty",
    );

    let dir = tempfile::tempdir().expect("tempdir");
    assert_missing(
        &report_with_outputs("success", &[json!({ "platform": "core", "path": "/etc/passwd" })]),
        dir.path(),
        "absolute or contains `..`",
    );

    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("secret.txt"), "secret").expect("write");
    assert_missing(
        &report_with_outputs("success", &[json!({ "platform": "core", "path": "../secret.txt" })]),
        dir.path(),
        "absolute or contains `..`",
    );
}
