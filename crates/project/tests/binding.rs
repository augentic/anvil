//! Locator parse, bounded-read policy, local ingest, CID reuse, and GC roots.

mod support;

use std::path::{Path, PathBuf};

use project::binding::{
    Cache, Location, Locator, Meter, Policy, Session, Staged, check_https, is_private, raw_github,
    roots, view,
};
use project::workspace::{self, Access};
use support::KernelSeam;

struct Lab {
    _root: tempfile::TempDir,
    seam: KernelSeam,
    workspaces: PathBuf,
    change: PathBuf,
    scratch: PathBuf,
}

fn lab() -> Lab {
    let root = tempfile::tempdir().expect("tempdir");
    let seam = KernelSeam::new(root.path());
    let workspaces = root.path().join("workspaces");
    let change = root.path().join("change");
    let scratch = root.path().join("scratch");
    std::fs::create_dir_all(&change).expect("change");
    std::fs::create_dir_all(&scratch).expect("scratch");
    Lab {
        seam,
        workspaces,
        change,
        scratch,
        _root: root,
    }
}

fn write(dir: &Path, rel: &str, bytes: &[u8]) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("mkdir");
    }
    std::fs::write(path, bytes).expect("write");
}

fn code(err: &impl std::fmt::Display) -> String {
    err.to_string()
}

mod parse {
    use super::*;

    #[test]
    fn git_https_path_and_ssh() {
        let git = Locator::parse(
            "https://github.com/acme/orders@0123456789abcdef0123456789abcdef01234567",
        )
        .expect("git");
        assert!(matches!(git, Locator::Git { .. }));
        assert_eq!(
            git.key(),
            "https://github.com/acme/orders@0123456789abcdef0123456789abcdef01234567"
        );

        let ssh = Locator::parse("git@github.com:acme/orders@main").expect("ssh");
        assert!(matches!(ssh, Locator::Git { revision, .. } if revision == "main"));

        let https = Locator::parse("https://example.com/docs/api.md").expect("https");
        assert!(matches!(https, Locator::Https(_)));

        let rel = Locator::parse("./notes.md").expect("rel");
        assert!(matches!(rel, Locator::Path(path) if path == Path::new("./notes.md")));

        let abs = Locator::parse("/var/src").expect("abs");
        assert!(matches!(abs, Locator::Path(path) if path == Path::new("/var/src")));
    }

    #[test]
    fn http_and_userinfo() {
        let err = Locator::parse("http://example.com/file").expect_err("http");
        assert!(code(&err).contains("locator-http-unsupported"), "{err}");

        let err = Locator::parse("https://user:pass@example.com/file.md").expect_err("creds");
        assert!(code(&err).contains("locator-credentials-forbidden"), "{err}");

        let err = Location::parse("/repo", Some("/abs")).expect_err("abs selector");
        assert!(code(&err).contains("locator-malformed"), "{err}");
    }

    #[test]
    fn strips_git_suffix() {
        let git = Locator::parse("https://github.com/acme/orders.git@main").expect("git");
        match git {
            Locator::Git { url, revision } => {
                assert_eq!(url, "https://github.com/acme/orders");
                assert_eq!(revision, "main");
            }
            other => panic!("expected git, got {other:?}"),
        }
    }
}

mod gate {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use super::*;

    #[test]
    fn github_raw_rewrite() {
        assert_eq!(
            raw_github("https://github.com/acme/orders/blob/main/README.md"),
            "https://raw.githubusercontent.com/acme/orders/main/README.md"
        );
        assert_eq!(raw_github("https://example.com/file.md"), "https://example.com/file.md");
    }

