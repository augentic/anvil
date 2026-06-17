use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::pack::pack_adapter;

fn write(root: &Path, rel: &str, bytes: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, bytes).expect("write");
}

fn packed_demo() -> Vec<u8> {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path().join("adapter");
    write(&root, "adapter.yaml", b"name: demo\nversion: 1.0.0\n");
    write(&root, "briefs/build.md", b"# build\n");
    pack_adapter(&root, &[]).expect("pack demo")
}

#[test]
fn install_layer_writes_read_only_tree() {
    let store = TempDir::new().expect("store root");
    let entry = store.path().join("demo@1.0.0");

    install_layer(&entry, &packed_demo()).expect("install");

    assert!(entry.join("adapter.yaml").is_file());
    assert_eq!(fs::read(entry.join("briefs/build.md")).expect("read brief"), b"# build\n");
    let perms = fs::metadata(entry.join("adapter.yaml")).expect("stat").permissions();
    assert!(perms.readonly(), "installed files must be read-only");

    let leftover_temp = fs::read_dir(store.path())
        .expect("read store root")
        .filter_map(Result::ok)
        .any(|e| e.file_name().to_string_lossy().contains(".tmp."));
    assert!(!leftover_temp, "no temp dir may survive a successful install");
}

#[test]
fn install_layer_is_idempotent() {
    let store = TempDir::new().expect("store root");
    let entry = store.path().join("demo@1.0.0");
    let layer = packed_demo();

    install_layer(&entry, &layer).expect("first install");
    // The immutable entry is present, so a second call is a no-op rather
    // than a re-unpack — concurrent installers of one identity converge.
    install_layer(&entry, &layer).expect("idempotent second install");
    assert!(entry.join("adapter.yaml").is_file());
}

#[test]
fn entry_key_is_name_at_version() {
    assert_eq!(entry_key(Path::new("/store/omnia@1.2.0")).expect("key"), "omnia@1.2.0");
}

#[test]
fn install_tofu_returns_present_entry() {
    use crate::test_support::{EnvGuard, env_lock};

    let _lock = env_lock();
    let store = TempDir::new().expect("store root");
    let _guard = EnvGuard::scoped("SPECIFY_ADAPTER_CACHE", Some(store.path()));

    // Seed the immutable entry at the resolved store location, then assert
    // TOFU install short-circuits to it without touching the network
    // (the reference is deliberately unreachable).
    let entry = adapter_store_entry("demo", "1.0.0");
    install_layer(&entry, &packed_demo()).expect("seed entry");

    let resolved = install_tofu(
        "demo",
        "1.0.0",
        "unused.invalid/specify/demo:1.0.0",
        &RegistryAuth::Anonymous,
    )
    .expect("idempotent tofu");
    assert_eq!(resolved, entry);
}

#[test]
fn record_store_meta_writes_sidecar() {
    use crate::test_support::{EnvGuard, env_lock};

    let _lock = env_lock();
    let store = TempDir::new().expect("store root");
    let _guard = EnvGuard::scoped("SPECIFY_ADAPTER_CACHE", Some(store.path()));

    // The record-on-install half of RFC-48 D4: a freshly installed entry
    // gains a verify-on-read sidecar that the resolver later re-checks.
    let entry = adapter_store_entry("demo", "1.0.0");
    let layer = packed_demo();
    install_layer(&entry, &layer).expect("install");
    record_store_meta("demo", "1.0.0", &entry, &layer).expect("record sidecar");

    // The sidecar is a writable sibling, never inside the read-only entry
    // tree the digest walks.
    let meta_path = cache::store_meta_path("demo", "1.0.0");
    assert!(meta_path.is_file(), "install must record a verify-on-read sidecar");
    assert!(!meta_path.starts_with(&entry), "the sidecar must be an entry sibling");

    // Verify-on-read passes for the freshly recorded, untouched entry.
    cache::verify_store_entry("demo", "1.0.0").expect("a freshly recorded entry verifies");
}

#[test]
fn verify_store_entry_detects_corruption() {
    use crate::test_support::{EnvGuard, env_lock};

    let _lock = env_lock();
    let store = TempDir::new().expect("store root");
    let _guard = EnvGuard::scoped("SPECIFY_ADAPTER_CACHE", Some(store.path()));

    let entry = adapter_store_entry("demo", "1.0.0");
    let layer = packed_demo();
    install_layer(&entry, &layer).expect("install");
    record_store_meta("demo", "1.0.0", &entry, &layer).expect("record sidecar");

    // Corrupt an installed (read-only) file: relax its perms, rewrite the
    // bytes. The recomputed tree digest must no longer match the sidecar.
    let target = entry.join("adapter.yaml");
    let mut perms = fs::metadata(&target).expect("stat").permissions();
    #[expect(
        clippy::permissions_set_readonly_false,
        reason = "test deliberately makes a read-only store entry writable to simulate on-disk corruption"
    )]
    perms.set_readonly(false);
    fs::set_permissions(&target, perms).expect("relax perms");
    fs::write(&target, b"name: tampered\nversion: 9.9.9\n").expect("corrupt file");

    let mismatch = cache::verify_store_entry("demo", "1.0.0")
        .expect_err("a corrupted entry must fail verify-on-read");
    assert_ne!(mismatch.recorded, mismatch.actual, "the mismatch carries both digests");
}

#[test]
fn verify_store_entry_fails_open() {
    use crate::test_support::{EnvGuard, env_lock};

    let _lock = env_lock();
    let store = TempDir::new().expect("store root");
    let _guard = EnvGuard::scoped("SPECIFY_ADAPTER_CACHE", Some(store.path()));

    // A legacy / foreign entry installed before sidecars existed carries
    // no `.meta`, so verify-on-read is a pass — the entry's read-only
    // immutability remains the baseline guarantee (RFC-48 D4 fail-open).
    let entry = adapter_store_entry("demo", "1.0.0");
    install_layer(&entry, &packed_demo()).expect("install");
    assert!(!cache::store_meta_path("demo", "1.0.0").exists(), "no sidecar was recorded");

    cache::verify_store_entry("demo", "1.0.0").expect("an absent sidecar fails open");
}
