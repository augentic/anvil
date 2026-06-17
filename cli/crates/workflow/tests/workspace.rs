//! Integration tests for `specify_workflow::registry::workspace`.
//!
//! Deliberately narrow: the binary-level `tests/workspace.rs` covers the
//! `workspace {sync,push,prepare}` wire surface (selector preflight,
//! journaling, symlink materialisation, the self-slot and foreign-entry
//! mirror pins). This file keeps only what that layer does not reach:
//! slot-problem classification and sync refusal edges, mirror corner
//! cases not pinned in-module or at the binary, branch-preparation
//! dirtiness and fast-forward semantics, push outcome classification,
//! and `topology.lock` regeneration.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use specify_workflow::registry::branch::{
    LocalAction, RemoteAction, Request as BranchRequest, prepare,
};
use specify_workflow::registry::workspace::{
    PushOutcome, SlotKind, SlotProblemReason, push_projects, slot_problem,
    sync_projects as workspace_sync_projects,
};
use specify_workflow::registry::{Registry, RegistryProject};
use tempfile::TempDir;

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).expect("symlink");
}

#[cfg(windows)]
fn symlink_dir(target: &Path, link: &Path) {
    std::os::windows::fs::symlink_dir(target, link).expect("symlink");
}

#[path = "../../../tests/common/fs_git.rs"]
mod fs_git;
use fs_git::run_git;

const CACHE_ENV: &str = "SPECIFY_PROJECT_CACHE";
const MIRROR_ENV: &str = "SPECIFY_MIRROR_CACHE";

/// Restores the previous `SPECIFY_PROJECT_CACHE` and `SPECIFY_MIRROR_CACHE`
/// values on drop.
struct CacheGuard {
    cache: Option<std::ffi::OsString>,
    mirror: Option<std::ffi::OsString>,
}

impl Drop for CacheGuard {
    #[expect(unsafe_code, reason = "restore the cache-root env vars pinned for the test")]
    fn drop(&mut self) {
        // SAFETY: nextest runs each test in its own process, so no other
        // thread observes the env mutation for the guard's lifetime.
        unsafe {
            match self.cache.take() {
                Some(prev) => std::env::set_var(CACHE_ENV, prev),
                None => std::env::remove_var(CACHE_ENV),
            }
            match self.mirror.take() {
                Some(prev) => std::env::set_var(MIRROR_ENV, prev),
                None => std::env::remove_var(MIRROR_ENV),
            }
        }
    }
}

/// Pin the out-of-tree project cache and Git mirror roots inside `dir` so
/// adapter / codex cache writes and remote-peer mirror clones are hermetic
/// and auto-cleaned with the tempdir.
#[expect(unsafe_code, reason = "pin the cache-root env vars into the test tempdir")]
fn scoped_cache(dir: &Path) -> CacheGuard {
    let cache = std::env::var_os(CACHE_ENV);
    let mirror = std::env::var_os(MIRROR_ENV);
    // SAFETY: see `CacheGuard::drop` — single-process test isolation.
    unsafe { std::env::set_var(CACHE_ENV, dir.join("project-cache")) };
    // SAFETY: see `CacheGuard::drop` — single-process test isolation.
    unsafe { std::env::set_var(MIRROR_ENV, dir.join("mirror-cache")) };
    CacheGuard { cache, mirror }
}

/// Out-of-tree cache directory for `project_dir` under the pinned root.
fn expected_cache_dir(project_dir: &Path) -> PathBuf {
    specify_schema::cache::project_cache_dir(project_dir)
}

fn registry_with_projects(names: &[&str]) -> Registry {
    Registry {
        version: 1,
        projects: names
            .iter()
            .map(|name| RegistryProject {
                name: (*name).to_string(),
                url: format!("./{name}"),
                adapter: Some("omnia@1.0.0".to_string()),
                description: Some(format!("{name} service")),
                contracts: None,
                greenfield_seed: None,
            })
            .collect(),
    }
}

fn branch_request(change_name: &str) -> BranchRequest {
    BranchRequest {
        change_name: change_name.to_string(),
        source_paths: Vec::new(),
        output_paths: Vec::new(),
    }
}

