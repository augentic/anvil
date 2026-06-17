use serde_json::{Value, json};

use super::*;

/// A minimal schema-valid [`Diagnostic`] JSON of the given severity,
/// left at the default `violation` kind and untriaged status so
/// `critical` / `important` instances block.
fn finding(severity: &str) -> Value {
    json!({
        "id": "DIAG-0001",
        "title": "test finding",
        "severity": severity,
        "source": "tool",
        "artifact": "code",
        "evidence": { "kind": "snippet", "value": "x" },
        "impact": "impact",
        "remediation": "fix it",
        "fingerprint": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
    })
}

fn report(status: &str, findings: &[Value]) -> BuildReport {
    serde_json::from_value(json!({
        "version": 1,
        "slice": "identity-service",
        "target": "omnia@1.0.0",
        "status": status,
        "findings": findings,
    }))
    .expect("report deserialises")
}

fn report_with_outputs(status: &str, outputs: &[Value]) -> BuildReport {
    serde_json::from_value(json!({
        "version": 1,
        "slice": "identity-service",
        "target": "vectis@1.0.0",
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
        "target": "vectis@1.0.0",
        "status": "success",
        "findings": [],
        "ui-surface": { "screens": screens },
    }))
    .expect("report with ui-surface deserialises")
}

/// Write `body` to a `composition.yaml` under a fresh tempdir and return
/// the dir handle (kept alive by the caller) plus the file path.
fn staged_composition(body: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("composition.yaml");
    std::fs::write(&path, body).expect("write composition.yaml");
    (dir, path)
}

#[test]
fn ui_surface_serialisation() {
    // Present: round-trips and renders kebab-case.
    let with_surface = report_with_ui_surface(3);
    assert_eq!(with_surface.ui_surface, Some(UiSurface { screens: 3 }));
    let serialised = serde_json::to_string(&with_surface).expect("serialise");
    assert!(serialised.contains("ui-surface"), "ui-surface renders kebab-case: {serialised}");
    let reparsed: BuildReport = serde_json::from_str(&serialised).expect("re-deserialise");
    assert_eq!(with_surface, reparsed);

    // Absent: defaults to None and is skipped on the wire.
    let plain = report("success", &[]);
    assert!(plain.ui_surface.is_none(), "missing ui-surface defaults to None");
    let serialised = serde_json::to_string(&plain).expect("serialise");
    assert!(!serialised.contains("ui-surface"), "absent ui-surface is skipped: {serialised}");
}

/// The two A4 mismatch warnings, each non-blocking: a UI-surface claim
/// against a non-UI composition (`composition-unexpected-for-non-ui-slice`)
/// and a UI slice against an empty / absent composition
/// (`composition-empty-for-ui-slice`).
#[test]
fn coherence_flags_mismatches() {
    let (_d0, screens) = staged_composition("version: 1\nscreens:\n  home:\n    name: Home\n");
    let warnings = evaluate_ui_surface_coherence(&report_with_ui_surface(0), &screens);
    assert_eq!(warnings.len(), 1, "expected one warning, got {warnings:?}");
    assert_eq!(warnings[0].rule_id.as_deref(), Some("composition-unexpected-for-non-ui-slice"));
    assert!(!blocking(&warnings[0]), "A4 warnings must never block");

    let (_d1, empty) = staged_composition("version: 1\nscreens: {}\n");
    let warnings = evaluate_ui_surface_coherence(&report_with_ui_surface(2), &empty);
    assert_eq!(warnings.len(), 1, "expected one warning, got {warnings:?}");
    assert_eq!(warnings[0].rule_id.as_deref(), Some("composition-empty-for-ui-slice"));
    assert!(!blocking(&warnings[0]), "A4 warnings must never block");

    // An absent composition with a UI-surface claim also flags empty-for-ui-slice.
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("composition.yaml");
    let warnings = evaluate_ui_surface_coherence(&report_with_ui_surface(1), &missing);
    assert_eq!(warnings.len(), 1, "absent composition for a UI slice warns: {warnings:?}");
    assert_eq!(warnings[0].rule_id.as_deref(), Some("composition-empty-for-ui-slice"));
}