    #[test]
    fn private_and_localhost() {
        let err = check_https("https://127.0.0.1/secret").expect_err("loopback");
        assert!(code(&err).contains("locator-private-network"), "{err}");

        let err = check_https("https://localhost/secret").expect_err("localhost");
        assert!(code(&err).contains("locator-private-network"), "{err}");

        let err = check_https("https://192.168.1.9/x").expect_err("rfc1918");
        assert!(code(&err).contains("locator-private-network"), "{err}");

        check_https("https://example.com/doc.md").expect("public host");
        assert!(is_private(IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert!(is_private(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(!is_private(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
    }
}

mod ingest {
    use super::*;

    async fn pin(lab: &Lab, location: &Location) -> project::binding::Resolved {
        let mut cache = Cache::new();
        let policy = Policy::standard();
        let mut meter = Meter::new();
        let mut session = Session {
            workspaces: &lab.seam,
            scratch: &lab.scratch,
            change_root: &lab.change,
            cache: &mut cache,
            policy: &policy,
            meter: &mut meter,
        };
        session.ingest(location, Staged::Disk, None, None).await.expect("ingest")
    }

    #[tokio::test]
    async fn file_is_one_file_tree() {
        let lab = lab();
        write(&lab.change, "notes.md", b"# hello\n");
        let location = Location::parse("notes.md", None).expect("parse");
        let resolved = pin(&lab, &location).await;
        let out = lab.scratch.join("out");
        lab.seam.store.materialize(&resolved.cid, &out).await.expect("materialize");
        assert_eq!(std::fs::read_to_string(out.join("notes.md")).expect("read"), "# hello\n");
        assert_eq!(std::fs::read_dir(&out).expect("list").count(), 1);
    }

    #[tokio::test]
    async fn both_roles_reuse_cid() {
        let lab = lab();
        write(&lab.change, "src/lib.rs", b"pub fn x() {}\n");
        let location = Location::parse("src", None).expect("parse");
        let mut cache = Cache::new();
        let policy = Policy::standard();
        let mut meter = Meter::new();
        let mut session = Session {
            workspaces: &lab.seam,
            scratch: &lab.scratch,
            change_root: &lab.change,
            cache: &mut cache,
            policy: &policy,
            meter: &mut meter,
        };
        let a = session.ingest(&location, Staged::Disk, None, None).await.expect("a");
        let b = session.ingest(&location, Staged::Disk, None, None).await.expect("b");
        assert_eq!(a.cid, b.cid);
    }

    #[tokio::test]
    async fn recorded_cid_skips_origin() {
        let lab = lab();
        write(&lab.change, "a.txt", b"one\n");
        let location = Location::parse("a.txt", None).expect("parse");
        let first = pin(&lab, &location).await;
        std::fs::write(lab.change.join("a.txt"), b"two\n").expect("mutate");
        let mut cache = Cache::new();
        let policy = Policy::standard();
        let mut meter = Meter::new();
        let mut session = Session {
            workspaces: &lab.seam,
            scratch: &lab.scratch,
            change_root: &lab.change,
            cache: &mut cache,
            policy: &policy,
            meter: &mut meter,
        };
        let again = session
            .ingest(&location, Staged::Disk, Some(&first.cid), None)
            .await
            .expect("recorded");
        assert_eq!(again.cid, first.cid);
        let out = lab.scratch.join("recorded");
        lab.seam.store.materialize(&again.cid, &out).await.expect("materialize");
        assert_eq!(std::fs::read_to_string(out.join("a.txt")).expect("read"), "one\n");
    }

    #[tokio::test]
    async fn path_selector_and_missing() {
        let lab = lab();
        write(&lab.change, "tree/docs/api.md", b"api\n");
        write(&lab.change, "tree/src/lib.rs", b"lib\n");
        let location = Location::parse("tree", Some("docs")).expect("parse");
        let resolved = pin(&lab, &location).await;
        let out = lab.scratch.join("sel");
        lab.seam.store.materialize(&resolved.cid, &out).await.expect("materialize");
        assert!(out.join("api.md").is_file());
        assert!(!out.join("src").exists());

        let missing = Location::parse("tree", Some("nope")).expect("parse");
        let err = {
            let mut cache = Cache::new();
            let policy = Policy::standard();
            let mut meter = Meter::new();
            let mut session = Session {
                workspaces: &lab.seam,
                scratch: &lab.scratch,
                change_root: &lab.change,
                cache: &mut cache,
                policy: &policy,
                meter: &mut meter,
            };
            session.ingest(&missing, Staged::Disk, None, None).await.expect_err("missing")
        };
        assert!(code(&err).contains("locator-path-missing"), "{err}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn escaping_symlink_refused() {
        let lab = lab();
        write(&lab.change, "tree/ok.txt", b"ok\n");
        std::os::unix::fs::symlink("../outside.txt", lab.change.join("tree/link"))
            .expect("symlink");
        write(&lab.change, "outside.txt", b"secret\n");
        let location = Location::parse("tree", None).expect("parse");
        let err = {
            let mut cache = Cache::new();
            let policy = Policy::standard();
            let mut meter = Meter::new();
            let mut session = Session {
                workspaces: &lab.seam,
                scratch: &lab.scratch,
                change_root: &lab.change,
                cache: &mut cache,
                policy: &policy,
                meter: &mut meter,
            };
            session.ingest(&location, Staged::Disk, None, None).await.expect_err("escape")
        };
        assert!(code(&err).contains("locator-symlink-escape"), "{err}");
    }

    #[tokio::test]
    async fn binding_budget() {
        let lab = lab();
        write(&lab.change, "a.txt", b"a\n");
        let location = Location::parse("a.txt", None).expect("parse");
        let policy = Policy {
            bindings: 1,
            ..Policy::standard()
        };
        let mut cache = Cache::new();
        let mut meter = Meter::new();
        let mut session = Session {
            workspaces: &lab.seam,
            scratch: &lab.scratch,
            change_root: &lab.change,
            cache: &mut cache,
            policy: &policy,
            meter: &mut meter,
        };
        session.ingest(&location, Staged::Disk, None, None).await.expect("first");
        let err = session.ingest(&location, Staged::Disk, None, None).await.expect_err("cap");
        assert!(code(&err).contains("binding-budget-exhausted"), "{err}");
    }

    #[tokio::test]
    async fn view_refuses_capture() {
        let lab = lab();
        write(&lab.change, "a.txt", b"a\n");
        let resolved = pin(&lab, &Location::parse("a.txt", None).expect("parse")).await;
        let ws = view(&lab.seam.store, &lab.workspaces, &resolved.cid).await.expect("view");
        assert!(!ws.writable);
        let err = workspace::capture(&lab.seam.store, &lab.workspaces, &ws.id)
            .await
            .expect_err("capture");
        assert!(code(&err).contains("read-only"), "{err}");
        workspace::discard(&lab.workspaces, &ws.id).expect("discard");
    }

    #[tokio::test]
    async fn gc_roots_retained() {
        let lab = lab();
        write(&lab.change, "keep.txt", b"keep\n");
        write(&lab.change, "drop.txt", b"drop\n");
        let keep = pin(&lab, &Location::parse("keep.txt", None).expect("parse")).await;
        let gone = pin(&lab, &Location::parse("drop.txt", None).expect("parse")).await;
        let live = roots(std::slice::from_ref(&keep));
        let removed =
            lab.seam.store.sweep(std::slice::from_ref(&gone.cid), &live).await.expect("sweep");
        assert!(removed > 0, "dead tree objects should be collected");
        assert!(lab.seam.store.contains(&keep.cid).await);
        assert!(!lab.seam.store.contains(&gone.cid).await);

        let _writable = workspace::prepare(
            &lab.seam.store,
            &lab.workspaces,
            &keep.cid,
            Access { writable: true },
        )
        .await
        .expect("live root still prepares");
    }
}
