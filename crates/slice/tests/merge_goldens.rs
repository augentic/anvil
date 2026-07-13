//! Merge-engine goldens through the public slice preview operation.

use std::path::{Path, PathBuf};

#[path = "../../project/tests/common/mod.rs"]
mod common;

use common::{Project, run};
use slice::handlers::{Preview, PreviewInput};

const MERGE_CASES: &[&str] = &[
    "case-01-single-req",
    "case-02-multi-req",
    "case-03-new-baseline",
    "case-04-modified",
    "case-05-removed",
    "case-06-renamed",
    "case-07-all-sections",
];

const VALIDATION_CASES: &[&str] = &["case-08-validation-ok", "case-09-validation-fails"];

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/merge")
}

fn stage(project: &Project, case: &Path, delta: &str) {
    let baseline = case.join("baseline.md");
    if baseline.is_file() {
        let destination = project.root.join(".specify/specs").join(delta);
        std::fs::create_dir_all(&destination).expect("create baseline directory");
        std::fs::copy(baseline, destination.join("spec.md")).expect("stage baseline");
    }

    let destination = project.root.join(".specify/slices/golden/specs").join(delta);
    std::fs::create_dir_all(&destination).expect("create delta directory");
    let source = case.join("delta.md");
    if source.is_file() {
        std::fs::copy(source, destination.join("spec.md")).expect("stage delta");
    } else {
        std::fs::write(destination.join("spec.md"), "").expect("stage empty delta");
    }
}

fn assert_golden(path: &Path, actual: &str) {
    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        std::fs::write(path, actual).expect("regenerate golden");
    }
    let expected = std::fs::read_to_string(path).expect("read golden");
    assert_eq!(actual, expected, "golden mismatch: {}", path.display());
}

#[tokio::test]
async fn merged_outputs() {
    for name in MERGE_CASES {
        let project = Project::initialised();
        let case = fixtures().join(name);
        stage(&project, &case, "golden");

        let result = run::<Preview, _>(
            &project,
            PreviewInput {
                name: "golden".to_string(),
            },
        )
        .await;
        let error_golden = case.join("expected-merge-errors.txt");
        match result {
            Ok(body) => {
                if error_golden.is_file() {
                    assert_golden(&error_golden, "");
                }
                assert_eq!(body.specs.len(), 1, "{name}: expected one preview");
                assert_golden(&case.join("expected-merged.md"), &body.specs[0].result.output);
            }
            Err(err) if error_golden.is_file() => {
                assert_golden(&error_golden, &format!("{}\n", err.core()));
            }
            Err(err) => panic!("{name}: preview failed: {err:?}"),
        }
    }
}

#[tokio::test]
async fn validation_outputs() {
    for name in VALIDATION_CASES {
        let project = Project::initialised();
        let case = fixtures().join(name);
        stage(&project, &case, "FAIL");

        let result = run::<Preview, _>(
            &project,
            PreviewInput {
                name: "golden".to_string(),
            },
        )
        .await;
        let actual = match result {
            Ok(body) => {
                assert_eq!(body.specs.len(), 1, "{name}: expected one preview");
                String::new()
            }
            Err(err) => match err.core() {
                error::Error::Diag {
                    code: "merge-spec-conflicts",
                    detail,
                } => format!("{detail}\n"),
                other => panic!("{name}: unexpected preview error: {other:?}"),
            },
        };
        assert_golden(&case.join("expected-validation.txt"), &actual);
    }
}
