//! Integration tests for the hydration kernel
//! (`workflow::hydrate`).
//!
//! Covers pinned-ref collection over `project.yaml` (the `adapter:`
//! pin, the `adapters:` prefetch list) and `plan.yaml` source pins;
//! the unpinned-prefetch refusal; the `hydrate` driver's warm-store
//! no-op probe, fetch-on-miss, frozen-mode refusal, and verify-on-read
//! gate; and the committed `.specify/adapters.lock` digest pin
//! (write-on-first-install, idempotent re-verify, drift refusal,
//! append-without-disturbing, frozen read-only). The fetch leg is a
//! test closure — nothing here touches the network.

use std::fs;
use std::path::{Path, PathBuf};

use error::Error;
use workflow::hydrate::{ResolvedAdapter, collect_refs, hydrate};
use workflow::init::AdapterPackage;

use crate::common;

/// `.specify/adapters.lock` under `root`.
fn lock_path(root: &Path) -> PathBuf {
    root.join(".specify").join("adapters.lock")
}

/// Write a `.specify/project.yaml` with the given body under `root`.
fn seed_project(root: &Path, body: &str) {
    let dir = root.join(".specify");
    fs::create_dir_all(&dir).expect("mkdir .specify");
    fs::write(dir.join("project.yaml"), body).expect("write project.yaml");
}

/// Stage a verified store entry (bytes + digest sidecar) for
/// `(name, version)` inside the scoped store root.
fn stage_store_entry(name: &str, version: &str, bytes: &str) -> PathBuf {
    let entry = schema::cache::adapter_store_entry(name, version);
    fs::create_dir_all(entry.parent().expect("store root")).expect("create store root");
    fs::write(&entry, bytes).expect("write store component");
    let digest = schema::cache::file_content_digest(&entry);
    schema::cache::write_store_meta(name, version, &digest, None).expect("write sidecar");
    entry
}

/// A fetch leg that must never be reached (warm-store and frozen-mode
/// assertions).
fn no_fetch(package: &AdapterPackage) -> Result<PathBuf, Error> {
    panic!("hydration must not fetch `{}@{}` in this test", package.name, package.version)
}

fn pinned(name: &str, version: &str) -> AdapterPackage {
    AdapterPackage::first_party(name, semver::Version::parse(version).expect("semver"))
}

#[test]
fn collects_project_and_plan_pins() {
    // Every pinned identity is collected — the `adapter:` target pin,
    // both prefetch forms, and the plan source pin — deduplicated on
    // (name, version); bare names (the unpinned `docs` binding, a bare
    // prefetch-free `adapter:`) are never hydration inputs.
    let tmp = tempfile::tempdir().expect("tempdir");
    seed_project(
        tmp.path(),
        "name: demo\n\
         adapter: specify:omnia@1.0.0\n\
         adapters:\n\
         - vectis@2.0.0\n\
         - specify:omnia@1.0.0\n",
    );
    fs::write(
        tmp.path().join("plan.yaml"),
        "name: demo\n\
         sources:\n\
         \x20 ts:\n\
         \x20   adapter: typescript\n\
         \x20   version: 1.2.3\n\
         \x20   path: ./src\n\
         \x20 docs:\n\
         \x20   adapter: documentation\n\
         \x20   path: ./docs\n\
         slices: []\n",
    )
    .expect("write plan.yaml");

    let refs = collect_refs(tmp.path()).expect("collect pinned refs");
    let identities: Vec<String> =
        refs.iter().map(|r| format!("{}:{}@{}", r.namespace, r.name, r.version)).collect();
    assert_eq!(
        identities,
        vec![
            "specify:omnia@1.0.0".to_string(),
            "specify:vectis@2.0.0".to_string(),
            "specify:typescript@1.2.3".to_string(),
        ],
        "adapter pin + prefetch list + plan pin, deduped, unpinned bindings excluded"
    );
}

#[test]
fn bare_adapter_no_plan_collects_nothing() {
    // A bare (or local-path) `adapter:` keeps project-local resolution
    // and an absent plan contributes nothing: the collected set is
    // empty and hydration over it is a no-op.
    let tmp = tempfile::tempdir().expect("tempdir");
    seed_project(tmp.path(), "name: demo\nadapter: omnia\n");

    let refs = collect_refs(tmp.path()).expect("collect refs");
    assert!(refs.is_empty(), "bare names are not hydration inputs: {refs:?}");
    assert!(hydrate(tmp.path(), &refs, false, &no_fetch).expect("empty hydrate").is_empty());
    assert!(!lock_path(tmp.path()).exists(), "an empty declared set writes no lock");
}

