//! Provider-contract coverage for the public target build report.

use serde_json::json;
use workflow::Platform;
use workflow::slice::{BuildReport, UiSurface};

#[test]
fn report_wire() {
    let report: BuildReport = serde_json::from_value(json!({
        "version": 1,
        "slice": "identity-service",
        "target": "demo-target@1.0.0",
        "status": "success",
        "findings": [],
        "ui-surface": { "screens": 3 },
        "outputs": [
            { "platform": "core", "path": "shared/src/app.rs" },
            { "platform": "ios", "path": "iOS/MyApp/ContentView.swift" }
        ]
    }))
    .expect("report deserialises");

    assert_eq!(report.ui_surface, Some(UiSurface { screens: 3 }));
    assert_eq!(report.outputs[0].platform, Platform::Core);
    assert_eq!(report.outputs[1].platform, Platform::Ios);

    let serialised = serde_json::to_string(&report).expect("serialise");
    assert!(serialised.contains("ui-surface"));
    assert_eq!(report, serde_json::from_str(&serialised).expect("reparse"));
}
