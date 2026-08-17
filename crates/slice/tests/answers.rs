//! Parity gate for the committed synthesis judgment-answer schema
//! golden under `crates/slice/answers/`: the committed document must
//! byte-match the current generation from the Rust wire types.
//! Regenerate with `REGENERATE_GOLDENS=1`.

use std::path::PathBuf;

#[test]
fn synthesis_golden() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("answers/synthesis.schema.json");
    let actual = project::answers::render(&slice::answers::synthesis());
    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        std::fs::create_dir_all(path.parent().expect("answers dir")).expect("create answers dir");
        std::fs::write(&path, &actual).expect("regenerate golden");
    }
    let expected = std::fs::read_to_string(&path).expect("read golden");
    assert_eq!(actual, expected, "golden mismatch: {}", path.display());
}

/// The `version` property is pinned to the synthesis wire constant, so
/// the model reads the literal off the schema instead of hunting for it.
#[test]
fn version_const() {
    let schema = slice::answers::synthesis();
    assert_eq!(
        schema.pointer("/properties/version/const").expect("version const"),
        &serde_json::json!(4)
    );
}
