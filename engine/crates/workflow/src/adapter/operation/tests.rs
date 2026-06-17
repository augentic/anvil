use std::str::FromStr;

use super::*;

#[test]
fn source_operation_round_trips_kebab_case() {
    assert_eq!(SourceOperation::Survey.to_string(), "survey");
    assert_eq!(SourceOperation::Extract.to_string(), "extract");
    assert_eq!(<SourceOperation as FromStr>::from_str("survey"), Ok(SourceOperation::Survey));
    assert_eq!(<SourceOperation as FromStr>::from_str("extract"), Ok(SourceOperation::Extract));
    <SourceOperation as FromStr>::from_str("shape")
        .expect_err("`shape` is a target op; must not parse as a SourceOperation");
    let json = serde_json::to_string(&SourceOperation::Extract).expect("serialise");
    assert_eq!(json, "\"extract\"");
    let back: SourceOperation = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(back, SourceOperation::Extract);
}

#[test]
fn target_operation_round_trips_kebab_case() {
    assert_eq!(TargetOperation::Shape.to_string(), "shape");
    assert_eq!(TargetOperation::Build.to_string(), "build");
    assert_eq!(TargetOperation::Merge.to_string(), "merge");
    assert_eq!(<TargetOperation as FromStr>::from_str("shape"), Ok(TargetOperation::Shape));
    assert_eq!(<TargetOperation as FromStr>::from_str("build"), Ok(TargetOperation::Build));
    assert_eq!(<TargetOperation as FromStr>::from_str("merge"), Ok(TargetOperation::Merge));
    <TargetOperation as FromStr>::from_str("define")
        .expect_err("legacy `define` must not parse as a TargetOperation");
    let json = serde_json::to_string(&TargetOperation::Merge).expect("serialise");
    assert_eq!(json, "\"merge\"");
    let back: TargetOperation = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(back, TargetOperation::Merge);
    assert!(TargetOperation::Build < TargetOperation::Merge);
    assert!(TargetOperation::Merge < TargetOperation::Shape);
}

#[test]
fn unknown_operation_rejected() {
    let err = serde_json::from_str::<SourceOperation>("\"foo\"")
        .expect_err("unknown source operation must fail");
    let detail = err.to_string();
    assert!(detail.contains("foo") || detail.contains("survey"), "{detail}");

    let err = serde_json::from_str::<TargetOperation>("\"define\"")
        .expect_err("legacy `define` rejected on the target axis");
    let detail = err.to_string();
    assert!(detail.contains("define") || detail.contains("shape"), "{detail}");
}
