//! Build-report DTO wire coverage through public crate boundaries.

use project::Platform;
use project::seam::wire::{BuildReport, UiSurface};

const REPORT: &str = include_str!("fixtures/build/report.json");

#[test]
fn wire_round_trip() {
    let report: BuildReport = serde_json::from_str(REPORT).expect("report DTO deserialises");
    assert_eq!(report.ui_surface, Some(UiSurface { screens: 3 }));
    assert_eq!(report.outputs[0].platform, Platform::Core);
    assert_eq!(report.outputs[1].platform, Platform::Ios);

    let serialised = serde_json::to_value(&report).expect("report DTO serialises");
    assert_eq!(report, serde_json::from_value(serialised).expect("report DTO reparses"));
}

#[test]
fn rejects_unknown_and_malformed_fields() {
    let mut value: serde_json::Value = serde_json::from_str(REPORT).expect("fixture JSON");
    value["surprise"] = serde_json::json!(true);
    assert!(serde_json::from_value::<BuildReport>(value.clone()).is_err(), "closed shape");

    let mut bad_status: serde_json::Value = serde_json::from_str(REPORT).expect("fixture JSON");
    bad_status["status"] = serde_json::json!("mostly-fine");
    assert!(serde_json::from_value::<BuildReport>(bad_status).is_err(), "closed status enum");
}
