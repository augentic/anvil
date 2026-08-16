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

/// RFC-96 D6: `compose(base, patches)` — same-base disjoint patches
/// only, deterministic deployment-independent identity, typed
/// refusals with no store mutation.
mod compose {
    use super::*;

    /// Capture one mutation of `base` as a patch.
    async fn patch_of(
        lab: &Lab, base: &SnapshotId, mutate: impl FnOnce(&Path),
    ) -> project::snapshot::CodePatch {
        let ws = workspace::prepare(&lab.store, &lab.workspaces, base, Access { writable: true })
            .await
            .expect("prepare");
        mutate(&ws.root);
        workspace::capture(&lab.store, &lab.workspaces, &ws.id).await.expect("capture")
    }

    #[tokio::test]
    async fn disjoint_deterministic() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a\n");
        write(&lab.source, "b.txt", b"b\n");
        write(&lab.source, "c.txt", b"c\n");
        let base = lab.store.snapshot(&lab.source).await.expect("base");

        let edits = patch_of(&lab, &base, |root| {
            write(root, "a.txt", b"a2\n");
            write(root, "d.txt", b"d\n");
        })
        .await;
        let deletes = patch_of(&lab, &base, |root| {
            std::fs::remove_file(root.join("c.txt")).expect("rm");
            write(root, "b.txt", b"b2\n");
        })
        .await;

        let composed = workspace::compose(&lab.store, &base, &[edits.clone(), deletes.clone()])
            .await
            .expect("compose");
        assert_eq!(composed.base, base);
        assert_eq!(composed.touched, vec!["a.txt", "b.txt", "c.txt", "d.txt"]);

        // The composed identity equals a direct snapshot of the same
        // tree — deterministic and deployment-independent.
        let manual = lab.source.parent().expect("parent").join("manual");
        std::fs::create_dir_all(&manual).expect("manual dir");
        write(&manual, "a.txt", b"a2\n");
        write(&manual, "b.txt", b"b2\n");
        write(&manual, "d.txt", b"d\n");
        let oracle = lab.store.snapshot(&manual).await.expect("oracle");
        assert_eq!(composed.result, oracle, "composed identity equals the assembled tree");

        let again =
            workspace::compose(&lab.store, &base, &[edits, deletes]).await.expect("recompose");
        assert_eq!(again.result, composed.result, "replay mints the same identity");
    }

    #[tokio::test]
    async fn overlap_refused() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a\n");
        let base = lab.store.snapshot(&lab.source).await.expect("base");
        let first = patch_of(&lab, &base, |root| write(root, "a.txt", b"one\n")).await;
        let second = patch_of(&lab, &base, |root| write(root, "a.txt", b"two\n")).await;

        let err = workspace::compose(&lab.store, &base, &[first, second])
            .await
            .expect_err("overlapping touched sets must refuse");
        assert!(err.to_string().contains("workspace-compose-overlap"), "{err}");
        assert!(err.to_string().contains("a.txt"), "the refusal names the path: {err}");
    }

    #[tokio::test]
    async fn base_mismatch_refused() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a\n");
        let base = lab.store.snapshot(&lab.source).await.expect("base");
        write(&lab.source, "a.txt", b"moved\n");
        let moved = lab.store.snapshot(&lab.source).await.expect("moved");
        let stale = patch_of(&lab, &moved, |root| write(root, "a.txt", b"edit\n")).await;

        let err = workspace::compose(&lab.store, &base, &[stale])
            .await
            .expect_err("a patch off another base must refuse");
        assert!(err.to_string().contains("workspace-compose-base-mismatch"), "{err}");
    }

    #[tokio::test]
    async fn empty_is_base() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a\n");
        let base = lab.store.snapshot(&lab.source).await.expect("base");
        let composed = workspace::compose(&lab.store, &base, &[]).await.expect("compose");
        assert_eq!(composed.base, base);
        assert_eq!(composed.result, base, "an empty patch set composes to the base itself");
        assert!(composed.touched.is_empty());
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

    /// An exec → plain flip changes the digest and survives
    /// materialization of the captured result.
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

        let out = lab.source.parent().expect("parent").join("flipped");
        lab.store.materialize(&patch.result, &out).await.expect("materialize");
        let mode = std::fs::metadata(out.join("run.sh")).expect("meta").permissions().mode();
        assert_eq!(mode & 0o111, 0, "captured result must clear the executable bit");
    }
}

