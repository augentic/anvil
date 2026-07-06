use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::*;

fn stage_component(dir: &Path, bytes: &[u8]) -> PathBuf {
    let staged = dir.join("staged.wasm");
    fs::write(&staged, bytes).expect("write staged component");
    staged
}

// `install_component` publishes a read-only file with no surviving temp
// file, and is idempotent: a second call on a present immutable entry is
// a no-op rather than a re-copy. Both halves collapse into one test.
#[test]
fn install_component_read_only_idempotent() {
    let store = TempDir::new().expect("store root");
    let entry = store.path().join("demo@1.0.0.wasm");
    let staged = stage_component(store.path(), b"\0asm-demo");

    install_component(&entry, &staged).expect("install");

    assert_eq!(fs::read(&entry).expect("read entry"), b"\0asm-demo");
    let perms = fs::metadata(&entry).expect("stat").permissions();
    assert!(perms.readonly(), "installed component must be read-only");

    let leftover_temp = fs::read_dir(store.path())
        .expect("read store root")
        .filter_map(Result::ok)
        .any(|e| e.file_name().to_string_lossy().contains(".tmp."));
    assert!(!leftover_temp, "no temp file may survive a successful install");

    // Concurrent installers of one identity converge: the second call is a
    // no-op against the present immutable entry.
    install_component(&entry, &staged).expect("idempotent second install");
    assert_eq!(fs::read(&entry).expect("read entry"), b"\0asm-demo");
}

#[test]
fn entry_key_is_name_at_version() {
    assert_eq!(
        entry_key(Path::new("/store/demo-target@1.2.0.wasm")).expect("key"),
        "demo-target@1.2.0.wasm"
    );
}

#[test]
fn install_tofu_returns_present_entry() {
    use crate::test_support::{EnvGuard, env_lock};

    let _lock = env_lock();
    let store = TempDir::new().expect("store root");
    let _guard = EnvGuard::scoped("SPECIFY_ADAPTER_CACHE", Some(store.path()));

    // Seed the immutable entry at the resolved store location, then assert
    // TOFU install short-circuits to it without touching the network
    // (the project dir is deliberately empty).
    let entry = adapter_store_entry("demo", "1.0.0");
    let staged = stage_component(store.path(), b"\0asm-demo");
    install_component(&entry, &staged).expect("seed entry");

    let resolved =
        install_tofu("augentic", "demo", "1.0.0", store.path()).expect("idempotent tofu");
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
    let staged = stage_component(store.path(), b"\0asm-demo");
    install_component(&entry, &staged).expect("install");
    record_store_meta("demo", "1.0.0", &entry, "registrydigest").expect("record sidecar");

    // The sidecar is a writable sibling, never the read-only entry itself.
    let meta_path = cache::store_meta_path("demo", "1.0.0");
    assert!(meta_path.is_file(), "install must record a verify-on-read sidecar");
    assert_ne!(meta_path, entry, "the sidecar must be an entry sibling");

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
    let staged = stage_component(store.path(), b"\0asm-demo");
    install_component(&entry, &staged).expect("install");
    record_store_meta("demo", "1.0.0", &entry, "registrydigest").expect("record sidecar");

    // Corrupt the installed (read-only) file: relax its perms, rewrite the
    // bytes. The recomputed byte digest must no longer match the sidecar.
    let mut perms = fs::metadata(&entry).expect("stat").permissions();
    #[expect(
        clippy::permissions_set_readonly_false,
        reason = "test deliberately makes a read-only store entry writable to simulate on-disk corruption"
    )]
    perms.set_readonly(false);
    fs::set_permissions(&entry, perms).expect("relax perms");
    fs::write(&entry, b"\0asm-tampered").expect("corrupt file");

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
    let staged = stage_component(store.path(), b"\0asm-demo");
    install_component(&entry, &staged).expect("install");
    assert!(!cache::store_meta_path("demo", "1.0.0").exists(), "no sidecar was recorded");

    cache::verify_store_entry("demo", "1.0.0").expect("an absent sidecar fails open");
}
