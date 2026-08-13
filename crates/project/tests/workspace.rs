//! Private-workspace kernel coverage (RFC-87): snapshot round-trip
//! fidelity, workspace privacy, read-only views, discard-and-retry,
//! determinism, and garbage collection.

use std::path::Path;
use std::time::{Duration, SystemTime};

use project::snapshot::SnapshotId;
use project::workspace::{self, Access, FsObjects, Store};

struct Lab {
    _root: tempfile::TempDir,
    store: Store<FsObjects>,
    workspaces: std::path::PathBuf,
    source: std::path::PathBuf,
}

fn lab() -> Lab {
    let root = tempfile::tempdir().expect("tempdir");
    let store = Store::new(root.path().join("snapshots"));
    let workspaces = root.path().join("workspaces");
    let source = root.path().join("source");
    std::fs::create_dir_all(&source).expect("source dir");
    Lab {
        store,
        workspaces,
        source,
        _root: root,
    }
}

fn write(dir: &Path, rel: &str, bytes: &[u8]) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, bytes).expect("write");
}

#[cfg(unix)]
fn chmod_exec(path: &Path) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
}

mod round_trip {
    use super::*;

    /// Exec bits and symlink targets survive materialization.
    #[cfg(unix)]
    fn assert_unix_fidelity(root: &Path) {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(root.join("run.sh")).expect("meta").permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "exec bit survives materialization");
        let target = std::fs::read_link(root.join("link.rs")).expect("readlink");
        assert_eq!(target, Path::new("src/lib.rs"));
    }

    #[tokio::test]
    async fn full_fidelity() {
        let lab = lab();
        write(&lab.source, "src/lib.rs", b"pub fn hello() {}\n");
        write(&lab.source, "empty.txt", b"");
        write(&lab.source, "assets/logo.bin", &[0_u8, 159, 146, 150, 255]);
        write(&lab.source, "run.sh", b"#!/bin/sh\necho hi\n");
        #[cfg(unix)]
        chmod_exec(&lab.source.join("run.sh"));
        #[cfg(unix)]
        std::os::unix::fs::symlink("src/lib.rs", lab.source.join("link.rs")).expect("symlink");

        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");

        // Byte-identical materialization, modes and links included.
        assert_eq!(
            std::fs::read(ws.root.join("assets/logo.bin")).expect("read"),
            vec![0_u8, 159, 146, 150, 255]
        );
        assert_eq!(std::fs::read(ws.root.join("empty.txt")).expect("read"), Vec::<u8>::new());
        #[cfg(unix)]
        assert_unix_fidelity(&ws.root);

        // Mutate: add, edit, delete, and an empty-file addition.
        write(&ws.root, "src/new.rs", b"pub struct New;\n");
        write(&ws.root, "src/lib.rs", b"pub fn hello() { println!(); }\n");
        std::fs::remove_file(ws.root.join("assets/logo.bin")).expect("rm");
        write(&ws.root, "blank", b"");

        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).await.expect("capture");
        assert_eq!(patch.base, base);
        assert_eq!(patch.touched, vec!["assets/logo.bin", "blank", "src/lib.rs", "src/new.rs"]);

        // The result snapshot materializes to the exact mutated tree.
        let out = lab.source.parent().expect("parent").join("out");
        lab.store.materialize(&patch.result, &out).await.expect("materialize");
        assert_eq!(
            std::fs::read_to_string(out.join("src/lib.rs")).expect("read"),
            "pub fn hello() { println!(); }\n"
        );
        assert!(!out.join("assets").join("logo.bin").exists());
        assert!(out.join("blank").exists());
    }

    #[tokio::test]
    async fn streamed_file_identity() {
        let lab = lab();
        let payload: Vec<u8> = (0_u8..=250).cycle().take(200_000).collect();
        write(&lab.source, "big.bin", &payload);

        let first = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let again = lab.store.snapshot(&lab.source).await.expect("resnapshot");
        assert_eq!(first, again, "streamed ingest is deterministic");

        let out = lab.source.parent().expect("parent").join("out");
        lab.store.materialize(&first, &out).await.expect("materialize");
        assert_eq!(std::fs::read(out.join("big.bin")).expect("read"), payload);
        assert_eq!(
            diagnostics::digest::sha256_path(&out.join("big.bin")).expect("hash"),
            diagnostics::digest::sha256_hex(&payload),
            "blob identity is SHA-256 of the file bytes"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn mode_change_touched() {
        let lab = lab();
        write(&lab.source, "tool", b"#!/bin/sh\n");
        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");
        chmod_exec(&ws.root.join("tool"));
        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).await.expect("capture");
        assert_eq!(patch.touched, vec!["tool"]);
    }

    #[tokio::test]
    async fn apply_writes_touched() {
        let lab = lab();
        write(&lab.source, "src/lib.rs", b"pub fn hello() {}\n");
        write(&lab.source, "contracts/api.yaml", b"openapi: 3.1.0\n");
        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");
        write(&ws.root, "src/new.rs", b"pub struct New;\n");
        std::fs::remove_file(ws.root.join("src/lib.rs")).expect("rm");
        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).await.expect("capture");

        // Between capture and apply the product tree moves on — the
        // deterministic merge folds the contracts baseline. Apply must
        // write only the patch's touched paths and leave the fold.
        write(&lab.source, "contracts/api.yaml", b"openapi: 3.1.0 # folded\n");
        lab.store.apply(&patch, &lab.source).await.expect("apply");
        assert_eq!(
            std::fs::read_to_string(lab.source.join("contracts/api.yaml")).expect("read"),
            "openapi: 3.1.0 # folded\n",
            "apply never rewinds paths the patch did not touch"
        );
        assert_eq!(
            std::fs::read_to_string(lab.source.join("src/new.rs")).expect("read"),
            "pub struct New;\n"
        );
        assert!(!lab.source.join("src/lib.rs").exists(), "touched deletions apply");
    }

    /// Snapshots carry relative links only: an absolute target cannot
    /// be re-created inside a sandboxed guest, so refusal is typed and
    /// symmetric at snapshot time.
    #[cfg(unix)]
    #[tokio::test]
    async fn absolute_symlink_target() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        std::os::unix::fs::symlink("/etc/hosts", lab.source.join("abs")).expect("symlink");
        let err =
            lab.store.snapshot(&lab.source).await.expect_err("must refuse an absolute target");
        assert!(err.to_string().contains("absolute target"), "unexpected error: {err}");
    }

    #[tokio::test]
    async fn unchanged_tree_empty() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");
        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).await.expect("capture");
        assert_eq!(patch.base, patch.result);
        assert!(patch.touched.is_empty());
    }
}