#[test]
fn unpinned_prefetch_entry_refused() {
    // Every `adapters:` entry must carry an exact pin — a bare name is
    // the typed `adapter-prefetch-unpinned`, naming the entry.
    let tmp = tempfile::tempdir().expect("tempdir");
    seed_project(tmp.path(), "name: demo\nadapter: omnia\nadapters:\n- vectis\n");

    let err = collect_refs(tmp.path()).expect_err("unpinned prefetch entry must be refused");
    let detail = err.to_string();
    assert!(
        matches!(
            err,
            Error::Diag {
                code: "adapter-prefetch-unpinned",
                ..
            }
        ),
        "{detail}"
    );
    assert!(detail.contains("`vectis`"), "error names the offending entry: {detail}");
}

#[test]
fn warm_store_is_noop_probe() {
    // A warm store hydrates without touching the fetch leg, returning
    // the resolved set (entry path + recorded sidecar digest) — the
    // idempotency property.
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));
    let entry = stage_store_entry("typescript", "1.2.3", "\0asm-ts");
    let digest = schema::cache::file_content_digest(&entry);

    let resolved = hydrate(tmp.path(), &[pinned("typescript", "1.2.3")], false, &no_fetch)
        .expect("warm hydrate");
    assert_eq!(
        resolved,
        vec![ResolvedAdapter {
            name: "typescript".to_string(),
            version: semver::Version::new(1, 2, 3),
            path: entry,
            digest,
        }]
    );
}

#[test]
fn miss_pulls_through_fetch_leg() {
    // A cold identity routes through the injected fetch leg exactly
    // once; duplicate refs collapse to one resolved entry.
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));

    let fetch = |package: &AdapterPackage| {
        Ok(stage_store_entry(&package.name, &package.version.to_string(), "\0asm-pulled"))
    };
    let refs = [pinned("vectis", "2.0.0"), pinned("vectis", "2.0.0")];
    let resolved = hydrate(tmp.path(), &refs, false, &fetch).expect("hydrate pulls on miss");
    assert_eq!(resolved.len(), 1, "duplicate identities collapse to one resolved entry");
    assert_eq!(resolved[0].name, "vectis");
    assert!(resolved[0].path.is_file(), "the fetched entry is materialized in the store");
}

#[test]
fn frozen_miss_is_typed_and_fetch_free() {
    // Frozen mode turns a would-be fetch into the typed
    // `adapter-not-installed` (`Error::Validation`, exit 2), naming
    // the identity and the literal sync command; the fetch leg is
    // never reached (`no_fetch` panics).
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));

    let err = hydrate(tmp.path(), &[pinned("omnia", "1.0.0")], true, &no_fetch)
        .expect_err("a frozen-mode miss must fail");
    let detail = err.to_string();
    assert!(
        matches!(&err, Error::Validation { code, .. } if code == "adapter-not-installed"),
        "{detail}"
    );
    assert!(detail.contains("omnia@1.0.0"), "error names the identity: {detail}");
    assert!(detail.contains("specify adapters sync"), "error names the sync command: {detail}");
}

#[test]
fn drifted_entry_refused() {
    // Verify-on-read: a store entry whose bytes no longer
    // match the recorded sidecar digest is `adapter-digest-mismatch`.
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));
    let entry = stage_store_entry("typescript", "1.2.3", "\0asm-ts");
    fs::write(&entry, "\0asm-drifted").expect("drift the entry");

    let err = hydrate(tmp.path(), &[pinned("typescript", "1.2.3")], false, &no_fetch)
        .expect_err("a drifted entry must fail verify-on-read");
    let detail = err.to_string();
    assert!(
        matches!(
            err,
            Error::Diag {
                code: "adapter-digest-mismatch",
                ..
            }
        ),
        "{detail}"
    );
    assert!(detail.contains("typescript@1.2.3"), "error names the identity: {detail}");
}

#[test]
fn first_hydration_writes_lock() {
    // First hydration pins every resolved identity in
    // `.specify/adapters.lock`: sorted keys, stable serialization,
    // trailing newline — the committed cross-machine digest pin.
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));
    let vectis = stage_store_entry("vectis", "2.0.0", "\0asm-vectis");
    let ts = stage_store_entry("typescript", "1.2.3", "\0asm-ts");

    // Refs deliberately unsorted — the lock sorts regardless.
    let refs = [pinned("vectis", "2.0.0"), pinned("typescript", "1.2.3")];
    hydrate(tmp.path(), &refs, false, &no_fetch).expect("first hydrate");

    let contents = fs::read_to_string(lock_path(tmp.path())).expect("lock written");
    assert_eq!(
        contents,
        format!(
            "version: 1\nadapters:\n  typescript@1.2.3: {}\n  vectis@2.0.0: {}\n",
            schema::cache::file_content_digest(&ts),
            schema::cache::file_content_digest(&vectis),
        ),
        "deterministic sorted lock bytes with a trailing newline"
    );
}

