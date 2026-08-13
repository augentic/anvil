//! Parity gate for the committed correlation judgment-answer schema
//! golden under `crates/system/answers/`: the committed document must
//! byte-match the current generation from the Rust wire types.
//! Regenerate with `REGENERATE_GOLDENS=1`.

use std::path::PathBuf;

#[test]
fn correlation_golden() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("answers/correlation.schema.json");
    let actual = project::answers::render(&system::answers::correlation());
    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        std::fs::create_dir_all(path.parent().expect("answers dir")).expect("create answers dir");
        std::fs::write(&path, &actual).expect("regenerate golden");
    }
    let expected = std::fs::read_to_string(&path).expect("read golden");
    assert_eq!(actual, expected, "golden mismatch: {}", path.display());
}