/// The migration oracle: the manifest encoding is canonical, so this
/// exact tree must always hash to this exact snapshot id — across the
/// native kernel, the in-guest kernel, and any object backend. Exec
/// bits and symlink targets participate in the digest, so any mode or
/// link infidelity fails here first.
#[cfg(unix)]
mod golden {
    use super::*;

    const BASE: &str = "sha256:601f78baead19ad4ed7c1cdf9bae8581ea83582d406cccc03ee9ee50ee4579f1";
    const FLIPPED: &str = "sha256:e1237523c4dfe1e9f84c93b13132e3bfa09a69de215fcf125319b431316888c5";

    fn fixture(lab: &Lab) {
        write(&lab.source, "src/lib.rs", b"pub fn hello() {}\n");
        write(&lab.source, "run.sh", b"#!/bin/sh\necho hi\n");
        chmod_exec(&lab.source.join("run.sh"));
        std::os::unix::fs::symlink("src/lib.rs", lab.source.join("link.rs")).expect("symlink");
    }

    #[tokio::test]
    async fn pinned_snapshot_id() {
        let lab = lab();
        fixture(&lab);
        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        assert_eq!(base.as_str(), BASE, "canonical tree digest drifted");
    }

    /// An exec → plain flip changes the digest and survives `apply`
    /// onto a live tree.
    #[tokio::test]
    async fn exec_flip_round_trip() {
        use std::os::unix::fs::PermissionsExt as _;

        let lab = lab();
        fixture(&lab);
        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");
        std::fs::set_permissions(ws.root.join("run.sh"), std::fs::Permissions::from_mode(0o644))
            .expect("chmod plain");
        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).await.expect("capture");
        assert_eq!(patch.result.as_str(), FLIPPED, "flipped tree digest drifted");
        assert_eq!(patch.touched, vec!["run.sh"]);

