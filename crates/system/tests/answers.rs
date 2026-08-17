//! Parity gate for the committed judgment-answer schema goldens under
//! `crates/system/answers/`: each committed document must byte-match
//! the current generation from the Rust wire types. Regenerate with
//! `REGENERATE_GOLDENS=1`.

use std::path::PathBuf;

fn assert_golden(file: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("answers").join(file);
    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        std::fs::create_dir_all(path.parent().expect("answers dir")).expect("create answers dir");
        std::fs::write(&path, actual).expect("regenerate golden");
    }
    let expected = std::fs::read_to_string(&path).expect("read golden");
    assert_eq!(actual, expected, "golden mismatch: {}", path.display());
}

#[test]
fn correlation_golden() {
    assert_golden(
        "correlation.schema.json",
        &project::answers::render(&system::answers::correlation()),
    );
}

#[test]
fn proposal_golden() {
    assert_golden("proposal.schema.json", &project::answers::render(&system::answers::proposal()));
}

/// The `version` property is pinned to each wire constant, so the
/// model reads the literal off the schema instead of hunting for it.
#[test]
fn version_consts() {
    for schema in [system::answers::correlation(), system::answers::proposal()] {
        assert_eq!(
            schema.pointer("/properties/version/const").expect("version const"),
            &serde_json::json!(1)
        );
    }
}