fn remote_project(url: String) -> RegistryProject {
    RegistryProject {
        name: "alpha".to_string(),
        url,
        adapter: Some("omnia@1.0.0".to_string()),
        description: Some("alpha service".to_string()),
        contracts: None,
        greenfield_seed: None,
    }
}

fn seed_bare_remote(tmp: &TempDir) -> (PathBuf, String) {
    let source = tmp.path().join("source");
    fs::create_dir_all(&source).unwrap();
    run_git(&source, &["init", "-b", "main"]);
    fs::write(source.join("README.md"), "seed\n").unwrap();
    run_git(&source, &["add", "README.md"]);
    run_git(&source, &["commit", "--no-gpg-sign", "-m", "seed"]);

    let remote = tmp.path().join("alpha.git");
    run_git(tmp.path(), &["clone", "--bare", source.to_str().unwrap(), remote.to_str().unwrap()]);
    (remote.clone(), format!("file://{}", remote.display()))
}

fn clone_workspace_slot(project_dir: &Path, remote_url: &str) -> PathBuf {
    let slot = project_dir.join("workspace/alpha");
    fs::create_dir_all(slot.parent().unwrap()).unwrap();
    run_git(project_dir, &["clone", remote_url, slot.to_str().unwrap()]);
    slot
}

fn git_output(root: &Path, args: &[&str]) -> String {
    run_git(root, args).trim().to_string()
}

fn current_branch(root: &Path) -> String {
    git_output(root, &["branch", "--show-current"])
}

fn prepare_change_branch(slot: &Path, change_name: &str) -> String {
    let branch = format!("specify/{change_name}");
    run_git(slot, &["checkout", "-b", &branch]);
    fs::write(slot.join("change.txt"), format!("{change_name}\n")).unwrap();
    run_git(slot, &["add", "change.txt"]);
    run_git(slot, &["commit", "--no-gpg-sign", "-m", "change work"]);
    git_output(slot, &["rev-parse", "HEAD"])
}

fn remote_branch_head(remote_url: &str, branch: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["ls-remote", "--heads", remote_url, &format!("refs/heads/{branch}")])
        .output()
        .expect("spawn git ls-remote");
    assert!(
        output.status.success(),
        "git ls-remote failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.split_whitespace().next())
        .map(ToString::to_string)
}

// ---------- sync_projects ---------------------------------------

#[test]
fn c01_selector_preserves_order() {
    // `select` returns projects in registry order regardless of the
    // order the selectors were passed in — the binary only pins the
    // unknown-selector preflight, not the ordering contract.
    let registry = registry_with_projects(&["billing", "orders", "inventory"]);
    let selected =
        registry.select(&["orders".to_string(), "billing".to_string()]).expect("selectors resolve");
    let names: Vec<&str> = selected.iter().map(|project| project.name.as_str()).collect();
    assert_eq!(names, ["billing", "orders"]);
}

#[test]
fn c02_local_slot_refuses_non_symlink() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path();
    fs::create_dir_all(project_dir.join("peer")).unwrap();
    fs::create_dir_all(project_dir.join("workspace/peer")).unwrap();
    fs::write(project_dir.join("workspace/peer/sentinel.txt"), "keep").unwrap();

    let registry = registry_with_projects(&["peer"]);
    let selected = registry.select(&["peer".to_string()]).unwrap();
    let err = workspace_sync_projects(project_dir, &selected)
        .expect_err("mismatched local slot should fail");
    let msg = err.to_string();

    assert!(msg.contains("not a symlink"), "msg: {msg}");
    assert_eq!(
        fs::read_to_string(project_dir.join("workspace/peer/sentinel.txt")).unwrap(),
        "keep",
        "mismatched slot must not be overwritten"
    );
}