/// RFC-105 snapshot membership: kernel excludes plus the tree's own
/// `.gitignore` decide what enters a snapshot and `touched`.
mod membership {
    use super::*;

    /// AC1 + AC5 (capture side): an ignored `target/` never reaches
    /// the result id or `touched`; unignored build output still lands.
    #[tokio::test]
    async fn ignored_output_excluded() {
        let lab = lab();
        write(&lab.source, "src/lib.rs", b"pub fn hello() {}\n");
        write(&lab.source, ".gitignore", b"target/\n");
        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");

        // The build writes compiler output and product code alike.
        write(&ws.root, "target/foo.o", b"\x7fELF");
        write(&ws.root, "mock-build/greeting.md", b"hello\n");
        write(&ws.root, "src/new.rs", b"pub struct New;\n");

        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).await.expect("capture");
        assert_eq!(patch.touched, vec!["mock-build/greeting.md", "src/new.rs"]);

        let out = lab.workspaces.join("result");
        lab.store.materialize(&patch.result, &out).await.expect("materialize");
        assert!(!out.join("target").exists(), "ignored output enters no snapshot");
        assert_eq!(
            std::fs::read_to_string(out.join("mock-build/greeting.md")).expect("read"),
            "hello\n",
            "unignored build output still lands"
        );
    }

    /// AC1 (no-ignore side) + AC5: a tree without a matching ignore
    /// admits everything except kernel excludes — today's behaviour.
    #[tokio::test]
    async fn no_gitignore_admits() {
        let lab = lab();
        write(&lab.source, "src/lib.rs", b"pub fn hello() {}\n");
        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");
        write(&ws.root, "target/foo.o", b"\x7fELF");
        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).await.expect("capture");
        assert_eq!(patch.touched, vec!["target/foo.o"]);
    }

    /// AC2: freeze of a checkout that carries VCS state and a local
    /// `target/` reads the tree only — `.git` bytes stay untouched and
    /// the ignored output stays out of the base snapshot.
    #[tokio::test]
    async fn freeze_reads_only() {
        let lab = lab();
        write(&lab.source, "src/lib.rs", b"pub fn hello() {}\n");
        write(&lab.source, ".gitignore", b"target/\n");
        write(&lab.source, "target/foo.o", b"\x7fELF");
        write(&lab.source, ".git/index", b"DIRC-operator-index");
        write(&lab.source, ".git/config", b"[core]");

        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        assert_eq!(
            std::fs::read(lab.source.join(".git/index")).expect("read"),
            b"DIRC-operator-index",
            "freeze never writes the operator index"
        );

        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");
        assert!(!ws.root.join("target").exists(), "dirty checkout output leaves the wave base");
        assert!(!ws.root.join(".git").exists(), "workspaces stay gitless");
        assert_eq!(
            std::fs::read_to_string(ws.root.join(".gitignore")).expect("read"),
            "target/\n",
            ".gitignore is product and rides the snapshot"
        );
    }

    /// AC4: kernel excludes win — a `.gitignore` negation cannot admit
    /// `.git` or a nested change home (`.emery/change`). Durable
    /// `.emery/` state and root plan files stay in the tree.
    #[tokio::test]
    async fn negation_kernel_excludes() {
        let lab = lab();
        write(&lab.source, "a.txt", b"a");
        write(&lab.source, ".gitignore", b"!.git/\n!.emery/change/\n");
        write(&lab.source, ".git/config", b"[core]");
        write(&lab.source, ".emery/project.yaml", b"emery: 1.0.0");
        write(&lab.source, ".emery/change/plan.yaml", b"name: demo");
        write(&lab.source, "plan.yaml", b"name: demo");
        write(&lab.source, "change.md", b"# change");

        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");
        assert!(!ws.root.join(".git").exists());
        assert!(!ws.root.join(".emery/change").exists());
        assert!(ws.root.join(".emery/project.yaml").exists());
        assert!(ws.root.join("plan.yaml").exists());
        assert!(ws.root.join("change.md").exists());
        assert!(ws.root.join("a.txt").exists());
    }

    /// Nested `.gitignore` files scope to their directory, and a
    /// whitelist (`!pattern`) re-admits within the same file.
    #[tokio::test]
    async fn nested_and_whitelist() {
        let lab = lab();
        write(&lab.source, "pkg/.gitignore", b"out/\n*.log\n!keep.log\n");
        write(&lab.source, "pkg/out/junk.o", b"junk");
        write(&lab.source, "pkg/debug.log", b"noise");
        write(&lab.source, "pkg/keep.log", b"signal");
        write(&lab.source, "out/data.txt", b"product");

        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");
        assert!(!ws.root.join("pkg/out").exists(), "nested ignore applies beneath its dir");
        assert!(!ws.root.join("pkg/debug.log").exists());
        assert!(ws.root.join("pkg/keep.log").exists(), "whitelist re-admits");
        assert!(ws.root.join("out/data.txt").exists(), "nested ignore does not leak upward");
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
}

