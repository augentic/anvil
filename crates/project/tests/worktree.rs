//! The D11 worktree-export kernel behind the `emery:vcs/worktree`
//! seam: placement (slot clone, in-place), the closed state table
//! (created / matched / rematerialized / dirty / no-rewind /
//! branch-diverged / checked-out-elsewhere / destination-conflict /
//! parent-unavailable / clone-failed), and index staging.

use std::path::{Path, PathBuf};
use std::process::Command;

use project::seam::{WorktreeError, WorktreeRequest, WorktreeState};
use project::snapshot::SnapshotId;
use project::vcs::worktree::{ExportEnv, export};
use project::workspace::{FsObjects, Store};

struct Lab {
    root: tempfile::TempDir,
    store: Store<FsObjects>,
    publication: PathBuf,
}

fn lab() -> Lab {
    let root = tempfile::tempdir().expect("tempdir");
    let store = Store::new(root.path().join("snapshots"));
    let publication = root.path().join("publication");
    Lab {
        root,
        store,
        publication,
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(["-c", "init.defaultBranch=main", "-c", "commit.gpgsign=false"])
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// An origin repository with one commit of `files`; returns its path
/// and head SHA.
fn origin(root: &Path, files: &[(&str, &str)]) -> (PathBuf, String) {
    let dir = root.join("origin");
    std::fs::create_dir_all(&dir).expect("origin");
    git(&dir, &["init", "--template="]);
    git(&dir, &["config", "user.email", "dev@example.com"]);
    git(&dir, &["config", "user.name", "Dev"]);
    commit(&dir, files)
}

/// Write `files` and commit them; returns the repo path and new head.
fn commit(dir: &Path, files: &[(&str, &str)]) -> (PathBuf, String) {
    for (rel, body) in files {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(path, body).expect("write");
    }
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "step"]);
    let sha = git(dir, &["rev-parse", "HEAD"]);
    (dir.to_path_buf(), sha)
}

/// Snapshot a fresh content tree of `files` into the lab store.
async fn cid(lab: &Lab, name: &str, files: &[(&str, &str)]) -> SnapshotId {
    let dir = lab.root.path().join(name);
    for (rel, body) in files {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, body).expect("write");
    }
    lab.store.snapshot_path(&dir).await.expect("snapshot")
}

fn request(repository: &Path, parent: &str, cid: SnapshotId) -> WorktreeRequest {
    WorktreeRequest {
        repository: repository.display().to_string(),
        parent_revision: parent.to_string(),
        branch: "change/demo".into(),
        cid,
        plan: "demo".into(),
        target: "default".into(),
        allow_in_place: false,
    }
}

fn env<'a>(lab: &'a Lab, product: Option<&'a Path>) -> ExportEnv<'a> {
    ExportEnv {
        store: &lab.store,
        publication_root: &lab.publication,
        product_root: product,
    }
}

fn staged_rows(dest: &Path) -> Vec<String> {
    git(dest, &["status", "--porcelain"]).lines().map(ToString::to_string).collect()
}

#[tokio::test]
async fn slot_created_then_matched() {
    let lab = lab();
    let (repo, parent) = origin(lab.root.path(), &[("src/lib.rs", "v1\n")]);
    let accepted =
        cid(&lab, "content", &[("src/lib.rs", "v2\n"), (".emery/specs/a.md", "s\n")]).await;

    let (dest, state) =
        export(&env(&lab, None), &request(&repo, &parent, accepted.clone())).await.expect("export");
    assert_eq!(state, WorktreeState::Created);
    assert_eq!(dest, lab.publication.join("demo/default"));
    // HEAD sits at the recorded parent on the publication branch; the
    // index and worktree carry the accepted CID, staged.
    assert_eq!(git(&dest, &["rev-parse", "HEAD"]), parent);
    assert_eq!(git(&dest, &["rev-parse", "--abbrev-ref", "HEAD"]), "change/demo");
    assert_eq!(std::fs::read_to_string(dest.join("src/lib.rs")).expect("read"), "v2\n");
    let rows = staged_rows(&dest);
    assert!(rows.iter().all(|row| !row.starts_with("??")), "everything staged: {rows:?}");
    assert!(!rows.is_empty(), "the diff is staged");

    // Re-entry with the same CID is the matched no-op.
    let (again, state) =
        export(&env(&lab, None), &request(&repo, &parent, accepted)).await.expect("re-export");
    assert_eq!(state, WorktreeState::Matched);
    assert_eq!(again, dest);
}