#[test]
fn c02_remote_slot_refuses_existing_symlink() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path();
    let target = project_dir.join("not-remote");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(project_dir.join("workspace")).unwrap();
    symlink_dir(&target, &project_dir.join("workspace/remote"));

    let project = RegistryProject {
        name: "remote".to_string(),
        url: "https://example.invalid/org/remote.git".to_string(),
        adapter: Some("https://example.invalid/adapter".to_string()),
        description: Some("remote service".to_string()),
        contracts: None,
        greenfield_seed: None,
    };
    let err = workspace_sync_projects(project_dir, &[&project])
        .expect_err("remote-backed symlink slot should fail");
    let msg = err.to_string();

    assert!(msg.contains("is a symlink"), "msg: {msg}");
    assert!(
        fs::symlink_metadata(project_dir.join("workspace/remote"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn c10_slot_problem_wrong_symlink() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path();
    let peer = project_dir.join("peer");
    let other = project_dir.join("other");
    fs::create_dir_all(&peer).unwrap();
    fs::create_dir_all(&other).unwrap();
    fs::create_dir_all(project_dir.join("workspace")).unwrap();
    symlink_dir(&other, &project_dir.join("workspace/peer"));

    let registry = registry_with_projects(&["peer"]);
    let project = &registry.projects[0];
    let problem = slot_problem(project_dir, project).expect("wrong symlink problem");
    assert_eq!(problem.reason, SlotProblemReason::SymlinkTargetMismatch);
    assert_eq!(problem.observed_kind, Some(SlotKind::Symlink));

    let selected = registry.select(&["peer".to_string()]).unwrap();
    let err = workspace_sync_projects(project_dir, &selected)
        .expect_err("sync should refuse same wrong symlink");
    let msg = err.to_string();
    assert!(msg.contains(problem.message()), "msg: {msg}\nproblem: {}", problem.message());
}

#[test]
fn c10_slot_problem_wrong_origin() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path();
    let slot = project_dir.join("workspace/remote");
    fs::create_dir_all(&slot).unwrap();
    run_git(&slot, &["init"]);
    run_git(&slot, &["remote", "add", "origin", "https://example.invalid/old.git"]);

    let project = RegistryProject {
        name: "remote".to_string(),
        url: "https://example.invalid/new.git".to_string(),
        adapter: Some("https://example.invalid/adapter".to_string()),
        description: Some("remote service".to_string()),
        contracts: None,
        greenfield_seed: None,
    };
    let problem = slot_problem(project_dir, &project).expect("wrong origin problem");
    assert_eq!(problem.reason, SlotProblemReason::RemoteOriginMismatch);
    assert_eq!(problem.observed_url.as_deref(), Some("https://example.invalid/old.git"));

    let err = workspace_sync_projects(project_dir, &[&project])
        .expect_err("sync should refuse same wrong origin");
    let msg = err.to_string();
    assert!(msg.contains(problem.message()), "msg: {msg}\nproblem: {}", problem.message());
}

#[test]
fn c02_sync_refuses_escaping_name() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path();
    fs::create_dir_all(project_dir.join("peer")).unwrap();
    let project = RegistryProject {
        name: "../escape".to_string(),
        url: "./peer".to_string(),
        adapter: Some("omnia@1.0.0".to_string()),
        description: Some("bad selector".to_string()),
        contracts: None,
        greenfield_seed: None,
    };

    let err = workspace_sync_projects(project_dir, &[&project])
        .expect_err("traversal-like project name should fail");
    let msg = err.to_string();

    assert!(msg.contains("single path component"), "msg: {msg}");
    assert!(
        !project_dir.join(".specify/escape").exists(),
        "sync must not materialise a path outside workspace/<project>/"
    );
}

#[test]
fn c02_sync_refuses_symlinked_base() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path();
    fs::create_dir_all(project_dir.join("peer")).unwrap();
    fs::create_dir_all(project_dir.join(".specify")).unwrap();
    let outside = project_dir.join("outside-workspace");
    fs::create_dir_all(&outside).unwrap();
    symlink_dir(&outside, &project_dir.join("workspace"));

    let registry = registry_with_projects(&["peer"]);
    let selected = registry.select(&["peer".to_string()]).unwrap();
    let err = workspace_sync_projects(project_dir, &selected)
        .expect_err("workspace base symlink should fail");
    let msg = err.to_string();

    assert!(msg.contains("workspace/ is a symlink"), "msg: {msg}");
    assert!(
        !outside.join("peer").exists(),
        "sync must not materialise slots through a symlinked workspace base"
    );
}

