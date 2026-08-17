//! The VCS fetch kernel behind the `emery:vcs/trees` seam (both
//! credential legs) and the engine bind flow over it: exact SHA,
//! moved branch (engine-side comparison), missing commit, both-roles
//! CID reuse. The ambient Git clone leg against a live remote is
//! covered by the operator-invoked rungs.

mod support;

use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

use project::binding::{Cache, Location, Meter, Policy, Session, resolve};
use project::seam::{TreeCredentials, TreeError, TreeLimits};
use project::vcs;
use support::KernelSeam;

struct Lab {
    root: tempfile::TempDir,
    seam: KernelSeam,
    scratch: PathBuf,
    change: PathBuf,
}

fn lab() -> Lab {
    let root = tempfile::tempdir().expect("tempdir");
    let seam = KernelSeam::new(root.path());
    let scratch = root.path().join("scratch");
    let change = root.path().join("change");
    std::fs::create_dir_all(&scratch).expect("scratch");
    std::fs::create_dir_all(&change).expect("change");
    Lab {
        root,
        seam,
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

/// Serve `body` (HTTP 200) for every connection on a fresh local
/// port. Git's `ls-remote` probe consumes connections before the
/// document leg, so the server thread loops for the process's life.
fn serve(body: &'static str) -> std::net::SocketAddr {
    serve_response(move || {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    })
}

/// Serve an empty-bodied `status` response for every connection.
fn serve_status(status: &'static str) -> std::net::SocketAddr {
    serve_response(move || {
        format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
    })
}

fn serve_response(response: impl Fn() -> String + Send + 'static) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut request = [0_u8; 2048];
            drop(stream.read(&mut request));
            drop(stream.write_all(response().as_bytes()));
        }
    });
    addr
}

mod kernel {
    use super::*;

    #[test]
    fn classify() {
        for remote in
            ["https://erp.example.com", "http://a/b", "ssh://git@host/r.git", "git@h:r.git"]
        {
            assert!(vcs::is_remote(remote), "{remote}");
        }
        for local in ["./orders", "orders", "/abs/orders", "../up", "file:///tree"] {
            assert!(!vcs::is_remote(local), "{local}");
        }
    }

    #[test]
    fn ambient_local_refused() {
        let staging = tempfile::tempdir().expect("tempdir");
        let err = vcs::fetch(
            staging.path(),
            "./orders",
            TreeCredentials::Ambient,
            &TreeLimits::unbounded(),
        )
        .expect_err("local paths never fetch");
        assert!(matches!(&err, TreeError::InvalidRequest(_)), "{err}");
    }

    #[test]
    fn ambient_document_fetch() {
        let staging = tempfile::tempdir().expect("tempdir");
        let body = "openapi: 3.1.0\n";
        let locator = format!("http://{}/specs/orders.yaml", serve(body));

        let fetched = vcs::fetch(
            staging.path(),
            &locator,
            TreeCredentials::Ambient,
            &TreeLimits::unbounded(),
        )
        .expect("document origin fetches");
        assert!(fetched.name.starts_with("vcs-"), "{}", fetched.name);
        assert_eq!(fetched.dir, staging.path().join(&fetched.name));
        assert_eq!(fetched.revision, None, "a document origin reports no revision");
        let document = fetched.dir.join("orders.yaml");
        assert_eq!(std::fs::read_to_string(document).expect("downloaded document"), body);

        // Discard removes the tree and is idempotent.
        vcs::discard(staging.path(), &fetched.name).expect("discard");
        assert!(!fetched.dir.exists());
        vcs::discard(staging.path(), &fetched.name).expect("discard is idempotent");
    }

    #[test]
    fn ambient_fetch_refused() {
        // A refused origin is a typed fetch failure, not a panic or
        // an empty tree — and no partial staged tree survives (D11).
        let staging = tempfile::tempdir().expect("tempdir");
        let locator = format!("http://{}/gone", serve_status("404 Not Found"));
        let err = vcs::fetch(
            staging.path(),
            &locator,
            TreeCredentials::Ambient,
            &TreeLimits::unbounded(),
        )
        .expect_err("a 404 origin fails");
        assert!(matches!(&err, TreeError::Access(_)), "{err}");
        assert_staging_clean(staging.path());
    }

    #[test]
    fn ambient_over_limit() {
        // D11: the ambient document leg is metered — a body past the
        // byte cap is a typed limit failure and staging stays clean.
        let staging = tempfile::tempdir().expect("tempdir");
        let body = "x".repeat(64);
        let leaked: &'static str = Box::leak(body.into_boxed_str());
        let locator = format!("http://{}/big.yaml", serve(leaked));
        let limits = TreeLimits {
            max_bytes: 16,
            max_redirects: 4,
            time_ms: 60_000,
        };
        let err = vcs::fetch(staging.path(), &locator, TreeCredentials::Ambient, &limits)
            .expect_err("over-cap body fails");
        assert!(matches!(&err, TreeError::Limit(_)), "{err}");
        assert_staging_clean(staging.path());
    }

