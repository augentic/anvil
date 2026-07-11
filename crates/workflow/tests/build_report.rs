//! Build-report schema and DTO drift coverage through public crate boundaries.

use schema::{
    BUILD_REPORT_JSON_SCHEMA, DIAGNOSTIC_JSON_SCHEMA, compile_ref_validator, validation_errors,
};
use workflow::Platform;
use workflow::slice::{BuildReport, UiSurface};

const DIAGNOSTIC_SCHEMA_URL: &str =
    "https://github.com/augentic/specify/schemas/diagnostics/diagnostic.schema.json";
const REPORT: &str = include_str!("../../../tests/fixtures/build/report.json");

#[test]
fn schema_and_dto_agree() {
    let instance: serde_json::Value = serde_json::from_str(REPORT).expect("fixture JSON");
    let validator = compile_ref_validator(
        BUILD_REPORT_JSON_SCHEMA,
        DIAGNOSTIC_SCHEMA_URL,
        DIAGNOSTIC_JSON_SCHEMA,
    )
    .expect("embedded schemas compile");
    let errors = validation_errors(&validator, &instance);
    assert!(errors.is_empty(), "fixture must satisfy the build-report schema: {errors:#?}");

    let report: BuildReport = serde_json::from_value(instance).expect("report DTO deserialises");
    assert_eq!(report.ui_surface, Some(UiSurface { screens: 3 }));
    assert_eq!(report.outputs[0].platform, Platform::Core);
    assert_eq!(report.outputs[1].platform, Platform::Ios);

    let serialised = serde_json::to_value(&report).expect("report DTO serialises");
    assert!(validation_errors(&validator, &serialised).is_empty());
    assert_eq!(report, serde_json::from_value(serialised).expect("report DTO reparses"));
}