#[test]
fn c02_sync_preserves_gitignore_once() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path();
    fs::create_dir_all(project_dir.join("peer")).unwrap();
    fs::write(project_dir.join(".gitignore"), "target/\nworkspace/\n").unwrap();
    let registry = registry_with_projects(&["peer"]);
    let selected = registry.select(&[]).unwrap();

    workspace_sync_projects(project_dir, &selected).expect("sync ok");
    workspace_sync_projects(project_dir, &selected).expect("sync remains idempotent");

    let gitignore = fs::read_to_string(project_dir.join(".gitignore")).unwrap();
    assert_eq!(gitignore.lines().filter(|line| line.trim() == "workspace/").count(), 1);
    assert_eq!(gitignore.lines().filter(|line| line.trim() == ".specify/scratch/").count(), 1);
}

// ---------- adapter mirror (slot adapter provisioning) ---------

/// Stage an adapter dir (`adapter.yaml` + optional extra files) under
/// `root/<rel>/`.
fn stage_adapter_at(root: &Path, rel: &str, body: &str, extra: &[(&str, &str)]) {
    let dir = root.join(rel);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("adapter.yaml"), body).unwrap();
    for (name, contents) in extra {
        let path = dir.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
}

/// A workspace with one local symlink peer (`./peer`) that is itself a
/// Specify project (carries `.specify/`). Returns the peer dir.
fn workspace_with_specify_peer(project_dir: &Path) -> PathBuf {
    let peer = project_dir.join("peer");
    fs::create_dir_all(peer.join(".specify")).unwrap();
    fs::write(
        project_dir.join("registry.yaml"),
        "version: 1\nprojects:\n  - name: peer\n    url: ./peer\n    adapter: omnia@1.0.0\n",
    )
    .unwrap();
    peer
}

fn sync_all(project_dir: &Path) {
    let registry = Registry::load(project_dir).unwrap().expect("registry present");
    workspace_sync_projects(project_dir, &registry.select(&[]).unwrap()).expect("sync ok");
}

#[test]
fn mirror_covers_target_axis_and_sidecars() {
    // The binary mirror pins only exercise the source axis with a bare
    // `adapter.yaml`; target-axis coverage and sidecar files riding the
    // mirror are pinned here.
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path();
    let _cache = scoped_cache(project_dir);
    let peer = workspace_with_specify_peer(project_dir);
    stage_adapter_at(
        project_dir,
        "adapters/targets/vectis",
        "name: vectis\n",
        &[("tools.yaml", "tools: []\n")],
    );

    sync_all(project_dir);

    let mirrored = expected_cache_dir(&peer).join("manifests/targets/vectis");
    assert!(mirrored.join("adapter.yaml").is_file(), "target axis must be mirrored");
    assert_eq!(
        fs::read_to_string(mirrored.join("tools.yaml")).expect("mirrored tools.yaml"),
        "tools: []\n",
        "tool sidecars must ride the mirror"
    );
}

#[test]
fn mirror_skips_slot_vendored_name() {
    // The loader probes the cache before the vendored tree, so the
    // mirror must skip a name the slot vendors itself — otherwise the
    // mirrored twin would shadow the slot's copy.
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path();
    let _cache = scoped_cache(project_dir);
    let peer = workspace_with_specify_peer(project_dir);
    stage_adapter_at(project_dir, "adapters/sources/documentation", "workspace copy\n", &[]);
    stage_adapter_at(&peer, "adapters/sources/documentation", "slot copy\n", &[]);

    sync_all(project_dir);

    assert!(
        !expected_cache_dir(&peer).join("manifests/sources/documentation").exists(),
        "a slot-vendored name must not be shadowed by a mirrored cache copy"
    );
    assert_eq!(
        fs::read_to_string(peer.join("adapters/sources/documentation/adapter.yaml")).unwrap(),
        "slot copy\n",
        "the slot's vendored copy must be untouched"
    );
}

#[test]
fn mirror_skips_non_specify_peer() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path();
    let peer = project_dir.join("peer");
    fs::create_dir_all(&peer).unwrap();
    fs::write(
        project_dir.join("registry.yaml"),
        "version: 1\nprojects:\n  - name: peer\n    url: ./peer\n    adapter: omnia@1.0.0\n",
    )
    .unwrap();
    stage_adapter_at(project_dir, "adapters/sources/documentation", "name: documentation\n", &[]);

    sync_all(project_dir);

    assert!(
        !peer.join(".specify").exists(),
        "the mirror must not manufacture `.specify/` in a non-Specify peer"
    );
}

