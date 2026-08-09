//! Merge-engine goldens over the crate-private [`three_way`] read
//! kernel — a dense fixture matrix that is only reachable in-process
//! since the read-only merge preview verb was retired.

use std::path::{Path, PathBuf};

use super::read::three_way;
use crate::merge::artifact_classes;

const MERGE_CASES: &[&str] = &[
    "spec-single-req",
    "spec-multi-req",
    "spec-new-baseline",
    "spec-modified",
    "spec-removed",
    "spec-renamed",
    "spec-all-sections",
];

const VALIDATION_CASES: &[&str] = &["spec-validation-ok", "spec-validation-fails"];

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn stage(root: &Path, case: &Path, name: &str) {
    let baseline = case.join("baseline.md");
    if baseline.is_file() {
        let destination = root.join(".emery/specs").join(name);
        std::fs::create_dir_all(&destination).expect("create baseline directory");
        std::fs::copy(baseline, destination.join("spec.md")).expect("stage baseline");
    }

    let destination = root.join(".emery/slices/golden/specs").join(name);
    std::fs::create_dir_all(&destination).expect("create delta directory");
    let source = case.join("delta.md");
    if source.is_file() {
        std::fs::copy(source, destination.join("spec.md")).expect("stage delta");
    } else {
        std::fs::write(destination.join("spec.md"), "").expect("stage empty delta");
    }
}

fn plan(root: &Path) -> Result<Vec<super::PreviewEntry>, error::Error> {
    let slice_dir = root.join(".emery/slices/golden");
    let classes = artifact_classes(root, &slice_dir);
    three_way(&slice_dir, &classes)
}

fn assert_golden(path: &Path, actual: &str) {
    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        std::fs::write(path, actual).expect("regenerate golden");
    }
    let expected = std::fs::read_to_string(path).expect("read golden");
    assert_eq!(actual, expected, "golden mismatch: {}", path.display());
}

#[test]
fn merged_outputs() {
    for name in MERGE_CASES {
        let root = tempfile::tempdir().expect("tempdir");
        let case = fixtures().join(name);
        stage(root.path(), &case, "golden");

        let error_golden = case.join("expected-merge-errors.txt");
        match plan(root.path()) {
            Ok(merged) => {
                if error_golden.is_file() {
                    assert_golden(&error_golden, "");
                }
                assert_eq!(merged.len(), 1, "{name}: expected one merged entry");
                assert_golden(&case.join("expected-merged.md"), &merged[0].result.output);
            }
            Err(err) if error_golden.is_file() => {
                assert_golden(&error_golden, &format!("{err}\n"));
            }
            Err(err) => panic!("{name}: merge plan failed: {err:?}"),
        }
    }
}

#[test]
fn validation_outputs() {
    for name in VALIDATION_CASES {
        let root = tempfile::tempdir().expect("tempdir");
        let case = fixtures().join(name);
        stage(root.path(), &case, "FAIL");

        let actual = match plan(root.path()) {
            Ok(merged) => {
                assert_eq!(merged.len(), 1, "{name}: expected one merged entry");
                String::new()
            }
            Err(error::Error::Diag {
                code: "merge-spec-conflicts",
                detail,
            }) => format!("{detail}\n"),
            Err(other) => panic!("{name}: unexpected merge-plan error: {other:?}"),
        };
        assert_golden(&case.join("expected-validation.txt"), &actual);
    }
}