#[tokio::test]
async fn rematerialize_new_cid() {
    let lab = lab();
    let (repo, parent) = origin(lab.root.path(), &[("a.txt", "base\n")]);
    let first = cid(&lab, "one", &[("a.txt", "one\n")]).await;
    let second = cid(&lab, "two", &[("a.txt", "two\n"), ("b.txt", "new\n")]).await;

    export(&env(&lab, None), &request(&repo, &parent, first)).await.expect("first export");
    let (dest, state) =
        export(&env(&lab, None), &request(&repo, &parent, second)).await.expect("second export");
    assert_eq!(state, WorktreeState::Rematerialized);
    assert_eq!(std::fs::read_to_string(dest.join("a.txt")).expect("read"), "two\n");
    assert_eq!(std::fs::read_to_string(dest.join("b.txt")).expect("read"), "new\n");
}

#[tokio::test]
async fn dirty_operator_edits() {
    let lab = lab();
    let (repo, parent) = origin(lab.root.path(), &[("a.txt", "base\n")]);
    let first = cid(&lab, "one", &[("a.txt", "one\n")]).await;
    let (dest, _) =
        export(&env(&lab, None), &request(&repo, &parent, first)).await.expect("first export");

    // An unstaged edit on the materialized tree refuses.
    std::fs::write(dest.join("a.txt"), "operator\n").expect("edit");
    let second = cid(&lab, "two", &[("a.txt", "two\n")]).await;
    let err = export(&env(&lab, None), &request(&repo, &parent, second.clone()))
        .await
        .expect_err("dirty");
    assert_eq!(err, WorktreeError::Dirty);

    // An untracked file refuses the same way.
    std::fs::write(dest.join("a.txt"), "one\n").expect("restore");
    std::fs::write(dest.join("notes.txt"), "untracked\n").expect("untracked");
    let err = export(&env(&lab, None), &request(&repo, &parent, second.clone()))
        .await
        .expect_err("untracked");
    assert_eq!(err, WorktreeError::Dirty);

    // A staged-but-uncommitted operator edit refuses too — engine
    // staging is recognized by the recorded publication tree, not by
    // a blank unstaged column.
    std::fs::remove_file(dest.join("notes.txt")).expect("remove");
    std::fs::write(dest.join("a.txt"), "operator staged\n").expect("edit");
    git(&dest, &["add", "-A"]);
    let err =
        export(&env(&lab, None), &request(&repo, &parent, second)).await.expect_err("staged edit");
    assert_eq!(err, WorktreeError::Dirty);
}

#[tokio::test]
async fn no_rewind() {
    let lab = lab();
    let (repo, parent) = origin(lab.root.path(), &[("a.txt", "base\n")]);
    let first = cid(&lab, "one", &[("a.txt", "one\n")]).await;
    let (dest, _) =
        export(&env(&lab, None), &request(&repo, &parent, first)).await.expect("first export");

    // The operator commits the staged materialization — HEAD moves
    // off the parent; a later CID must leave everything alone.
    git(&dest, &["config", "user.email", "op@example.com"]);
    git(&dest, &["config", "user.name", "Op"]);
    git(&dest, &["commit", "-m", "publish"]);
    let second = cid(&lab, "two", &[("a.txt", "two\n")]).await;
    let (_, state) =
        export(&env(&lab, None), &request(&repo, &parent, second)).await.expect("no-rewind");
    assert_eq!(state, WorktreeState::Matched);
    assert_eq!(std::fs::read_to_string(dest.join("a.txt")).expect("read"), "one\n");
}

#[tokio::test]
async fn branch_diverged() {
    let lab = lab();
    let (repo, first_sha) = origin(lab.root.path(), &[("a.txt", "v1\n")]);
    let (_, second_sha) = commit(&repo, &[("a.txt", "v2\n")]);

    // Pre-provision the slot with `change/demo` at a different commit.
    let slot = lab.publication.join("demo/default");
    std::fs::create_dir_all(slot.parent().expect("parent")).expect("mkdir");
    git(
        lab.root.path(),
        &["clone", "--quiet", &repo.display().to_string(), "publication/demo/default"],
    );
    git(&slot, &["branch", "change/demo", &second_sha]);

    let accepted = cid(&lab, "content", &[("a.txt", "x\n")]).await;
    let err = export(&env(&lab, None), &request(&repo, &first_sha, accepted))
        .await
        .expect_err("diverged");
    assert_eq!(err, WorktreeError::BranchDiverged);
}

