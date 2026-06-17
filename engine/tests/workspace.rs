//! Integration tests for `specify workspace *` (workspace orchestration contract).
//!
//! Covers `workspace sync`, `workspace push`, and the hidden
//! `workspace prepare` executor helper. Selector
//! preflight, slot enrichment, and branch-preparation diagnostics are
//! pinned to the wire shape skill authors rely on.

use std::fs;

use tempfile::tempdir;

mod common;
use common::{
    Project, expected_cache_dir, init_workspace, omnia_schema_dir, parse_stdout, run_git,
    specify_cmd,
};

#[test]
fn workspace_help_lists_active_subcommands() {
    // `workspace --help` must exit 0; the verb inventory is asserted
    // via the contract dump rather than exact clap wording.
    specify_cmd().args(["workspace", "--help"]).assert().success();
    let verbs = common::contract_dump_verbs(&["workspace"]);
    for verb in ["sync", "push"] {
        assert!(verbs.iter().any(|v| v == verb), "workspace must declare `{verb}`: {verbs:?}");
    }
}

#[test]
fn c01_sync_unknown_selector_preflight() {
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");
    fs::write(
        tmp.path().join("registry.yaml"),
        "version: 1\n\
         projects:\n\
         \x20\x20- name: alpha\n\
         \x20\x20\x20\x20url: git@github.com:org/alpha.git\n\
         \x20\x20\x20\x20adapter: omnia@1.0.0\n",
    )
    .unwrap();
    let gitignore_before = fs::read_to_string(tmp.path().join(".gitignore")).ok();

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "workspace", "sync", "ghost"])
        .assert()
        .failure();
    let value: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stderr).expect("json");
    assert_eq!(value["error"], "registry-project-selector-unknown");
    let msg = value["message"].as_str().expect("message");
    assert!(msg.contains("unknown project"), "msg: {msg}");
    assert!(msg.contains("ghost"), "msg: {msg}");
    assert!(
        !tmp.path().join("workspace/ghost").exists(),
        "unknown selector must fail before materialising the requested slot"
    );
    assert!(
        !tmp.path().join("workspace/alpha").exists(),
        "unknown selector must fail before syncing any registry project"
    );
    assert_eq!(
        fs::read_to_string(tmp.path().join(".gitignore")).ok(),
        gitignore_before,
        "unknown selector must fail before sync mutates .gitignore again"
    );
}

#[test]
fn c01_sync_skips_unselected_slots() {
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");
    for name in ["billing", "orders", "inventory"] {
        fs::create_dir_all(tmp.path().join(name)).unwrap();
    }
    fs::write(
        tmp.path().join("registry.yaml"),
        "version: 1\n\
         projects:\n\
         \x20\x20- name: billing\n\
         \x20\x20\x20\x20url: ./billing\n\
         \x20\x20\x20\x20adapter: omnia@1.0.0\n\
         \x20\x20\x20\x20description: billing service\n\
         \x20\x20- name: orders\n\
         \x20\x20\x20\x20url: ./orders\n\
         \x20\x20\x20\x20adapter: omnia@1.0.0\n\
         \x20\x20\x20\x20description: orders service\n\
         \x20\x20- name: inventory\n\
         \x20\x20\x20\x20url: ./inventory\n\
         \x20\x20\x20\x20adapter: omnia@1.0.0\n\
         \x20\x20\x20\x20description: inventory service\n",
    )
    .unwrap();

    specify_cmd()
        .current_dir(tmp.path())
        .args(["workspace", "sync", "orders", "billing"])
        .assert()
        .success();
    assert!(tmp.path().join("workspace/billing").exists());
    assert!(tmp.path().join("workspace/orders").exists());
    assert!(
        !tmp.path().join("workspace/inventory").exists(),
        "selected sync must not materialise unselected slots"
    );
}