/// Coherent and back-compat cases stay silent: matched surface/composition
/// pairs, and any report without a `ui_surface` claim.
#[test]
fn coherence_silent_cases() {
    let (_s, screens) = staged_composition("version: 1\nscreens:\n  home:\n    name: Home\n");
    assert!(
        evaluate_ui_surface_coherence(&report_with_ui_surface(1), &screens).is_empty(),
        "ui slice + non-empty composition is coherent"
    );
    let (_e, empty) = staged_composition("version: 1\nscreens: {}\n");
    assert!(
        evaluate_ui_surface_coherence(&report_with_ui_surface(0), &empty).is_empty(),
        "non-ui slice + empty composition is coherent"
    );
    assert!(
        evaluate_ui_surface_coherence(&report("success", &[]), &screens).is_empty(),
        "absent ui-surface emits no warnings even with a non-empty composition"
    );
}

/// A non-empty `delta:` envelope counts as a UI surface, so a non-UI
/// slice that emits one is flagged; an all-empty `delta:` does not.
#[test]
fn coherence_reads_delta_envelope() {
    let (_added_dir, added) = staged_composition(
        "version: 1\ndelta:\n  added:\n    home:\n      name: Home\n  modified: {}\n  removed: {}\n",
    );
    let warnings = evaluate_ui_surface_coherence(&report_with_ui_surface(0), &added);
    assert_eq!(warnings.len(), 1, "non-empty delta is a UI surface: {warnings:?}");
    assert_eq!(warnings[0].rule_id.as_deref(), Some("composition-unexpected-for-non-ui-slice"));

    let (_empty_dir, empty) =
        staged_composition("version: 1\ndelta:\n  added: {}\n  modified: {}\n  removed: {}\n");
    assert!(
        evaluate_ui_surface_coherence(&report_with_ui_surface(0), &empty).is_empty(),
        "an all-empty delta carries no UI surface"
    );
}

#[test]
fn request_round_trips() {
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
    let reparsed: BuildRequest = serde_json::from_str(&serialised).expect("re-deserialise");
    assert_eq!(parsed, reparsed);
}

#[test]
fn report_rejects_unknown_field() {
    let bogus = json!({
        "version": 1,
        "slice": "identity-service",
        "target": "omnia@1.0.0",
        "status": "success",
        "findings": [],
        "stray": true
    });
    serde_json::from_value::<BuildReport>(bogus)
        .expect_err("deny_unknown_fields rejects stray keys");
}

#[test]
fn report_outputs_round_trip() {
    // Empty outputs default to empty and are skipped on the wire.
    let report = report("success", &[]);
    assert!(report.outputs.is_empty(), "missing outputs defaults to empty");
    let serialised = serde_json::to_string(&report).expect("serialise");
    assert!(!serialised.contains("outputs"), "empty outputs is skipped in serialisation");
    assert_eq!(report, serde_json::from_str::<BuildReport>(&serialised).expect("re-deserialise"));

    // Present per-platform outputs parse and round-trip.
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
    assert_eq!(report, serde_json::from_str::<BuildReport>(&serialised).expect("re-deserialise"));
}

/// A `success` report may carry only non-blocking findings; a blocking
/// finding fires the gate. `failure` reports may carry blocking findings.
#[test]
fn blocking_finding_gate() {
    match enforce_report_no_blocking_on_success(&report("success", &[finding("critical")])) {
        Err(Error::Validation { code, .. }) => {
            assert_eq!(code, "target-build-success-with-blocking-finding");
        }
        other => panic!("expected blocking-finding gate to fire, got {other:?}"),
    }
    enforce_report_no_blocking_on_success(&report("success", &[finding("suggestion")]))
        .expect("non-blocking success passes");
    enforce_report_no_blocking_on_success(&report("failure", &[finding("critical")]))
        .expect("failure may carry blocking findings");
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
    // A non-empty tree output (vectis declares per-platform `shared/` trees).
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
