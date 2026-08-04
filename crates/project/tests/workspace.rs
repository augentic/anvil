//! Private-workspace kernel coverage (RFC-87): snapshot round-trip
//! fidelity, workspace privacy, read-only views, discard-and-retry,
//! determinism, and garbage collection.

use std::path::Path;
use std::time::{Duration, SystemTime};

use project::snapshot::SnapshotId;
use project::workspace::{self, Access, Store};

struct Lab {
    _root: tempfile::TempDir,
    store: Store,
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

    #[test]
    fn full_fidelity() {
        let lab = lab();
        write(&lab.source, "src/lib.rs", b"pub fn hello() {}\n");
        write(&lab.source, "empty.txt", b"");
        write(&lab.source, "assets/logo.bin", &[0_u8, 159, 146, 150, 255]);
        write(&lab.source, "run.sh", b"#!/bin/sh\necho hi\n");
        #[cfg(unix)]
        chmod_exec(&lab.source.join("run.sh"));
        #[cfg(unix)]
        std::os::unix::fs::symlink("src/lib.rs", lab.source.join("link.rs")).expect("symlink");

        let base = lab.store.snapshot(&lab.source).expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
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

        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).expect("capture");
        assert_eq!(patch.base, base);
        assert_eq!(patch.touched, vec!["assets/logo.bin", "blank", "src/lib.rs", "src/new.rs"]);

        // The result snapshot materializes to the exact mutated tree.
        let out = lab.source.parent().expect("parent").join("out");
        lab.store.materialize(&patch.result, &out).expect("materialize");
        assert_eq!(
            std::fs::read_to_string(out.join("src/lib.rs")).expect("read"),
            "pub fn hello() { println!(); }\n"
        );
        assert!(!out.join("assets").join("logo.bin").exists());
        assert!(out.join("blank").exists());
    }

    #[cfg(unix)]
    #[test]
    fn mode_only_change_is_touched() {
        let lab = lab();
        write(&lab.source, "tool", b"#!/bin/sh\n");
        let base = lab.store.snapshot(&lab.source).expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .expect("prepare");
        chmod_exec(&ws.root.join("tool"));
        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).expect("capture");
        assert_eq!(patch.touched, vec!["tool"]);
    }

    #[test]
    fn apply_writes_only_touched_paths() {
        let lab = lab();
        write(&lab.source, "src/lib.rs", b"pub fn hello() {}\n");
        write(&lab.source, "contracts/api.yaml", b"openapi: 3.1.0\n");
        let base = lab.store.snapshot(&lab.source).expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .expect("prepare");
        write(&ws.root, "src/new.rs", b"pub struct New;\n");
        std::fs::remove_file(ws.root.join("src/lib.rs")).expect("rm");
        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).expect("capture");

        // Between capture and apply the product tree moves on — the
        // deterministic merge folds the contracts baseline. Apply must
        // write only the patch's touched paths and leave the fold.
        write(&lab.source, "contracts/api.yaml", b"openapi: 3.1.0 # folded\n");
        lab.store.apply(&patch, &lab.source).expect("apply");
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

    #[test]
    fn unchanged_tree_is_empty_patch() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        let base = lab.store.snapshot(&lab.source).expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .expect("prepare");
        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).expect("capture");
        assert_eq!(patch.base, patch.result);
        assert!(patch.touched.is_empty());
    }
}

mod privacy {
    use super::*;

    #[test]
    fn two_preparations_never_share_a_directory() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        let base = lab.store.snapshot(&lab.source).expect("snapshot");
        let one = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .expect("prepare one");
        let two = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .expect("prepare two");
        assert_ne!(one.root, two.root);