#[test]
fn c01_sync_journals_completed_event() {
    // workflow §Observability: one `workspace.sync.completed` per
    // successful sync, carrying the materialised project names.
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");
    for name in ["billing", "orders"] {
        fs::create_dir_all(tmp.path().join(name)).unwrap();
    }
    fs::write(
        tmp.path().join("registry.yaml"),
        "version: 1\n\
         projects:\n\
         \x20\x20- name: billing\n\
         \x20\x20\x20\x20url: ./billing\n\
         \x20\x20\x20\x20adapter: omnia@1.0.0\n\
         \x20\x20- name: orders\n\
         \x20\x20\x20\x20url: ./orders\n\
         \x20\x20\x20\x20adapter: omnia@1.0.0\n",
    )
    .unwrap();

    specify_cmd().current_dir(tmp.path()).args(["workspace", "sync"]).assert().success();

    let raw = fs::read_to_string(tmp.path().join(".specify/journal.jsonl"))
        .expect("sync must journal workspace.sync.completed");
    let lines: Vec<&str> = raw.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1, "exactly one event per sync, got:\n{raw}");
    assert!(lines[0].contains(r#""event":"workspace.sync.completed""#), "got:\n{}", lines[0]);
    assert!(lines[0].contains(r#""projects":["billing","orders"]"#), "got:\n{}", lines[0]);
}

#[test]
fn sync_no_registry_no_journal() {
    let project = Project::init();
    specify_cmd().current_dir(project.root()).args(["workspace", "sync"]).assert().success();
    assert!(
        !project.root().join(".specify/journal.jsonl").exists(),
        "the registry-less no-op sync must not journal workspace.sync.completed"
    );
}

#[test]
fn push_journals_completed_event() {
    // workflow §Observability: one `workspace.push.completed` per
    // successful non-dry-run push (a `local-only` outcome is not a
    // failure); dry runs emit nothing.
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");
    fs::write(tmp.path().join("plan.yaml"), "name: demo-change\nslices: []\n").unwrap();
    // A local git worktree without an `origin` remote resolves to the
    // `local-only` push outcome — success without network.
    let alpha = tmp.path().join("alpha");
    fs::create_dir_all(&alpha).unwrap();
    run_git(&alpha, &["init", "-b", "main"]);
    fs::write(alpha.join("README.md"), "seed\n").unwrap();
    run_git(&alpha, &["add", "README.md"]);
    run_git(&alpha, &["commit", "--no-gpg-sign", "-m", "seed"]);
    fs::write(
        tmp.path().join("registry.yaml"),
        "version: 1\n\
         projects:\n\
         \x20\x20- name: alpha\n\
         \x20\x20\x20\x20url: ./alpha\n\
         \x20\x20\x20\x20adapter: omnia@1.0.0\n",
    )
    .unwrap();

    specify_cmd()
        .current_dir(tmp.path())
        .args(["workspace", "push", "--dry-run"])
        .assert()
        .success();
    assert!(
        !tmp.path().join(".specify/journal.jsonl").exists(),
        "--dry-run must not journal workspace.push.completed"
    );

    specify_cmd().current_dir(tmp.path()).args(["workspace", "push"]).assert().success();

    let raw = fs::read_to_string(tmp.path().join(".specify/journal.jsonl"))
        .expect("push must journal workspace.push.completed");
    let lines: Vec<&str> = raw.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1, "exactly one event per push, got:\n{raw}");
    assert!(lines[0].contains(r#""event":"workspace.push.completed""#), "got:\n{}", lines[0]);
    assert!(lines[0].contains(r#""plan-name":"demo-change""#), "got:\n{}", lines[0]);
    assert!(lines[0].contains(r#""branch":"specify/demo-change""#), "got:\n{}", lines[0]);
    assert!(lines[0].contains(r#""projects":["alpha"]"#), "got:\n{}", lines[0]);
}

#[test]
fn c01_push_unknown_selector_preflight() {
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");
    fs::write(tmp.path().join("plan.yaml"), "name: demo-change\nslices: []\n").unwrap();
    fs::write(
        tmp.path().join("registry.yaml"),
        "version: 1\n\
         projects:\n\
         \x20\x20- name: alpha\n\
         \x20\x20\x20\x20url: git@github.com:org/alpha.git\n\
         \x20\x20\x20\x20adapter: omnia@1.0.0\n",
    )
    .unwrap();

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "workspace", "push", "ghost", "--dry-run"])
        .assert()
        .failure();
    let value: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stderr).expect("json");
    assert_eq!(value["error"], "registry-project-selector-unknown");
    let msg = value["message"].as_str().expect("message");
    assert!(msg.contains("unknown project"), "msg: {msg}");
    assert!(msg.contains("ghost"), "msg: {msg}");
    assert!(
        !tmp.path().join("workspace/ghost").exists(),
        "unknown selector must fail before materialising the requested slot"
    );
    assert!(
        !tmp.path().join("workspace/alpha").exists(),
        "unknown selector must fail before push touches registry project slots"
    );
}

#[test]
fn c04_prepare_returns_json() {
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");

    let alpha = tmp.path().join("alpha");
    fs::create_dir_all(&alpha).unwrap();
    run_git(&alpha, &["init", "-b", "main"]);
    fs::write(alpha.join("README.md"), "seed\n").unwrap();
    run_git(&alpha, &["add", "README.md"]);
    run_git(&alpha, &["commit", "--no-gpg-sign", "-m", "seed"]);
    let remote = tmp.path().join("alpha.git");
    run_git(tmp.path(), &["clone", "--bare", alpha.to_str().unwrap(), remote.to_str().unwrap()]);
    let remote_url = format!("file://{}", remote.display());
    run_git(&alpha, &["remote", "add", "origin", &remote_url]);

    fs::write(
        tmp.path().join("registry.yaml"),
        "version: 1\n\
         projects:\n\
         \x20\x20- name: alpha\n\
         \x20\x20\x20\x20url: ./alpha\n\
         \x20\x20\x20\x20adapter: omnia@1.0.0\n",
    )
    .unwrap();

    let help = specify_cmd().args(["workspace", "--help"]).assert().success();
    let help_stdout = String::from_utf8(help.get_output().stdout.clone()).expect("help utf8");
    assert!(
        !help_stdout.contains("prepare"),
        "executor helper must stay hidden from human workspace help, got:\n{help_stdout}"
    );

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "workspace", "prepare", "alpha", "--change", "demo-change"])
        .assert()
        .success();
    let value: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("json");

    assert_eq!(value["prepared"], true);
    assert_eq!(value["project"], "alpha");
    assert_eq!(value["branch"], "specify/demo-change");
    assert_eq!(value["local-branch"], "created");
    assert_eq!(value["remote-branch"], "absent");
    assert_eq!(value["dirty"]["tracked-blocked"], serde_json::json!([]));
    assert!(
        value.get("diagnostics").is_none(),
        "PrepareBody no longer carries a diagnostics field, got: {value}"
    );
    assert_eq!(run_git(&alpha, &["branch", "--show-current"]).trim(), "specify/demo-change");
}

#[test]
fn c04_prepare_origin_head_diagnostic() {
    let tmp = tempdir().unwrap();
    init_workspace(&tmp, "platform-workspace");

    let remote = tmp.path().join("headless.git");
    run_git(tmp.path(), &["init", "--bare", remote.to_str().unwrap()]);
    let remote_url = format!("file://{}", remote.display());

    let alpha = tmp.path().join("alpha");
    fs::create_dir_all(&alpha).unwrap();
    run_git(&alpha, &["init", "-b", "main"]);
    run_git(&alpha, &["remote", "add", "origin", &remote_url]);
    fs::write(alpha.join("README.md"), "seed\n").unwrap();
    run_git(&alpha, &["add", "README.md"]);
    run_git(&alpha, &["commit", "--no-gpg-sign", "-m", "seed"]);

    fs::write(
        tmp.path().join("registry.yaml"),
        "version: 1\n\
         projects:\n\
         \x20\x20- name: alpha\n\
         \x20\x20\x20\x20url: ./alpha\n\
         \x20\x20\x20\x20adapter: omnia@1.0.0\n",
    )
    .unwrap();

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "workspace", "prepare", "alpha", "--change", "demo-change"])
        .assert()
        .failure();
    let value: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stderr).expect("json");

    assert_eq!(value["error"], "branch-preparation-failed");
    assert_eq!(value["exit-code"], 1);
    let message = value["message"].as_str().expect("message string");
    assert!(message.contains("alpha"), "message names the project: {message}");
    assert!(
        message.contains("origin-head-unresolved"),
        "message surfaces the diagnostic key: {message}"
    );
    assert_eq!(run_git(&alpha, &["branch", "--show-current"]).trim(), "main");
}

// ---- planning-path workspace smoke — planning-path workspace smoke ----
#[test]
fn planning_sync_no_registry_exits_zero() {
    let project = Project::init();
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "workspace", "sync"])
        .assert()
        .success();
    let v = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(v["synced"], false);
    assert!(v["message"].as_str().unwrap().contains("no registry"));
}

