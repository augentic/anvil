//! Host Git ingest: exact revision, moved branch, missing commit.

use std::path::{Path, PathBuf};
use std::process::Command;

use launcher::ingest;
use project::binding::{Cache, Location, Meter, Policy, Session, check_https};
use project::workspace::{FsObjects, Store};

struct Lab {
    root: tempfile::TempDir,
    store: Store<FsObjects>,
    scratch: PathBuf,
    change: PathBuf,
}

fn lab() -> Lab {
    let root = tempfile::tempdir().expect("tempdir");
    let store = Store::new(root.path().join("snapshots"));
    let scratch = root.path().join("scratch");
    let change = root.path().join("change");
    std::fs::create_dir_all(&scratch).expect("scratch");
    std::fs::create_dir_all(&change).expect("change");
    Lab {
        root,
        store,
        scratch,
        change,
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "init.defaultBranch=main",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_TEMPLATE_DIR", "")
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

fn repo(root: &Path, files: &[(&str, &str)]) -> (PathBuf, String) {
    let dir = root.join("repo");
    std::fs::create_dir_all(&dir).expect("repo");
    git(&dir, &["init", "--template="]);
    git(&dir, &["config", "user.email", "dev@example.com"]);
    git(&dir, &["config", "user.name", "Dev"]);
    git(&dir, &["config", "commit.gpgsign", "false"]);
    for (rel, body) in files {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(path, body).expect("write");
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-m", "init"]);
    let sha = git(&dir, &["rev-parse", "HEAD"]);
    (dir, sha)
}

fn code(err: &impl std::fmt::Display) -> String {
    err.to_string()
}

async fn bind(
    lab: &Lab, location: &Location, prior: Option<&str>, cache: &mut Cache,
) -> Result<project::binding::Resolved, String> {
    let policy = Policy::standard();
    let mut meter = Meter::new();
    let mut session = Session {
        store: &lab.store,
        scratch: &lab.scratch,
        change_root: &lab.change,
        cache,
        policy: &policy,
        meter: &mut meter,
    };
    ingest(&mut session, location, None, prior, &lab.scratch).await.map_err(|err| err.to_string())
}

#[tokio::test]
async fn exact_revision() {
    let lab = lab();
    let (dir, sha) = repo(lab.root.path(), &[("src/lib.rs", "v1\n")]);
    std::fs::write(dir.join("src/lib.rs"), "v2\n").expect("edit");
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-m", "move"]);
    let locator = format!("{}@{sha}", dir.display());
    let location = Location::parse(&locator, None).expect("parse");
    let mut cache = Cache::new();
    let resolved = bind(&lab, &location, None, &mut cache).await.expect("ingest");
    match &resolved.location.locator {
        project::binding::Locator::Git { revision, .. } => assert_eq!(revision, &sha),
        other => panic!("expected git, got {other:?}"),
    }
    let out = lab.scratch.join("out");
    lab.store.materialize(&resolved.cid, &out).await.expect("materialize");
    assert_eq!(std::fs::read_to_string(out.join("src/lib.rs")).expect("read"), "v1\n");
    assert!(!out.join(".git").exists(), ".git must not enter the CID");
}

#[tokio::test]
async fn moved_branch_warns() {
    let lab = lab();
    let (dir, sha1) = repo(lab.root.path(), &[("a.txt", "one\n")]);
    std::fs::write(dir.join("a.txt"), "two\n").expect("edit");
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-m", "move"]);
    let branch = git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let locator = format!("{}@{branch}", dir.display());
    let location = Location::parse(&locator, None).expect("parse");
    let mut cache = Cache::new();
    let resolved = bind(&lab, &location, Some(&sha1), &mut cache).await.expect("ingest");
    assert!(resolved.warning.as_ref().is_some_and(|w| w.contains("moved")), "{resolved:?}");
    let out = lab.scratch.join("prior");
    lab.store.materialize(&resolved.cid, &out).await.expect("materialize");
    assert_eq!(std::fs::read_to_string(out.join("a.txt")).expect("read"), "one\n");
}

#[tokio::test]
async fn missing_commit() {
    let lab = lab();
    let (dir, _) = repo(lab.root.path(), &[("a.txt", "x\n")]);
    let sha = "0123456789abcdef0123456789abcdef01234567";
    let locator = format!("{}@{sha}", dir.display());
    let location = Location::parse(&locator, None).expect("parse");
    let mut cache = Cache::new();
    let err = bind(&lab, &location, None, &mut cache).await.expect_err("missing");
    assert!(code(&err).contains("git-revision-unavailable"), "{err}");
}

#[tokio::test]
async fn both_roles_one_cid() {
    let lab = lab();
    let (dir, sha) = repo(lab.root.path(), &[("src/a.rs", "a\n")]);
    let locator = format!("{}@{sha}", dir.display());
    let location = Location::parse(&locator, None).expect("parse");
    let mut cache = Cache::new();
    let target = bind(&lab, &location, None, &mut cache).await.expect("target");
    let source = bind(&lab, &location, None, &mut cache).await.expect("source");
    assert_eq!(target.cid, source.cid);
}

#[test]
fn https_loopback_refused() {
    let err = check_https("https://127.0.0.1/doc").expect_err("loopback");
    assert!(code(&err).contains("locator-private-network"), "{err}");
    let mut meter = Meter::new();
    let err = launcher::fetch("https://127.0.0.1/doc", &Policy::standard(), &mut meter)
        .expect_err("fetch");
    assert!(code(&err).contains("locator-private-network"), "{err}");
}