// ---------- persistent bare-mirror + per-change worktree -----------------------------------------

#[test]
fn sync_remote_worktree_reuses_mirror() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("platform");
    fs::create_dir_all(&project_dir).unwrap();
    let _cache = scoped_cache(&project_dir);
    let (_remote, remote_url) = seed_bare_remote(&tmp);
    let project = remote_project(remote_url.clone());

    // First sync clones the persistent mirror once, then checks out a worktree.
    workspace_sync_projects(&project_dir, &[&project]).expect("first sync");

    let slot = project_dir.join("workspace/alpha");
    assert!(
        slot.join(".git").is_file(),
        "a remote slot is a git worktree, so `.git` is a gitlink file, not a directory"
    );
    assert!(slot.join("README.md").is_file(), "worktree checks out the remote's default branch");

    let mirror = specify_schema::cache::mirror_dir(&remote_url);
    assert!(mirror.join("HEAD").is_file(), "the persistent bare mirror lives out-of-tree");

    // A re-clone would wipe this sentinel; a fetch keeps it.
    let sentinel = mirror.join("sentinel");
    fs::write(&sentinel, "keep").unwrap();

    // Simulate a fresh checkout: the gitignored slot vanishes, the mirror persists.
    fs::remove_dir_all(&slot).unwrap();
    workspace_sync_projects(&project_dir, &[&project]).expect("second sync reuses the mirror");

    assert!(slot.join(".git").is_file(), "the worktree re-materialises from the persistent mirror");
    assert!(sentinel.is_file(), "the mirror was reused via fetch, not re-cloned");
}

#[test]
fn prepare_on_worktree_creates_branch() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("platform");
    fs::create_dir_all(&project_dir).unwrap();
    let _cache = scoped_cache(&project_dir);
    let (_remote, remote_url) = seed_bare_remote(&tmp);
    let project = remote_project(remote_url);

    workspace_sync_projects(&project_dir, &[&project]).expect("sync materialises a worktree");

    let prepared = prepare(&project_dir, &project, &branch_request("demo-change"))
        .expect("branch preparation succeeds against a mirror-backed worktree");

    assert_eq!(prepared.local_branch, LocalAction::Created);
    let slot = project_dir.join("workspace/alpha");
    assert_eq!(current_branch(&slot), "specify/demo-change");
}

// ---------- branch preparation (workspace orchestration contract C04) ----------------------------

#[test]
fn c04_prepare_reuses_resume_dirty_ok() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("workspace");
    fs::create_dir_all(&project_dir).unwrap();
    let (_remote, remote_url) = seed_bare_remote(&tmp);
    let slot = clone_workspace_slot(&project_dir, &remote_url);
    let project = remote_project(remote_url);
    prepare(&project_dir, &project, &branch_request("demo-change")).expect("initial prepare");

    let tracked = slot.join(".specify/slices/demo-change/notes.md");
    fs::create_dir_all(tracked.parent().unwrap()).unwrap();
    fs::write(&tracked, "first\n").unwrap();
    run_git(&slot, &["add", ".specify/slices/demo-change/notes.md"]);
    run_git(&slot, &["commit", "--no-gpg-sign", "-m", "slice progress"]);
    fs::write(&tracked, "first\nsecond\n").unwrap();
    fs::write(slot.join("scratch.tmp"), "untracked\n").unwrap();

    let prepared = prepare(&project_dir, &project, &branch_request("demo-change"))
        .expect("resume prepare accepts active slice dirtiness");

    assert_eq!(prepared.local_branch, LocalAction::Reused);
    assert_eq!(current_branch(&slot), "specify/demo-change");
    assert_eq!(
        prepared.dirty.tracked_allowed,
        vec![".specify/slices/demo-change/notes.md".to_string()]
    );
    assert_eq!(prepared.dirty.tracked_blocked, Vec::<String>::new());
    assert_eq!(prepared.dirty.untracked, vec!["scratch.tmp".to_string()]);
}