        // Concurrent divergence: each capture sees only its own writes.
        write(&one.root, "one.txt", b"1");
        write(&two.root, "two.txt", b"2");
        let patch_one =
            workspace::capture(&lab.store, &lab.workspaces, &one.id).expect("capture one");
        let patch_two =
            workspace::capture(&lab.store, &lab.workspaces, &two.id).expect("capture two");
        assert_eq!(patch_one.touched, vec!["one.txt"]);
        assert_eq!(patch_two.touched, vec!["two.txt"]);
    }

    #[test]
    fn source_never_touched() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        let base = lab.store.snapshot(&lab.source).expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .expect("prepare");
        write(&ws.root, "b.txt", b"b");
        workspace::capture(&lab.store, &lab.workspaces, &ws.id).expect("capture");
        assert!(!lab.source.join("b.txt").exists(), "the snapshotted tree stays untouched");
    }

    #[test]
    fn artifacts_and_vcs_state_excluded() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        write(&lab.source, ".git/config", b"[core]");
        write(&lab.source, ".emery/project.yaml", b"emery: 1.0.0");
        let base = lab.store.snapshot(&lab.source).expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .expect("prepare");
        assert!(!ws.root.join(".git").exists());
        assert!(!ws.root.join(".emery").exists());
    }
}

mod access {
    use super::*;

    #[test]
    fn read_only_view_refuses_capture() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        let base = lab.store.snapshot(&lab.source).expect("snapshot");
        let view =
            workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: false })
                .expect("prepare view");
        let err = workspace::capture(&lab.store, &lab.workspaces, &view.id)
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

    #[test]
    fn discard_loses_nothing_and_retry_reprepares() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        let base = lab.store.snapshot(&lab.source).expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .expect("prepare");
        write(&ws.root, "b.txt", b"b");
        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).expect("capture");

        workspace::discard(&lab.workspaces, &ws.id).expect("discard");
        workspace::discard(&lab.workspaces, &ws.id).expect("discard is idempotent");
        assert!(!ws.root.exists());

        // The completed result survives by digest after discard.
        let out = lab.workspaces.join("re-materialized");
        lab.store.materialize(&patch.result, &out).expect("materialize after discard");
        assert_eq!(std::fs::read_to_string(out.join("b.txt")).expect("read"), "b");

        // Retry needs no recovery: a fresh workspace from the recorded base.
        let retry =
            workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
                .expect("re-prepare");
        assert!(retry.root.join("a.txt").exists());
        assert!(!retry.root.join("b.txt").exists(), "retry starts from the base, not the result");
    }

    #[test]
    fn determinism_across_locations() {
        let lab = lab();
        write(&lab.source, "x/y.txt", b"same");
        let elsewhere = lab.source.parent().expect("parent").join("elsewhere");
        write(&elsewhere, "x/y.txt", b"same");
        let a = lab.store.snapshot(&lab.source).expect("snapshot a");
        let b = lab.store.snapshot(&elsewhere).expect("snapshot b");
        assert_eq!(a, b, "equal trees hash to one snapshot identity");

        let out_a = lab.workspaces.join("out-a");
        let out_b = lab.workspaces.join("out-b");
        lab.store.materialize(&a, &out_a).expect("materialize a");
        lab.store.materialize(&b, &out_b).expect("materialize b");
        assert_eq!(
            std::fs::read(out_a.join("x/y.txt")).expect("read"),
            std::fs::read(out_b.join("x/y.txt")).expect("read"),
        );
    }

    #[test]
    fn missing_base_is_typed() {
        let lab = lab();
        let absent = SnapshotId::from_digest(&"0".repeat(64));
        let err =
            workspace::prepare(&lab.store, &lab.workspaces, &absent, Access { writable: true })
                .expect_err("must refuse an unknown base");
        assert!(err.to_string().contains("not in the store"), "unexpected error: {err}");
    }
}

mod gc {
    use super::*;

    #[test]
    fn sweeps_only_stale_entries() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        let base = lab.store.snapshot(&lab.source).expect("snapshot");
        let stale =
            workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
                .expect("prepare stale");
        let fresh =
            workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
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
            .expect("surviving workspace still captures");
    }

    fn set_mtime(path: &Path, to: SystemTime) {
        let file = std::fs::OpenOptions::new().read(true).open(path).expect("open");
        file.set_modified(to).expect("set mtime");
    }
}
