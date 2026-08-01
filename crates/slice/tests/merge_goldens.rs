//! Merge-engine goldens through the public `slice merge --preview`
//! operation.

use std::path::{Path, PathBuf};

use mock::invoke::run;
use mock::session::Session;
use slice::handlers::{MergeRun, MergeRunBody, MergeRunInput, PreviewBody};

/// The read-only `--preview` mode of `slice merge`.
fn preview_input() -> MergeRunInput {
    MergeRunInput {
        name: "golden".to_string(),
        allow_composition_replace: false,
        preview: true,
        conflict_check: false,
    }
}

fn unwrap_preview(body: MergeRunBody) -> PreviewBody {
    match body {
        MergeRunBody::Preview(preview) => preview,
        other => panic!("--preview must project the preview body, got {other:?}"),
    }
}

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

fn stage(project: &Session, case: &Path, delta: &str) {
    let baseline = case.join("baseline.md");
    if baseline.is_file() {
        let destination = project.root().join(".emery/specs").join(delta);
        std::fs::create_dir_all(&destination).expect("create baseline directory");
        std::fs::copy(baseline, destination.join("spec.md")).expect("stage baseline");
    }

    let destination = project.root().join(".emery/slices/golden/specs").join(delta);
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
        let project = Session::scripted("mock", Vec::new());
        let case = fixtures().join(name);
        stage(&project, &case, "golden");

        let result = run::<MergeRun, _, _>(project.provider(), preview_input()).await;
        let error_golden = case.join("expected-merge-errors.txt");
        match result {
            Ok(body) => {
                let body = unwrap_preview(body);
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
        let project = Session::scripted("mock", Vec::new());
        let case = fixtures().join(name);
        stage(&project, &case, "FAIL");

        let result = run::<MergeRun, _, _>(project.provider(), preview_input()).await;
        let actual = match result {
            Ok(body) => {
                let body = unwrap_preview(body);
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