#[test]
fn rehydration_verifies_without_rewriting() {
    // Re-hydration against an unchanged store verifies clean against
    // the committed pin and leaves the lock untouched — no rewrite, so
    // the file is byte- and mtime-stable (idempotent).
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));
    stage_store_entry("typescript", "1.2.3", "\0asm-ts");

    let refs = [pinned("typescript", "1.2.3")];
    hydrate(tmp.path(), &refs, false, &no_fetch).expect("first hydrate");
    let lock = lock_path(tmp.path());
    let bytes = fs::read(&lock).expect("lock bytes");
    let mtime = fs::metadata(&lock).expect("lock metadata").modified().expect("mtime");

    hydrate(tmp.path(), &refs, false, &no_fetch).expect("re-hydrate");
    assert_eq!(fs::read(&lock).expect("lock bytes"), bytes, "lock bytes unchanged");
    assert_eq!(
        fs::metadata(&lock).expect("lock metadata").modified().expect("mtime"),
        mtime,
        "a clean re-verify must not rewrite the lock"
    );
}

#[test]
fn locked_digest_drift_refused() {
    // A store entry whose digest no longer matches the committed pin
    // is `adapter-digest-mismatch`, naming the identity and both
    // digests — hydration aborts before any guest would load. Drift is
    // simulated by editing the locked digest (the cross-machine case:
    // this machine's install differs from the one that authored the
    // pin).
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));
    let entry = stage_store_entry("typescript", "1.2.3", "\0asm-ts");
    let actual = schema::cache::file_content_digest(&entry);

    let refs = [pinned("typescript", "1.2.3")];
    hydrate(tmp.path(), &refs, false, &no_fetch).expect("first hydrate");
    let lock = lock_path(tmp.path());
    let pinned_digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let edited = fs::read_to_string(&lock).expect("lock contents").replace(&actual, pinned_digest);
    fs::write(&lock, edited).expect("edit the locked digest");

    let err = hydrate(tmp.path(), &refs, false, &no_fetch)
        .expect_err("a locked-digest mismatch must abort hydration");
    let detail = err.to_string();
    assert!(
        matches!(
            err,
            Error::Diag {
                code: "adapter-digest-mismatch",
                ..
            }
        ),
        "{detail}"
    );
    assert!(detail.contains("typescript@1.2.3"), "error names the identity: {detail}");
    assert!(detail.contains(pinned_digest), "error names the locked digest: {detail}");
    assert!(detail.contains(&actual), "error names the actual digest: {detail}");
}

#[test]
fn frozen_warm_store_leaves_lock_untouched() {
    // Frozen mode is strictly read-only on the committed lock: a warm
    // entry whose identity is new to the lock resolves and verifies,
    // but nothing is appended — no lock is created when absent, and an
    // existing lock survives byte-identical.
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));
    stage_store_entry("typescript", "1.2.3", "\0asm-ts");
    stage_store_entry("vectis", "2.0.0", "\0asm-vectis");

    let resolved = hydrate(tmp.path(), &[pinned("typescript", "1.2.3")], true, &no_fetch)
        .expect("frozen warm hydrate");
    assert_eq!(resolved.len(), 1, "the warm entry still resolves in frozen mode");
    assert!(!lock_path(tmp.path()).exists(), "a frozen hydration never creates the lock");

    hydrate(tmp.path(), &[pinned("typescript", "1.2.3")], false, &no_fetch).expect("pin the lock");
    let bytes = fs::read(lock_path(tmp.path())).expect("lock bytes");
    let refs = [pinned("typescript", "1.2.3"), pinned("vectis", "2.0.0")];
    hydrate(tmp.path(), &refs, true, &no_fetch).expect("frozen hydrate with a new identity");
    assert_eq!(
        fs::read(lock_path(tmp.path())).expect("lock bytes"),
        bytes,
        "frozen mode must not append the new-to-the-lock identity"
    );
}

#[test]
fn new_identity_appends_to_lock() {
    // A new pin appends its entry without disturbing the existing
    // ones; identities no longer declared stay pinned (pruning is a
    // non-goal — store entries are shared across projects).
    let tmp = tempfile::tempdir().expect("tempdir");
    let _store = common::scoped_store(&tmp.path().join("store"));
    let ts = stage_store_entry("typescript", "1.2.3", "\0asm-ts");
    let vectis = stage_store_entry("vectis", "2.0.0", "\0asm-vectis");

    hydrate(tmp.path(), &[pinned("typescript", "1.2.3")], false, &no_fetch).expect("first hydrate");
    hydrate(tmp.path(), &[pinned("vectis", "2.0.0")], false, &no_fetch)
        .expect("hydrate the new pin");

    let contents = fs::read_to_string(lock_path(tmp.path())).expect("lock contents");
    assert_eq!(
        contents,
        format!(
            "version: 1\nadapters:\n  typescript@1.2.3: {}\n  vectis@2.0.0: {}\n",
            schema::cache::file_content_digest(&ts),
            schema::cache::file_content_digest(&vectis),
        ),
        "the new identity appends; the undeclared existing entry survives"
    );
}