#[test]
fn c04_prepare_ff_remote_ahead() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("workspace");
    fs::create_dir_all(&project_dir).unwrap();
    let (_remote, remote_url) = seed_bare_remote(&tmp);
    let slot = clone_workspace_slot(&project_dir, &remote_url);
    let project = remote_project(remote_url.clone());
    prepare(&project_dir, &project, &branch_request("demo-change")).expect("initial prepare");

    let peer = tmp.path().join("peer");
    run_git(tmp.path(), &["clone", &remote_url, peer.to_str().unwrap()]);
    run_git(&peer, &["checkout", "-b", "specify/demo-change"]);
    fs::write(peer.join("README.md"), "seed\nremote\n").unwrap();
    run_git(&peer, &["add", "README.md"]);
    run_git(&peer, &["commit", "--no-gpg-sign", "-m", "remote progress"]);
    run_git(&peer, &["push", "origin", "specify/demo-change"]);
    let remote_tip = git_output(&peer, &["rev-parse", "HEAD"]);

    let prepared = prepare(&project_dir, &project, &branch_request("demo-change"))
        .expect("remote ahead fast-forwards");

    assert_eq!(prepared.local_branch, LocalAction::Reused);
    assert_eq!(prepared.remote_branch, RemoteAction::FastForwarded);
    assert_eq!(git_output(&slot, &["rev-parse", "HEAD"]), remote_tip);
}

#[test]
fn c04_prepare_blocks_unrelated_dirty() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("workspace");
    fs::create_dir_all(&project_dir).unwrap();
    let (_remote, remote_url) = seed_bare_remote(&tmp);
    let slot = clone_workspace_slot(&project_dir, &remote_url);
    let project = remote_project(remote_url);
    fs::write(slot.join("README.md"), "unrelated\n").unwrap();

    let err = prepare(&project_dir, &project, &branch_request("demo-change"))
        .expect_err("unrelated tracked work must block");

    assert_eq!(err.key, "dirty-unrelated-tracked");
    assert_eq!(err.paths, vec!["README.md".to_string()]);
    assert_eq!(current_branch(&slot), "main", "branch must not be changed on refusal");
}

#[test]
fn c04_prepare_reports_missing_origin() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("workspace");
    fs::create_dir_all(&project_dir).unwrap();
    let slot = project_dir.join("workspace/alpha");
    fs::create_dir_all(&slot).unwrap();
    run_git(&slot, &["init", "-b", "main"]);
    fs::write(slot.join("README.md"), "seed\n").unwrap();
    run_git(&slot, &["add", "README.md"]);
    run_git(&slot, &["commit", "--no-gpg-sign", "-m", "seed"]);
    let project = remote_project("https://example.invalid/org/alpha.git".to_string());

    let err = prepare(&project_dir, &project, &branch_request("demo-change"))
        .expect_err("missing origin must fail");

    assert_eq!(err.key, "missing-origin");
    assert_eq!(current_branch(&slot), "main");
}

// ---------- workspace push (workspace orchestration contract C07) -------------------------------

#[test]
fn c07_push_outcomes() {
    // One clean change branch, three pushes: a dry run classifies as
    // Pushed without touching the remote, the first real push lands the
    // local head, and a repeat push reports UpToDate. The binary push
    // test only covers the origin-less `local-only` outcome.
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("workspace");
    fs::create_dir_all(&project_dir).unwrap();
    let (_remote, remote_url) = seed_bare_remote(&tmp);
    let slot = clone_workspace_slot(&project_dir, &remote_url);
    let local_head = prepare_change_branch(&slot, "demo-change");
    let project = remote_project(remote_url.clone());

    let results =
        push_projects(&project_dir, "demo-change", &[&project], true).expect("dry-run succeeds");
    assert_eq!(results[0].status, PushOutcome::Pushed);
    assert!(
        remote_branch_head(&remote_url, "specify/demo-change").is_none(),
        "dry-run must not push"
    );

    let results =
        push_projects(&project_dir, "demo-change", &[&project], false).expect("push succeeds");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, PushOutcome::Pushed);
    assert_eq!(results[0].branch.as_deref(), Some("specify/demo-change"));
    assert_eq!(
        remote_branch_head(&remote_url, "specify/demo-change").as_deref(),
        Some(local_head.as_str())
    );
    assert_eq!(current_branch(&slot), "specify/demo-change", "push must not rebrand HEAD");

    let results =
        push_projects(&project_dir, "demo-change", &[&project], false).expect("second push");
    assert_eq!(results[0].status, PushOutcome::UpToDate);
    assert_eq!(
        remote_branch_head(&remote_url, "specify/demo-change").as_deref(),
        Some(local_head.as_str())
    );
}

