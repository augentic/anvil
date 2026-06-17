use serde_json::Value;

use super::WorkspaceModelVersion;

#[test]
fn version_serialises_as_one() {
    let v = serde_json::to_value(WorkspaceModelVersion).expect("serialise");
    assert_eq!(v, Value::from(1));
}

#[test]
fn version_rejects_other_values() {
    let err = serde_json::from_value::<WorkspaceModelVersion>(Value::from(2))
        .expect_err("v2 must be rejected");
    assert!(err.to_string().contains("unsupported WorkspaceModel version"));
}