    /// No `vcs-*` staged tree survives a failed fetch.
    fn assert_staging_clean(staging: &Path) {
        let leftovers: Vec<String> = std::fs::read_dir(staging)
            .expect("staging root")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with("vcs-"))
            .collect();
        assert!(leftovers.is_empty(), "failed fetch left staged trees: {leftovers:?}");
    }

    #[test]
    fn hardened_exact_sha() {
        let staging = tempfile::tempdir().expect("tempdir");
        let (dir, sha) = repo(staging.path(), &[("src/lib.rs", "v1\n")]);
        std::fs::write(dir.join("src/lib.rs"), "v2\n").expect("edit");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-m", "move"]);

        let locator = format!("{}@{sha}", dir.display());
        let fetched =
            vcs::fetch(staging.path(), &locator, TreeCredentials::None, &TreeLimits::unbounded())
                .expect("hardened checkout");
        assert_eq!(fetched.revision.as_deref(), Some(sha.as_str()));
        assert_eq!(std::fs::read_to_string(fetched.dir.join("src/lib.rs")).expect("read"), "v1\n");
        vcs::discard(staging.path(), &fetched.name).expect("discard");
    }

    #[test]
    fn hardened_loopback_refused() {
        let staging = tempfile::tempdir().expect("tempdir");
        let err = vcs::fetch(
            staging.path(),
            "https://127.0.0.1/doc",
            TreeCredentials::None,
            &TreeLimits::unbounded(),
        )
        .expect_err("loopback");
        assert!(code(&err).contains("locator-private-network"), "{err}");
    }

    #[test]
    fn hardened_path_refused() {
        let staging = tempfile::tempdir().expect("tempdir");
        let err =
            vcs::fetch(staging.path(), "./local", TreeCredentials::None, &TreeLimits::unbounded())
                .expect_err("path locators never fetch");
        assert!(matches!(&err, TreeError::InvalidRequest(_)), "{err}");
    }

    #[test]
    fn discard_grammar() {
        let staging = tempfile::tempdir().expect("tempdir");
        for name in ["..", "workspace-1", "vcs-", "vcs-a/b", "vcs-a..b", ""] {
            let err = vcs::discard(staging.path(), name).expect_err(name);
            assert!(matches!(&err, TreeError::InvalidRequest(_)), "{name}: {err}");
        }
    }

    #[test]
    fn sweep_stale() {
        let staging = tempfile::tempdir().expect("tempdir");
        let abandoned = staging.path().join("vcs-dead");
        std::fs::create_dir_all(&abandoned).expect("abandoned");
        let kept = staging.path().join("workspace-live");
        std::fs::create_dir_all(&kept).expect("kept");
        vcs::sweep_stale(staging.path(), std::time::Duration::ZERO);
        assert!(!abandoned.exists(), "stale vcs-* trees sweep");
        assert!(kept.exists(), "non-vcs entries are never swept");
    }
}

mod bind {
    use super::*;

    async fn bind_tree(
        lab: &Lab, location: &Location, recorded: Option<&project::snapshot::SnapshotId>,
        prior: Option<&str>, cache: &mut Cache,
    ) -> Result<(project::binding::Resolved, PathBuf), String> {
        let policy = Policy::standard();
        let mut meter = Meter::new();
        let mut session = Session {
            workspaces: &lab.seam,
            scratch: &lab.scratch,
            change_root: &lab.change,
            cache,
            policy: &policy,
            meter: &mut meter,
        };
        resolve(&mut session, &lab.seam, location, recorded, prior)
            .await
            .map_err(|err| err.to_string())
    }

    async fn bind(
        lab: &Lab, location: &Location, prior: Option<&str>, cache: &mut Cache,
    ) -> Result<project::binding::Resolved, String> {
        bind_tree(lab, location, None, prior, cache).await.map(|(resolved, _)| resolved)
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
        lab.seam.store.materialize(&resolved.cid, &out).await.expect("materialize");
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
        lab.seam.store.materialize(&resolved.cid, &out).await.expect("materialize");
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
        let (target, _) = bind_tree(&lab, &location, None, None, &mut cache).await.expect("target");
        let (source, root) =
            bind_tree(&lab, &location, None, None, &mut cache).await.expect("source");
        assert_eq!(target.cid, source.cid);
        assert_ne!(root, lab.scratch, "intern skip must not return the ingest scratch");
        assert_eq!(std::fs::read_to_string(root.join("src/a.rs")).expect("read"), "a\n");
    }

    #[tokio::test]
    async fn recorded_skip_tree() {
        let lab = lab();
        let (dir, sha) = repo(
            lab.root.path(),
            &[
                (".emery/project.yaml", "name: app\nadapter: omnia\nrules: {}\n"),
                ("src/lib.rs", "ok\n"),
            ],
        );
        let locator = format!("{}@{sha}", dir.display());
        let location = Location::parse(&locator, None).expect("parse");
        let mut cache = Cache::new();
        let first = bind(&lab, &location, None, &mut cache).await.expect("first");
        std::fs::remove_dir_all(&dir).expect("drop origin");
        let mut cache = Cache::new();
        let (again, root) =
            bind_tree(&lab, &location, Some(&first.cid), None, &mut cache).await.expect("recorded");
        assert_eq!(again.cid, first.cid);
        assert_ne!(root, lab.scratch, "recorded skip must not return the ingest scratch");
        assert!(root.join(".emery/project.yaml").is_file());
        assert_eq!(std::fs::read_to_string(root.join("src/lib.rs")).expect("read"), "ok\n");
    }

    #[tokio::test]
    async fn https_skip_tree() {
        let lab = lab();
        let brief = lab.change.join("brief.md");
        std::fs::write(&brief, b"Ship the greeting.\n").expect("brief");
        let cid = lab.seam.store.snapshot_path(&brief).await.expect("seed");
        let location = Location::parse("https://example.com/brief.md", None).expect("https");
        let mut cache = Cache::new();
        let (resolved, root) =
            bind_tree(&lab, &location, Some(&cid), None, &mut cache).await.expect("recorded");
        assert_eq!(resolved.cid, cid);
        assert_ne!(root, lab.scratch, "HTTPS skip must not return the ingest scratch");
        assert_eq!(
            std::fs::read_to_string(root.join("brief.md")).expect("read"),
            "Ship the greeting.\n"
        );
    }
}