#[test]
fn c07_push_dirty_without_push() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("workspace");
    fs::create_dir_all(&project_dir).unwrap();
    let (_remote, remote_url) = seed_bare_remote(&tmp);
    let slot = clone_workspace_slot(&project_dir, &remote_url);
    prepare_change_branch(&slot, "demo-change");
    fs::write(slot.join("scratch.txt"), "dirty\n").unwrap();
    let project = remote_project(remote_url.clone());

    let results = push_projects(&project_dir, "demo-change", &[&project], false)
        .expect("best-effort push returns results");

    assert_eq!(results[0].status, PushOutcome::Failed);
    assert!(
        results[0].error.as_deref().is_some_and(|err| err.contains("dirty")),
        "dirty failure should be actionable: {:?}",
        results[0].error
    );
    assert!(
        remote_branch_head(&remote_url, "specify/demo-change").is_none(),
        "dirty checkouts must not be pushed"
    );
}

#[test]
fn c07_push_wrong_branch_no_checkout() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("workspace");
    fs::create_dir_all(&project_dir).unwrap();
    let (_remote, remote_url) = seed_bare_remote(&tmp);
    let slot = clone_workspace_slot(&project_dir, &remote_url);
    run_git(&slot, &["checkout", "-b", "feature/work"]);
    let project = remote_project(remote_url.clone());

    let results = push_projects(&project_dir, "demo-change", &[&project], false)
        .expect("best-effort push returns results");

    assert_eq!(results[0].status, PushOutcome::NoBranch);
    assert_eq!(current_branch(&slot), "feature/work", "push must not checkout another branch");
    assert!(remote_branch_head(&remote_url, "specify/demo-change").is_none());
}

// ---------- topology.lock regeneration ------------------------

/// Stage a materialised slot with a resolvable omnia adapter and the
/// given `project.yaml` body under `workspace/<name>/`.
fn stage_topology_slot(project_dir: &Path, name: &str, project_yaml: &str) {
    let slot = project_dir.join("workspace").join(name);
    let slot_specify = slot.join(".specify");
    fs::create_dir_all(&slot_specify).unwrap();
    fs::write(slot_specify.join("project.yaml"), project_yaml).unwrap();
    // The slot resolves its target adapter from the out-of-tree manifest
    // cache keyed by the slot path (the env is pinned by the caller).
    let omnia_manifest = expected_cache_dir(&slot).join("manifests/targets/omnia");
    fs::create_dir_all(&omnia_manifest).unwrap();
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/plugins/adapters/targets/omnia/adapter.yaml");
    fs::copy(fixture, omnia_manifest.join("adapter.yaml")).unwrap();
}

