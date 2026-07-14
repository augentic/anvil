//! Parity gate for the committed judgment-answer schema goldens under
//! `crates/project/answers/`: each committed document must byte-match
//! the current generation from the Rust wire types. Regenerate with
//! `REGENERATE_GOLDENS=1`.

use std::path::PathBuf;

use serde_json::Value;

fn assert_golden(file: &str, schema: &Value) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("answers").join(file);
    let actual = project::answers::render(schema);
    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        std::fs::create_dir_all(path.parent().expect("answers dir")).expect("create answers dir");
        std::fs::write(&path, &actual).expect("regenerate golden");
    }
    let expected = std::fs::read_to_string(&path).expect("read golden");
    assert_eq!(actual, expected, "golden mismatch: {}", path.display());
}

#[test]
fn leads_golden() {
    assert_golden("leads.schema.json", &project::answers::leads());
}

#[test]
fn evidence_golden() {
    assert_golden("evidence.schema.json", &project::answers::evidence());
}

#[test]
fn report_golden() {
    assert_golden("report.schema.json", &project::answers::report());
}

#[test]
fn proposal_golden() {
    assert_golden("proposal.schema.json", &project::answers::proposal());
}
