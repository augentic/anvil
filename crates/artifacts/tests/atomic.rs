//! Behavioural coverage for the crash-safe atomic writers
//! (`specify_model::atomic`).

use std::collections::BTreeMap;

use specify_model::atomic::{bytes_write, yaml_write};

#[test]
fn write_round_trips_appends_newline() {
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
fn write_creates_parent_dirs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("nested").join("deeper").join("out.yaml");
    let value = vec![1_u32, 2, 3];

    yaml_write(&path, &value).expect("yaml_write creates parents");

    assert!(path.exists(), "writer created the nested parent chain");
}

#[test]
fn write_overwrites_atomically() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("out.yaml");
    yaml_write(&path, &vec!["first"]).expect("first write");
    yaml_write(&path, &vec!["second"]).expect("second write");

    let parsed: Vec<String> =
        serde_saphyr::from_str(&std::fs::read_to_string(&path).expect("read")).expect("parse");
    assert_eq!(parsed, vec!["second".to_owned()], "rename replaces prior contents");
}

#[test]
fn bytes_write_persists_exact_bytes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("plan.lock");
    let payload = b"12345\n";

    bytes_write(&path, payload).expect("bytes_write succeeds");

    let on_disk = std::fs::read(&path).expect("file is readable");
    assert_eq!(on_disk, payload, "bytes_write writes the caller's bytes verbatim");
}

#[test]
fn bytes_write_writes_empty_payload() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("empty");

    bytes_write(&path, b"").expect("empty payload is allowed");

    assert_eq!(std::fs::read(&path).expect("read").len(), 0, "empty file is written");
}