#[test]
fn planning_sync_two_symlink_peers() {
    let tmp = tempdir().expect("tempdir");
    let peer = tmp.path().join("peer-proj");
    fs::create_dir_all(peer.join(".specify")).expect("peer .specify");
    let root = tmp.path().join("root");
    fs::create_dir_all(&root).expect("root");
    specify_cmd()
        .current_dir(&root)
        .args(["init"])
        .arg(omnia_schema_dir())
        .args(["--name", "planning-ws"])
        .assert()
        .success();

    let reg = "\
version: 1
projects:
  - name: alpha
    url: .
    adapter: omnia@1.0.0
    description: Root project
  - name: beta
    url: ../peer-proj
    adapter: omnia@1.0.0
    description: Peer project
";
    fs::write(root.join("registry.yaml"), reg).expect("registry");

    specify_cmd().current_dir(&root).args(["workspace", "sync"]).assert().success();

    assert!(root.join("workspace/alpha").exists());
    assert!(root.join("workspace/beta").exists());
}

// ---- adapter-mirror conflict pins ----

/// Vendor a minimal source adapter named `docs` at the workspace root.
fn vendor_docs_adapter(root: &std::path::Path) {
    let dir = root.join("adapters/sources/docs");
    fs::create_dir_all(&dir).expect("vendor adapter dir");
    fs::write(dir.join("adapter.yaml"), "workspace copy\n").expect("vendor adapter.yaml");
}

