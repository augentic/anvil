use super::*;

// Greenfield byte-output is pinned by the golden `case_03_new_baseline`
// (`tests/goldens/merge_engine.rs`). This unit test stays because the
// golden asserts only `result.output`, not the `CreatedBaseline`
// operation and its requirement count — the engine-only counting
// behaviour proven here.
#[test]
fn greenfield_counts_blocks() {
    let delta =
        "# Spec\n\n### Requirement: A\n\nID: REQ-001\n\n### Requirement: B\n\nID: REQ-002\n";
    let result = merge(None, delta).expect("merge ok");
    assert_eq!(result.output, delta);
    assert!(matches!(
        result.operations.as_slice(),
        [MergeOperation::CreatedBaseline { requirement_count: 2 }]
    ));
}

#[test]
fn modified_unknown_id_errors() {
    let baseline = "# Base\n\n### Requirement: Alpha\n\nID: REQ-001\n\n#### Scenario: ok\n\n- ok\n\n### Requirement: Beta\n\nID: REQ-002\n\n#### Scenario: ok\n\n- ok\n";
    let delta = "# delta\n\n## MODIFIED Requirements\n\n### Requirement: Ghost\n\nID: REQ-999\n\n#### Scenario: none\n\n- nothing\n";
    let err = merge(Some(baseline), delta).expect_err("expected merge failure");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "merge-spec-conflicts");
            assert!(
                detail.contains("MODIFIED: ID REQ-999 not found in baseline"),
                "unexpected merge error: {detail}"
            );
        }
        other => panic!("expected merge-spec-conflicts diag, got {other:?}"),
    }
}

#[test]
fn added_id_collision_errors() {
    let baseline = "### Requirement: A\n\nID: REQ-001\n\n#### Scenario: ok\n\n- ok\n";
    let delta = "## ADDED Requirements\n\n### Requirement: Another A\n\nID: REQ-001\n\n#### Scenario: ok\n\n- ok\n";
    let err = merge(Some(baseline), delta).expect_err("expected merge failure");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "merge-spec-conflicts");
            assert!(
                detail.contains("ADDED: ID REQ-001 already exists in baseline"),
                "unexpected error: {detail}"
            );
        }
        other => panic!("expected merge-spec-conflicts diag, got {other:?}"),
    }
}

#[test]
fn rename_records_op() {
    let baseline = "# B\n\n### Requirement: Old name\n\nID: REQ-001\n\n#### Scenario: ok\n\n- ok\n";
    let delta = "## RENAMED Requirements\n\nID: REQ-001\nTO: Shiny new name\n";
    let result = merge(Some(baseline), delta).expect("merge ok");
    assert!(result.output.contains("### Requirement: Shiny new name"));
    assert!(!result.output.contains("Old name"));
    assert_eq!(
        result.operations,
        vec![MergeOperation::Renamed {
            id: "REQ-001".to_string(),
            old_name: "Old name".to_string(),
            new_name: "Shiny new name".to_string(),
        }]
    );
}

#[test]
fn replace_first_once() {
    assert_eq!(replace_first("abab", "ab", "XY"), "XYab");
    assert_eq!(replace_first("abc", "z", "Q"), "abc");
    assert_eq!(replace_first("abc", "", "Q"), "abc");
}
