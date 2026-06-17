use std::path::PathBuf;

use tempfile::TempDir;

use super::{LockProbe, probe};

fn lock_path(dir: &TempDir) -> PathBuf {
    let specify = dir.path().join(".specify");
    std::fs::create_dir_all(&specify).expect("mkdir .specify");
    specify.join("plan.lock")
}

#[test]
fn missing_lockfile_is_unheld() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(".specify").join("plan.lock");
    assert_eq!(probe(&path).expect("probe"), LockProbe::Unheld);
}

#[test]
fn unlocked_file_is_unheld() {
    let dir = TempDir::new().expect("tempdir");
    let path = lock_path(&dir);
    std::fs::write(&path, "pid=0\n").expect("seed lockfile");
    assert_eq!(probe(&path).expect("probe"), LockProbe::Unheld);
}

#[cfg(unix)]
mod unix {
    use std::fs::File;
    use std::path::Path;

    use jiff::Timestamp;
    use specify_error::Error;

    use super::*;
    use crate::config::Layout;
    use crate::plan_lock::{acquire, require_held};

    fn now() -> Timestamp {
        "2026-05-21T20:00:00Z".parse().expect("fixed timestamp")
    }

    /// Hold a `flock`-family exclusive lock for the guard's lifetime —
    /// the same lock family `specify plan lock` (and the probe) use.
    /// Dropping the guard closes the descriptor, which releases the
    /// lock.
    struct FlockGuard {
        _file: File,
    }

    impl FlockGuard {
        fn acquire(path: &Path) -> Self {
            let file = File::options()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(path)
                .expect("open lockfile");
            file.lock().expect("test flock acquire failed");
            Self { _file: file }
        }
    }

    #[test]
    fn flock_holder_is_held() {
        let dir = TempDir::new().expect("tempdir");
        let path = lock_path(&dir);
        let guard = FlockGuard::acquire(&path);
        assert_eq!(probe(&path).expect("probe"), LockProbe::Held);
        drop(guard);
        assert_eq!(probe(&path).expect("probe"), LockProbe::Unheld, "release must unlock");
    }

    #[test]
    fn require_held_refuses_unlocked() {
        let dir = TempDir::new().expect("tempdir");
        let _path = lock_path(&dir);
        let err = require_held(Layout::new(dir.path())).expect_err("must refuse");
        match err {
            Error::Validation { code, .. } => assert_eq!(code, "plan-lock-not-held"),
            other => panic!("expected Error::Validation, got {other:?}"),
        }
    }

    #[test]
    fn require_held_passes_under_lock() {
        let dir = TempDir::new().expect("tempdir");
        let path = lock_path(&dir);
        let _guard = FlockGuard::acquire(&path);
        require_held(Layout::new(dir.path())).expect("held lock must pass");
    }

    /// `acquire` creates the lockfile, takes the lock (the probe sees
    /// it held and `require_held` passes), and writes the diagnostic
    /// pid body; dropping the guard releases it.
    #[test]
    fn acquire_holds_then_releases() {
        let dir = TempDir::new().expect("tempdir");
        let guard = acquire(Layout::new(dir.path()), now()).expect("acquire");
        let path = dir.path().join(".specify").join("plan.lock");
        assert_eq!(probe(&path).expect("probe"), LockProbe::Held);
        require_held(Layout::new(dir.path())).expect("held lock must pass require_held");
        let body = std::fs::read_to_string(&path).expect("read body");
        assert!(body.contains(&format!("pid={}", std::process::id())), "pid body: {body}");
        drop(guard);
        assert_eq!(probe(&path).expect("probe"), LockProbe::Unheld, "drop must release");
    }

    /// A second `acquire` while the first guard is alive fails fast with
    /// `plan-lock-busy`, surfacing the holder pid from the body.
    #[test]
    fn acquire_busy_when_held() {
        let dir = TempDir::new().expect("tempdir");
        let _guard = acquire(Layout::new(dir.path()), now()).expect("first acquire");
        let err = acquire(Layout::new(dir.path()), now()).expect_err("second acquire must refuse");
        match err {
            Error::Validation { code, detail, .. } => {
                assert_eq!(code, "plan-lock-busy");
                assert!(
                    detail.contains(&format!("holder-pid={}", std::process::id())),
                    "busy detail must carry holder pid: {detail}"
                );
            }
            other => panic!("expected Error::Validation, got {other:?}"),
        }
    }

    /// The probe itself must not leave the lock held (the try-acquire
    /// arm releases on success): two probes in a row both say unheld,
    /// and an acquire still succeeds afterwards.
    #[test]
    fn probe_leaves_lock_free() {
        let dir = TempDir::new().expect("tempdir");
        let path = lock_path(&dir);
        std::fs::write(&path, "").expect("seed lockfile");
        assert_eq!(probe(&path).expect("first probe"), LockProbe::Unheld);
        assert_eq!(probe(&path).expect("second probe"), LockProbe::Unheld);
        let _guard = FlockGuard::acquire(&path);
        assert_eq!(probe(&path).expect("probe under lock"), LockProbe::Held);
    }
}