        lab.store.apply(&patch, &lab.source).await.expect("apply");
        let mode = std::fs::metadata(lab.source.join("run.sh")).expect("meta").permissions().mode();
        assert_eq!(mode & 0o111, 0, "apply must clear the executable bit");
    }
}

mod privacy {
    use super::*;

    #[tokio::test]
    async fn two_preparations_never() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let one = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare one");
        let two = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare two");
        assert_ne!(one.root, two.root);

        // Concurrent divergence: each capture sees only its own writes.
        write(&one.root, "one.txt", b"1");
        write(&two.root, "two.txt", b"2");
        let patch_one =
            workspace::capture(&lab.store, &lab.workspaces, &one.id).await.expect("capture one");
        let patch_two =
            workspace::capture(&lab.store, &lab.workspaces, &two.id).await.expect("capture two");
        assert_eq!(patch_one.touched, vec!["one.txt"]);
        assert_eq!(patch_two.touched, vec!["two.txt"]);
    }

    #[tokio::test]
    async fn source_never_touched() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");
        write(&ws.root, "b.txt", b"b");
        workspace::capture(&lab.store, &lab.workspaces, &ws.id).await.expect("capture");
        assert!(!lab.source.join("b.txt").exists(), "the snapshotted tree stays untouched");
    }

    #[tokio::test]
    async fn artifacts_vcs_state() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        write(&lab.source, ".git/config", b"[core]");
        write(&lab.source, ".emery/project.yaml", b"emery: 1.0.0");
        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");
        assert!(!ws.root.join(".git").exists());
        assert!(!ws.root.join(".emery").exists());
    }
}

mod access {
    use super::*;

    #[tokio::test]
    async fn read_view_refuses_capture() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let view =
            workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: false })
                .await
                .expect("prepare view");
        let err = workspace::capture(&lab.store, &lab.workspaces, &view.id)
            .await
            .expect_err("capture must refuse a view");
        assert!(err.to_string().contains("read-only"), "unexpected error: {err}");
        workspace::discard(&lab.workspaces, &view.id).expect("discard view");
    }

    #[test]
    fn traversal_id_refused() {
        let lab = lab();
        for id in ["../escape", "a/b", "..", ""] {
            let err = workspace::discard(&lab.workspaces, id).expect_err("must refuse");
            assert!(err.to_string().contains("workspace id"), "unexpected error: {err}");
        }
    }
}

mod durability {
    use super::*;

    #[tokio::test]
    async fn discard_loses_nothing() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");
        write(&ws.root, "b.txt", b"b");
        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).await.expect("capture");

        workspace::discard(&lab.workspaces, &ws.id).expect("discard");
        workspace::discard(&lab.workspaces, &ws.id).expect("discard is idempotent");
        assert!(!ws.root.exists());

        // The completed result survives by digest after discard.
        let out = lab.workspaces.join("re-materialized");
        lab.store.materialize(&patch.result, &out).await.expect("materialize after discard");
        assert_eq!(std::fs::read_to_string(out.join("b.txt")).expect("read"), "b");

        // Retry needs no recovery: a fresh workspace from the recorded base.
        let retry =
            workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
                .await
                .expect("re-prepare");
        assert!(retry.root.join("a.txt").exists());
        assert!(!retry.root.join("b.txt").exists(), "retry starts from the base, not the result");
    }

    #[tokio::test]
    async fn determinism_across() {
        let lab = lab();
        write(&lab.source, "x/y.txt", b"same");
        let elsewhere = lab.source.parent().expect("parent").join("elsewhere");
        write(&elsewhere, "x/y.txt", b"same");
        let a = lab.store.snapshot(&lab.source).await.expect("snapshot a");
        let b = lab.store.snapshot(&elsewhere).await.expect("snapshot b");
        assert_eq!(a, b, "equal trees hash to one snapshot identity");

        let out_a = lab.workspaces.join("out-a");
        let out_b = lab.workspaces.join("out-b");
        lab.store.materialize(&a, &out_a).await.expect("materialize a");
        lab.store.materialize(&b, &out_b).await.expect("materialize b");
        assert_eq!(
            std::fs::read(out_a.join("x/y.txt")).expect("read"),
            std::fs::read(out_b.join("x/y.txt")).expect("read"),
        );
    }

    #[tokio::test]
    async fn missing_base_is_typed() {
        let lab = lab();
        let absent = SnapshotId::from_digest(&"0".repeat(64));
        let err =
            workspace::prepare(&lab.store, &lab.workspaces, &absent, Access { writable: true })
                .await
                .expect_err("must refuse an unknown base");
        assert!(err.to_string().contains("not in the store"), "unexpected error: {err}");
    }
}