#[tokio::test]
async fn checked_out_elsewhere() {
    let lab = lab();
    let (repo, parent) = origin(lab.root.path(), &[("a.txt", "v1\n")]);
    let slot = lab.publication.join("demo/default");
    std::fs::create_dir_all(slot.parent().expect("parent")).expect("mkdir");
    git(
        lab.root.path(),
        &["clone", "--quiet", &repo.display().to_string(), "publication/demo/default"],
    );
    let elsewhere = lab.root.path().join("elsewhere").display().to_string();
    git(&slot, &["worktree", "add", &elsewhere, "-b", "change/demo", &parent]);

    let accepted = cid(&lab, "content", &[("a.txt", "x\n")]).await;
    let err =
        export(&env(&lab, None), &request(&repo, &parent, accepted)).await.expect_err("elsewhere");
    assert_eq!(err, WorktreeError::BranchCheckedOutElsewhere);
}

#[tokio::test]
async fn destination_conflict() {
    let lab = lab();
    let (repo, parent) = origin(lab.root.path(), &[("a.txt", "v1\n")]);
    let slot = lab.publication.join("demo/default");
    std::fs::create_dir_all(&slot).expect("mkdir");
    std::fs::write(slot.join("squatter.txt"), "not a worktree\n").expect("squatter");

    let accepted = cid(&lab, "content", &[("a.txt", "x\n")]).await;
    let err =
        export(&env(&lab, None), &request(&repo, &parent, accepted)).await.expect_err("conflict");
    assert_eq!(err, WorktreeError::DestinationConflict);
}

#[tokio::test]
async fn parent_unavailable() {
    let lab = lab();
    let (repo, _) = origin(lab.root.path(), &[("a.txt", "v1\n")]);
    let missing = "0123456789abcdef0123456789abcdef01234567";
    let accepted = cid(&lab, "content", &[("a.txt", "x\n")]).await;
    let err = export(&env(&lab, None), &request(&repo, missing, accepted))
        .await
        .expect_err("parent unavailable");
    assert_eq!(err, WorktreeError::ParentUnavailable);
}

#[tokio::test]
async fn clone_failed() {
    let lab = lab();
    let missing_repo = lab.root.path().join("no-such-repo");
    let accepted = cid(&lab, "content", &[("a.txt", "x\n")]).await;
    let err = export(&env(&lab, None), &request(&missing_repo, "deadbeef", accepted))
        .await
        .expect_err("clone failed");
    assert!(matches!(err, WorktreeError::CloneFailed(_)), "{err}");
}

#[tokio::test]
async fn in_place_single_member() {
    let lab = lab();
    let (repo, parent) = origin(lab.root.path(), &[("src/lib.rs", "v1\n")]);
    // The product checkout: a clean clone at the recorded parent,
    // carrying an untracked change home the rule must ignore.
    let product = lab.root.path().join("product");
    git(lab.root.path(), &["clone", "--quiet", &repo.display().to_string(), "product"]);
    std::fs::create_dir_all(product.join(".emery/change")).expect("change home");
    std::fs::write(product.join(".emery/change/plan.yaml"), "name: demo\n").expect("plan");

    let accepted = cid(&lab, "content", &[("src/lib.rs", "v2\n")]).await;
    let mut req = request(&repo, &parent, accepted);
    req.allow_in_place = true;
    let (dest, state) = export(&env(&lab, Some(&product)), &req).await.expect("in-place export");
    assert_eq!(state, WorktreeState::Created);
    assert_eq!(dest, product);
    assert_eq!(git(&dest, &["rev-parse", "--abbrev-ref", "HEAD"]), "change/demo");
    assert_eq!(std::fs::read_to_string(dest.join("src/lib.rs")).expect("read"), "v2\n");
    // The change home never enters status or the staged index.
    let rows = staged_rows(&dest);
    assert!(rows.iter().all(|row| !row.contains(".emery/change")), "{rows:?}");
}

#[tokio::test]
async fn in_place_falls_to_slot() {
    let lab = lab();
    let (repo, parent) = origin(lab.root.path(), &[("a.txt", "v1\n")]);
    let product = lab.root.path().join("product");
    git(lab.root.path(), &["clone", "--quiet", &repo.display().to_string(), "product"]);
    // A dirty product checkout fails the in-place rule; the export
    // falls to the $EMERY_HOME publication slot instead.
    std::fs::write(product.join("a.txt"), "dirty\n").expect("dirty");

    let accepted = cid(&lab, "content", &[("a.txt", "x\n")]).await;
    let mut req = request(&repo, &parent, accepted);
    req.allow_in_place = true;
    let (dest, state) = export(&env(&lab, Some(&product)), &req).await.expect("slot export");
    assert_eq!(state, WorktreeState::Created);
    assert_eq!(dest, lab.publication.join("demo/default"));
    assert_eq!(std::fs::read_to_string(product.join("a.txt")).expect("read"), "dirty\n");
}