#[test]
fn sync_mirror_skips_self_slot() {
    // A `url: .` registry entry symlinks its slot to the
    // workspace itself; mirroring there would remove-then-copy the
    // workspace cache from itself. The self-slot is skipped: the
    // vendored adapter must not be self-mirrored into the workspace's
    // own manifest cache, and the init-seeded cache survives intact.
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("root");
    fs::create_dir_all(&root).expect("root");
    specify_cmd()
        .current_dir(&root)
        .args(["init"])
        .arg(omnia_schema_dir())
        .args(["--name", "self-slot-ws"])
        .assert()
        .success();
    vendor_docs_adapter(&root);
    fs::write(
        root.join("registry.yaml"),
        "version: 1\n\
         projects:\n\
         \x20\x20- name: alpha\n\
         \x20\x20\x20\x20url: .\n\
         \x20\x20\x20\x20adapter: omnia@1.0.0\n",
    )
    .expect("registry");

    specify_cmd().current_dir(&root).args(["workspace", "sync"]).assert().success();

    assert!(
        !expected_cache_dir(&root).join("manifests/sources/docs").exists(),
        "the self-slot must be skipped: the vendored adapter must not be mirrored \
         into the workspace's own manifest cache"
    );
    assert!(
        expected_cache_dir(&root).join("manifests/targets/omnia/adapter.yaml").is_file(),
        "the init-seeded cache entry must survive a sync over a `url: .` self-slot"
    );
}

#[test]
fn sync_mirror_keeps_foreign_cache_entries() {
    // Per-name delete-then-copy GC. Cache entries the workspace
    // does not own (e.g. an init-time adapter seed in the slot) are
    // never pruned; workspace-owned names are refreshed on re-sync.
    let tmp = tempdir().expect("tempdir");
    let peer = tmp.path().join("peer-proj");
    // The peer is itself a Specify project; the slot gate keys on its
    // `.specify/` directory before mirroring adapters into the slot cache.
    fs::create_dir_all(peer.join(".specify")).expect("peer .specify");
    // The local-path peer is symlinked into `root/workspace/beta`, so its
    // slot cache lives out-of-tree keyed by the (canonical) peer path.
    let foreign = expected_cache_dir(&peer).join("manifests/sources/slot-local");
    fs::create_dir_all(&foreign).expect("foreign cache entry");
    fs::write(foreign.join("adapter.yaml"), "slot-only adapter\n").expect("foreign adapter.yaml");
    let root = tmp.path().join("root");
    fs::create_dir_all(&root).expect("root");
    specify_cmd()
        .current_dir(&root)
        .args(["init"])
        .arg(omnia_schema_dir())
        .args(["--name", "foreign-ws"])
        .assert()
        .success();
    vendor_docs_adapter(&root);
    fs::write(
        root.join("registry.yaml"),
        "version: 1\n\
         projects:\n\
         \x20\x20- name: beta\n\
         \x20\x20\x20\x20url: ../peer-proj\n\
         \x20\x20\x20\x20adapter: omnia@1.0.0\n",
    )
    .expect("registry");

    specify_cmd().current_dir(&root).args(["workspace", "sync"]).assert().success();

    let slot_cache = expected_cache_dir(&peer).join("manifests/sources");
    assert!(
        slot_cache.join("docs/adapter.yaml").is_file(),
        "sync must mirror the workspace adapter into the slot cache"
    );
    assert_eq!(
        fs::read_to_string(slot_cache.join("slot-local/adapter.yaml")).expect("foreign entry"),
        "slot-only adapter\n",
        "a cache entry the workspace does not own must survive sync"
    );

    // Re-sync refreshes workspace-owned names and still leaves the
    // foreign entry alone.
    fs::write(root.join("adapters/sources/docs/adapter.yaml"), "workspace copy v2\n")
        .expect("edit vendored adapter");
    specify_cmd().current_dir(&root).args(["workspace", "sync"]).assert().success();

    assert_eq!(
        fs::read_to_string(slot_cache.join("docs/adapter.yaml")).expect("mirrored entry"),
        "workspace copy v2\n",
        "re-sync must refresh the mirrored copy"
    );
    assert!(
        slot_cache.join("slot-local/adapter.yaml").is_file(),
        "the foreign entry must survive re-sync"
    );
}
