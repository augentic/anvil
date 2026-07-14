//! Cache and immutable adapter-store behavior through public APIs.

use std::ffi::OsString;
use std::path::Path;

use diagnostics::cache::{
    adapter_store_entry, adapter_store_root, file_content_digest, project_cache_dir,
    read_store_meta, verify_store_entry, write_store_meta,
};

const STORE_ENV: &str = "SPECIFY_ADAPTER_STORE";

struct EnvGuard {
    previous: Option<OsString>,
}

impl Drop for EnvGuard {
    #[expect(unsafe_code, reason = "restore the store-root environment after the isolated test")]
    fn drop(&mut self) {
        // SAFETY: nextest runs each test in a separate process.
        unsafe {
            match self.previous.take() {
                Some(value) => std::env::set_var(STORE_ENV, value),
                None => std::env::remove_var(STORE_ENV),
            }
        }
    }
}

#[expect(unsafe_code, reason = "pin the store root for the isolated integration test")]
fn pin_store(path: &Path) -> EnvGuard {
    let guard = EnvGuard {
        previous: std::env::var_os(STORE_ENV),
    };
    // SAFETY: nextest runs each test in a separate process.
    unsafe { std::env::set_var(STORE_ENV, path) };
    guard
}

#[test]
fn project_paths_stable_distinct() {
    let a = project_cache_dir(Path::new("/some/project/a"));
    let b = project_cache_dir(Path::new("/some/project/b"));
    assert_ne!(a, b);
    assert_eq!(a.parent(), b.parent());
    assert_eq!(a, project_cache_dir(Path::new("/some/project/a")));
}

#[test]
fn sidecar_verifies_content() {
    let store = tempfile::tempdir().expect("store root");
    let _guard = pin_store(store.path());
    assert_eq!(adapter_store_root(), store.path());

    let entry = adapter_store_entry("demo-target", "1.2.0");
    assert_eq!(entry, store.path().join("demo-target@1.2.0.wasm"));
    std::fs::write(&entry, b"\0asm-component").expect("write component");

    let digest = file_content_digest(&entry);
    assert!(digest.starts_with("sha256:"));
    write_store_meta("demo-target", "1.2.0", &digest, Some("sha256:registry"))
        .expect("write sidecar");
    assert_eq!(read_store_meta("demo-target", "1.2.0").as_deref(), Some(digest.as_str()));
    verify_store_entry("demo-target", "1.2.0").expect("unchanged entry verifies");

    std::fs::write(&entry, b"\0asm-component-changed").expect("mutate component");
    let mismatch = verify_store_entry("demo-target", "1.2.0").expect_err("drift must fail");
    assert_eq!(mismatch.recorded, digest);
    assert_eq!(mismatch.actual, file_content_digest(&entry));
}