#[test]
fn topology_lock_projects_baseline() {
    use specify_workflow::registry::topology::TopologyLock;
    use specify_workflow::registry::workspace::regenerate_topology_lock;

    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path();
    let _cache = scoped_cache(project_dir);
    // A stale `capabilities:` key is silently ignored; routing
    // identity is derived from the slot's baseline, not re-authored.
    stage_topology_slot(
        project_dir,
        "alpha",
        "name: alpha\nadapter: omnia@1.0.0\ndescription: Alpha core\ncapabilities:\n  - auth\n",
    );
    let alpha_specify = project_dir.join("workspace/alpha/.specify");
    let session_dir = alpha_specify.join("specs/session");
    fs::create_dir_all(&session_dir).unwrap();
    fs::write(
        session_dir.join("spec.md"),
        "### Requirement: Issue session token\n\nID: REQ-001\n\nBody.\n\n\
         ### Requirement: Revoke session\n\nID: REQ-002\n\nBody.\n",
    )
    .unwrap();
    fs::write(
        alpha_specify.join("journal.jsonl"),
        "{\"timestamp\":\"2026-01-01T00:00:00Z\",\"event\":\"slice.archive.created\",\"payload\":{\"slice-name\":\"session\",\"touched-specs\":[\"session\"],\"outcome-summary\":\"session: 2 added\"}}\n",
    )
    .unwrap();

    // A registry member whose slot has not been materialised yet is
    // skipped, not an error.
    let registry = Registry {
        version: 1,
        projects: vec![
            RegistryProject {
                name: "alpha".to_string(),
                url: "git@github.com:org/alpha.git".to_string(),
                adapter: None,
                description: None,
                contracts: None,
                greenfield_seed: None,
            },
            RegistryProject {
                name: "beta".to_string(),
                url: "git@github.com:org/beta.git".to_string(),
                adapter: None,
                description: None,
                contracts: None,
                greenfield_seed: None,
            },
        ],
    };

    regenerate_topology_lock(project_dir, &registry).expect("regenerate");

    let lock = TopologyLock::load(&project_dir.join(".specify/topology.lock"))
        .expect("load")
        .expect("present");
    assert_eq!(lock.projects.len(), 1, "unmaterialised beta is skipped");
    let alpha = &lock.projects[0];
    assert_eq!(alpha.name, "alpha");
    assert_eq!(alpha.target, "omnia@1.0.0");
    assert_eq!(alpha.description.as_deref(), Some("Alpha core"));
    assert_eq!(alpha.surface.len(), 1);
    assert_eq!(alpha.surface[0].domain, "session");
    assert_eq!(
        alpha.surface[0].requirements,
        vec!["Issue session token".to_string(), "Revoke session".to_string()]
    );
    assert_eq!(alpha.recent, vec!["session: 2 added".to_string()]);
}

#[test]
fn topology_lock_projects_decisions() {
    use specify_workflow::registry::topology::TopologyLock;
    use specify_workflow::registry::workspace::regenerate_topology_lock;

    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path();
    let _cache = scoped_cache(project_dir);
    stage_topology_slot(
        project_dir,
        "alpha",
        "name: alpha\nadapter: omnia@1.0.0\ndescription: Alpha core\n",
    );
    let decisions_dir = project_dir.join("workspace/alpha/.specify/decisions");
    fs::create_dir_all(&decisions_dir).unwrap();
    let decision = |id: &str, slug: &str, status: &str, title: &str| {
        format!(
            "---\nid: {id}\nslug: {slug}\nstatus: {status}\nslice: s\ndate: 2026-06-02\n---\n\
             # {title}\n\n## Context\nc\n\n## Decision\nd\n\n## Consequences\ne\n"
        )
    };
    fs::write(
        decisions_dir.join("DEC-0001-use-postgres.md"),
        decision("DEC-0001", "use-postgres", "accepted", "Use PostgreSQL"),
    )
    .unwrap();
    fs::write(
        decisions_dir.join("DEC-0002-drop-redis.md"),
        decision("DEC-0002", "drop-redis", "rejected", "Drop Redis"),
    )
    .unwrap();

    let registry = Registry {
        version: 1,
        projects: vec![RegistryProject {
            name: "alpha".to_string(),
            url: "git@github.com:org/alpha.git".to_string(),
            adapter: None,
            description: None,
            contracts: None,
            greenfield_seed: None,
        }],
    };

    regenerate_topology_lock(project_dir, &registry).expect("regenerate");

    let lock = TopologyLock::load(&project_dir.join(".specify/topology.lock"))
        .expect("load")
        .expect("present");
    let alpha = &lock.projects[0];
    // Only the accepted record is projected, title-only.
    assert_eq!(alpha.decisions.len(), 1);
    assert_eq!(alpha.decisions[0].id, "DEC-0001");
    assert_eq!(alpha.decisions[0].title, "Use PostgreSQL");
    assert!(alpha.decisions_more.is_none());

    // The accepted decision round-trips onto the wire under `decisions:`.
    let yaml = fs::read_to_string(project_dir.join(".specify/topology.lock")).unwrap();
    assert!(yaml.contains("DEC-0001"), "{yaml}");
}
