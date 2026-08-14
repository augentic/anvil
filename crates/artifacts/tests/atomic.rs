//! Behavioural coverage for the crash-safe atomic writers
//! (`artifacts::atomic`).

use std::collections::BTreeMap;

use artifacts::atomic::{bytes_write, yaml_write};

#[test]
fn string_appends_newline() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("out.yaml");
    let mut value = BTreeMap::new();
    value.insert("name".to_owned(), "identity-service".to_owned());
    value.insert("kind".to_owned(), "slice".to_owned());

    yaml_write(&path, &value).expect("yaml_write succeeds");

    let raw = std::fs::read_to_string(&path).expect("written file is readable");
    assert!(raw.ends_with('\n'), "writer guarantees a trailing newline, got {raw:?}");
    let parsed: BTreeMap<String, String> =
        serde_saphyr::from_str(&raw).expect("written YAML re-parses");
    assert_eq!(parsed, value, "round-trip preserves the serialised value");
}

#[test]
fn creates_parent_dirs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("nested").join("deeper").join("out.yaml");
    let value = vec![1_u32, 2, 3];

    yaml_write(&path, &value).expect("yaml_write creates parents");

    assert!(path.exists(), "writer created the nested parent chain");
}

#[test]
fn overwrites_atomically() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("out.yaml");
    yaml_write(&path, &vec!["first"]).expect("first write");
    yaml_write(&path, &vec!["second"]).expect("second write");

    let parsed: Vec<String> =
        serde_saphyr::from_str(&std::fs::read_to_string(&path).expect("read")).expect("parse");
    assert_eq!(parsed, vec!["second".to_owned()], "rename replaces prior contents");
}

#[test]
fn bytes_exact() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("plan.lock");
    let payload = b"12345\n";

    bytes_write(&path, payload).expect("bytes_write succeeds");

    let on_disk = std::fs::read(&path).expect("file is readable");
    assert_eq!(on_disk, payload, "bytes_write writes the caller's bytes verbatim");
}

#[test]
fn bytes_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("empty");

    bytes_write(&path, b"").expect("empty payload is allowed");

    assert_eq!(std::fs::read(&path).expect("read").len(), 0, "empty file is written");
}

#[test]
fn copy_matches_bytes() {
    use artifacts::atomic::copy_write;

    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("src.bin");
    let dest = dir.path().join("nested").join("dest.bin");
    let payload: Vec<u8> = (0_u8..=250).cycle().take(200_000).collect();
    std::fs::write(&src, &payload).expect("src");

    copy_write(&dest, &src).expect("copy_write succeeds");

    assert_eq!(
        std::fs::read(&dest).expect("read"),
        payload,
        "copy_write streams src bytes verbatim"
    );
}
