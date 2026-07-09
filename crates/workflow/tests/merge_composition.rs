//! Integration coverage for the composition delta-merge kernel
//! (`workflow::merge::composition`) — the screen-level delta matrix has no
//! CLI fixture. One test sweeps the `merge` operations and errors, the other
//! the two pure shape predicates.

use error::Error;
use workflow::merge::MergeOperation;
use workflow::merge::composition::{baseline_is_non_empty, is_whole_document_replacement, merge};

fn diag_code(err: Error) -> String {
    match err {
        Error::Diag { code, .. } => code.to_string(),
        other => panic!("expected an Error::Diag, got {other:?}"),
    }
}

#[test]
fn merge_applies_delta_operations() {
    // A bare `screens:` document with no `delta:` is a wholesale baseline
    // create, echoed verbatim.
    let screens =
        "version: 1\nscreens:\n  home:\n    title: Home\n  settings:\n    title: Settings\n";
    let result = merge(None, screens).expect("create baseline");
    assert_eq!(result.output, screens);
    assert_eq!(result.operations, vec![MergeOperation::CreatedBaseline { requirement_count: 2 }]);

    // `added` splices a new screen alongside the baseline.
    let baseline = "version: 1\nscreens:\n  home:\n    title: Home\n";
    let result = merge(Some(baseline), "delta:\n  added:\n    settings:\n      title: Settings\n")
        .expect("add");
    assert!(result.output.contains("settings") && result.output.contains("home"));
    assert_eq!(
        result.operations,
        vec![MergeOperation::Added {
            id: "settings".to_string(),
            name: "settings".to_string()
        }]
    );

    // `modified` overwrites an existing screen.
    let result = merge(Some(baseline), "delta:\n  modified:\n    home:\n      title: Home v2\n")
        .expect("modify");
    assert!(result.output.contains("Home v2"));
    assert_eq!(
        result.operations,
        vec![MergeOperation::Modified {
            id: "home".to_string(),
            name: "home".to_string()
        }]
    );

    // `removed` drops a screen.
    let two = "version: 1\nscreens:\n  home:\n    title: Home\n  settings:\n    title: Settings\n";
    let result = merge(Some(two), "delta:\n  removed:\n    settings:\n      reason: deprecated\n")
        .expect("remove");
    assert!(!result.output.contains("settings") && result.output.contains("home"));
    assert_eq!(
        result.operations,
        vec![MergeOperation::Removed {
            id: "settings".to_string(),
            name: "settings".to_string()
        }]
    );

    // A `delta:` against no baseline seeds an empty baseline first.
    let result =
        merge(None, "delta:\n  added:\n    home:\n      title: Home\n").expect("add on empty");
    assert!(result.output.contains("home"));
    assert_eq!(
        result.operations,
        vec![MergeOperation::Added {
            id: "home".to_string(),
            name: "home".to_string()
        }]
    );

    // Re-adding an existing screen, or modifying an absent one, aggregates to
    // a `composition-screen-conflict`.
    let dup = merge(Some(baseline), "delta:\n  added:\n    home:\n      title: Another Home\n")
        .expect_err("duplicate add");
    assert_eq!(diag_code(dup), "composition-screen-conflict");
    let missing = merge(Some(baseline), "delta:\n  modified:\n    ghost:\n      title: Ghost\n")
        .expect_err("missing modify");
    assert_eq!(diag_code(missing), "composition-screen-conflict");

    // A document with neither `screens` nor `delta` is empty.
    let empty = merge(None, "version: 1\nfoo: bar\n").expect_err("empty delta");
    assert_eq!(diag_code(empty), "composition-delta-empty");
}

#[test]
fn shape_predicates() {
    // `is_whole_document_replacement`: `screens:` alone is whole-document;
    // `delta:` (or both keys) is not; malformed YAML errors.
    assert!(
        is_whole_document_replacement("version: 1\nscreens:\n  home:\n    title: Home\n").unwrap()
    );
    assert!(
        !is_whole_document_replacement("delta:\n  added:\n    home:\n      title: Home\n").unwrap()
    );
    assert!(
        !is_whole_document_replacement("screens:\n  home:\n    title: Home\ndelta:\n  added: {}\n")
            .unwrap()
    );
    assert_eq!(
        diag_code(is_whole_document_replacement("\tnot: [valid").unwrap_err()),
        "composition-delta-malformed"
    );

    // `baseline_is_non_empty`: a non-empty `screens` map is the only `true`;
    // empty/absent `screens` and malformed text are all `false`.
    assert!(baseline_is_non_empty("version: 1\nscreens:\n  home:\n    title: Home\n"));
    assert!(!baseline_is_non_empty("version: 1\nscreens: {}\n"));
    assert!(!baseline_is_non_empty("version: 1\n"));
    assert!(!baseline_is_non_empty("\tnot: [valid"));
}
