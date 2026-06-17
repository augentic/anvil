use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use tempfile::TempDir;

use super::*;

/// Extract a packed layer back into a map of `relative path -> bytes`
/// (directories appear as keys with a trailing `/` and empty bytes), so
/// tests can assert on the materialized tree without touching disk.
fn unpack(layer: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let decoder = zstd::stream::read::Decoder::new(layer).expect("zstd decoder");
    let mut archive = tar::Archive::new(decoder);
    let mut out = BTreeMap::new();
    for entry in archive.entries().expect("tar entries") {
        let mut entry = entry.expect("tar entry");
        let path = entry.path().expect("entry path").to_string_lossy().into_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).expect("read entry");
        out.insert(path, bytes);
    }
    out
}

fn write(root: &Path, rel: &str, bytes: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, bytes).expect("write");
}

#[test]
fn pack_is_byte_deterministic() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path().join("adapter");
    write(&root, "adapter.yaml", b"name: demo\nversion: 1.0.0\n");
    write(&root, "briefs/build.md", b"# build\n");
    write(&root, "rules/RULE-1.md", b"rule one\n");

    let first = pack_adapter(&root, &[]).expect("pack once");
    let second = pack_adapter(&root, &[]).expect("pack twice");
    assert_eq!(first, second, "identical trees must pack to identical bytes");
    assert_eq!(content_digest(&first), content_digest(&second));
}

#[test]
fn pack_order_independent_of_creation_order() {
    let make = |order: [&str; 3]| {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path().join("adapter");
        for name in order {
            write(&root, name, format!("body of {name}").as_bytes());
        }
        let bytes = pack_adapter(&root, &[]).expect("pack");
        (dir, bytes)
    };
    let (_a, packed_a) = make(["a.md", "b.md", "c.md"]);
    let (_b, packed_b) = make(["c.md", "a.md", "b.md"]);
    assert_eq!(packed_a, packed_b, "entry order must be sorted, not creation order");
}

#[test]
#[cfg(unix)]
fn pack_dereferences_symlinks_into_bytes() {
    use std::os::unix::fs::symlink;

    let dir = TempDir::new().expect("tempdir");
    // A shared bundle living outside the adapter tree, reached via a
    // symlink — exactly the `adapters/shared/` hub shape.
    let shared = dir.path().join("shared");
    fs::create_dir_all(&shared).expect("mkdir shared");
    fs::write(shared.join("protocol.md"), b"shared protocol body\n").expect("write shared");

    let root = dir.path().join("adapter");
    write(&root, "adapter.yaml", b"name: demo\n");
    fs::create_dir_all(root.join("references")).expect("mkdir references");
    symlink(&shared, root.join("references/shared")).expect("symlink dir");

    let layer = pack_adapter(&root, &[]).expect("pack");
    let tree = unpack(&layer);

    assert_eq!(
        tree.get("references/shared/protocol.md").map(Vec::as_slice),
        Some(b"shared protocol body\n".as_slice()),
        "symlinked shared content must be inlined as real bytes",
    );
    // No tar entry may be a bare symlink — everything is dereferenced.
    let decoder = zstd::stream::read::Decoder::new(layer.as_slice()).expect("decoder");
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries().expect("entries") {
        let entry = entry.expect("entry");
        assert_ne!(
            entry.header().entry_type(),
            tar::EntryType::Symlink,
            "packed artifact must contain no symlink entries",
        );
    }
}

#[test]
fn pack_excludes_vcs_and_extra_names() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path().join("adapter");
    write(&root, "adapter.yaml", b"name: demo\n");
    write(&root, ".git/config", b"[core]\n");
    write(&root, "target/debug/x", b"junk\n");
    write(&root, "extension/src/lib.rs", b"fn main() {}\n");

    let layer = pack_adapter(&root, &["extension"]).expect("pack");
    let tree = unpack(&layer);

    assert!(tree.contains_key("adapter.yaml"));
    assert!(tree.keys().all(|k| !k.starts_with(".git/")), "{:?}", tree.keys());
    assert!(tree.keys().all(|k| !k.starts_with("target/")), "{:?}", tree.keys());
    assert!(
        tree.keys().all(|k| !k.starts_with("extension/")),
        "declared extension source must be excluded: {:?}",
        tree.keys()
    );
}

#[test]
fn unpack_round_trips_packed_tree() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path().join("adapter");
    write(&root, "adapter.yaml", b"name: demo\nversion: 1.0.0\n");
    write(&root, "briefs/build.md", b"# build\n");
    let layer = pack_adapter(&root, &[]).expect("pack");

    let out = dir.path().join("unpacked");
    unpack_adapter(&layer, &out).expect("unpack");

    assert_eq!(
        fs::read(out.join("adapter.yaml")).expect("read yaml"),
        b"name: demo\nversion: 1.0.0\n"
    );
    assert_eq!(fs::read(out.join("briefs/build.md")).expect("read brief"), b"# build\n");
}

#[test]
fn verify_digest_match_and_drift() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path().join("adapter");
    write(&root, "adapter.yaml", b"name: demo\n");
    let layer = pack_adapter(&root, &[]).expect("pack");
    let recorded = content_digest(&layer);

    verify_digest("specify:demo@1.0.0", &layer, &recorded).expect("matching digest verifies");

    let tampered = {
        let mut bytes = layer;
        *bytes.last_mut().expect("non-empty") ^= 0xff;
        bytes
    };
    let err = verify_digest("specify:demo@1.0.0", &tampered, &recorded)
        .expect_err("a moved tag / tampered layer must be refused");
    assert!(matches!(
        err,
        ExtensionError::Diag {
            code: "adapter-digest-mismatch",
            ..
        }
    ));
}