mod gc {
    use super::*;

    #[tokio::test]
    async fn sweeps_only_stale_entries() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let stale =
            workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
                .await
                .expect("prepare stale");
        let fresh =
            workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
                .await
                .expect("prepare fresh");

        // Everything is newer than a cutoff in the past: nothing sweeps.
        let past = SystemTime::now() - Duration::from_hours(1);
        assert_eq!(workspace::gc(&lab.workspaces, past).expect("gc"), 0);

        // Age the stale workspace behind the cutoff.
        let old = SystemTime::now() - Duration::from_hours(2);
        set_mtime(&stale.root, old);
        set_mtime(&lab.workspaces.join(format!("{}.yaml", stale.id)), old);

        assert_eq!(workspace::gc(&lab.workspaces, past).expect("gc"), 1);
        assert!(!stale.root.exists());
        assert!(fresh.root.exists());
        workspace::capture(&lab.store, &lab.workspaces, &fresh.id)
            .await
            .expect("surviving workspace still captures");
    }

    fn set_mtime(path: &Path, to: SystemTime) {
        let file = std::fs::OpenOptions::new().read(true).open(path).expect("open");
        file.set_modified(to).expect("set mtime");
    }
}

mod sweep {
    use super::*;

    /// The change-scoped collection: dead roots' objects go, objects
    /// shared with a live root stay, and the live snapshot still
    /// materializes afterwards.
    #[tokio::test]
    async fn shared_objects_survive() {
        let lab = lab();
        write(&lab.source, "shared.txt", b"kept by both");
        write(&lab.source, "dead-only.txt", b"only the archived change");
        let dead = lab.store.snapshot(&lab.source).await.expect("snapshot dead");

        std::fs::remove_file(lab.source.join("dead-only.txt")).expect("rm");
        write(&lab.source, "live-only.txt", b"still live");
        let live = lab.store.snapshot(&lab.source).await.expect("snapshot live");

        let removed = lab
            .store
            .sweep(std::slice::from_ref(&dead), std::slice::from_ref(&live))
            .await
            .expect("sweep");
        // The dead manifest and its unique blob go; the shared blob stays.
        assert_eq!(removed, 2);
        assert!(!lab.store.contains(&dead).await, "dead root collected");
        assert!(lab.store.contains(&live).await, "live root survives");

        let out = lab.workspaces.join("post-sweep");
        lab.store.materialize(&live, &out).await.expect("live snapshot intact");
        assert_eq!(std::fs::read_to_string(out.join("shared.txt")).expect("read"), "kept by both");
    }

    /// Absent roots (never frozen, or already collected) are skipped;
    /// sweeping twice deletes nothing new.
    #[tokio::test]
    async fn absent_roots_skipped() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        let dead = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let never_frozen = SnapshotId::from_digest(&"0".repeat(64));

        let removed =
            lab.store.sweep(&[dead.clone(), never_frozen.clone()], &[]).await.expect("sweep");
        assert_eq!(removed, 2, "manifest plus one blob");
        assert_eq!(
            lab.store.sweep(&[dead, never_frozen], &[]).await.expect("re-sweep"),
            0,
            "a second sweep finds nothing"
        );
    }
}