mod boundary {
    use project::snapshot;

    use super::*;

    #[test]
    fn ignore_policy() {
        assert!(snapshot::ignored(".git"));
        assert!(snapshot::ignored("vendor/.git"));
        assert!(snapshot::ignored(".emery/change"));
        assert!(snapshot::ignored("nested/.emery/change"));
        assert!(!snapshot::ignored(".emery"));
        assert!(!snapshot::ignored(".emery/project.yaml"));
        assert!(!snapshot::ignored(".emery/specs/a/spec.md"));
        assert!(!snapshot::ignored(".emery/decisions/d.md"));
        assert!(!snapshot::ignored("src/lib.rs"));
        assert!(!snapshot::ignored("change.md"));
        assert!(!snapshot::ignored("plan.yaml"));
        assert!(!snapshot::ignored("discovery.md"));
    }

    /// Durable `.emery/` state survives freeze → prepare → capture;
    /// `.git` and the nested change home never appear. A prepared
    /// workspace exposes the baseline.
    #[tokio::test]
    async fn durable_round_trip() {
        let lab = lab();
        write(&lab.source, "src/lib.rs", b"pub fn hello() {}\n");
        write(&lab.source, ".git/config", b"[core]\n");
        write(&lab.source, ".emery/project.yaml", b"name: demo\nadapter: mock\nrules: {}\n");
        write(&lab.source, ".emery/specs/greeting/spec.md", b"# Greeting\n");
        write(&lab.source, ".emery/decisions/ttl.md", b"# TTL\n");
        write(&lab.source, ".emery/change/plan.yaml", b"name: demo\nsources: {}\nentries: []\n");
        write(&lab.source, ".emery/change/change.md", b"# Change\n");
        write(&lab.source, "vendor/.emery/change/plan.yaml", b"name: nested\n");
        write(&lab.source, "vendor/.emery/project.yaml", b"name: vendor\n");

        let base = lab.store.snapshot(&lab.source).await.expect("snapshot");
        let ws = workspace::prepare(&lab.store, &lab.workspaces, &base, Access { writable: true })
            .await
            .expect("prepare");

        assert!(!ws.root.join(".git").exists(), ".git is never product tree");
        assert!(
            !ws.root.join(".emery/change").exists(),
            "the nested change home is never product tree"
        );
        assert!(
            !ws.root.join("vendor/.emery/change").exists(),
            "a nested change home at any depth is excluded"
        );
        assert_eq!(
            std::fs::read_to_string(ws.root.join(".emery/project.yaml")).expect("project.yaml"),
            "name: demo\nadapter: mock\nrules: {}\n"
        );
        assert_eq!(
            std::fs::read_to_string(ws.root.join(".emery/specs/greeting/spec.md")).expect("spec"),
            "# Greeting\n"
        );
        assert_eq!(
            std::fs::read_to_string(ws.root.join(".emery/decisions/ttl.md")).expect("decision"),
            "# TTL\n"
        );
        assert_eq!(
            std::fs::read_to_string(ws.root.join("vendor/.emery/project.yaml")).expect("vendor"),
            "name: vendor\n"
        );

        // An agent write into the change home is not product tree.
        write(&ws.root, ".emery/change/sneak.yaml", b"nope\n");

        let patch = workspace::capture(&lab.store, &lab.workspaces, &ws.id).await.expect("capture");
        assert_eq!(patch.base, patch.result, "an untouched durable tree re-captures identically");
        assert!(patch.touched.is_empty(), "change-home writes are not captured");

        let out = lab.source.parent().expect("parent").join("captured");
        lab.store.materialize(&patch.result, &out).await.expect("materialize");
        assert!(out.join(".emery/specs/greeting/spec.md").is_file());
        assert!(out.join(".emery/project.yaml").is_file());
        assert!(out.join(".emery/decisions/ttl.md").is_file());
        assert!(!out.join(".emery/change").exists());
        assert!(!out.join(".git").exists());
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
