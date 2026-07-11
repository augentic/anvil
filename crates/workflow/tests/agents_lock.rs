//! Integration coverage for the AGENTS.md context-lock sidecar codec
//! (`workflow::agents::lock::{load, save}`): the cold-start read (missing →
//! `None`), the save/load round-trip, `snake_case` serialisation, and the
//! version gate's three failure shapes all run through the public codec.

use std::fs;

use error::Error;
use workflow::agents::lock::{ContextLock, Fences, Input, load, save};

fn input(path: &str, sha256: &str) -> Input {
    Input {
        path: path.to_string(),
        sha256: sha256.to_string(),
    }
}

fn sample_lock() -> ContextLock {
    ContextLock {
        version: 1,
        fingerprint: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_string(),
        cli_version: "0.2.0".to_string(),
        inputs: vec![input(
            ".specify/project.yaml",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )],
        fences: Fences {
            body_sha256: "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                .to_string(),
        },
    }
}

#[test]
fn lock_serializes_snake_case_keys() {
    let yaml = serde_saphyr::to_string(&sample_lock()).expect("serialize lock");
    assert!(yaml.contains("cli_version: 0.2.0"), "{yaml}");
    assert!(yaml.contains("body_sha256: sha256:cccc"), "{yaml}");
    assert!(!yaml.contains("cli-version"), "{yaml}");
    assert!(!yaml.contains("body-sha256"), "{yaml}");
}

// A missing lock file is the cold-start path and must read as `Ok(None)`.
#[test]
fn load_missing_is_none() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("context.lock");
    assert_eq!(load(&path).expect("missing lock is ok"), None);
}

// `save` then `load` must round-trip a lock through the YAML codec.
#[test]
fn save_load_round_trips() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("context.lock");
    let lock = sample_lock();
    save(&path, &lock).expect("save lock");
    assert_eq!(load(&path).expect("load lock"), Some(lock));
}

// The version gate distinguishes three failure shapes: a future
// version (forward-incompatible), an unsupported older version, and
// syntactically broken YAML. Each maps to its own closed rule id so
// the operator gets an actionable message.
#[test]
fn load_version_gate() {
    let dir = tempfile::tempdir().expect("tempdir");

    let too_new = dir.path().join("new.lock");
    fs::write(&too_new, "version: 2\n").expect("write");
    assert!(
        matches!(load(&too_new), Err(Error::Validation { code, .. }) if code == "context-lock-version-too-new"),
        "future version must be rejected with its own code"
    );

    let zero = dir.path().join("zero.lock");
    fs::write(&zero, "version: 0\n").expect("write");
    assert!(
        matches!(load(&zero), Err(Error::Validation { code, .. }) if code == "context-lock-malformed"),
        "an unsupported lower version is malformed"
    );

    let garbage = dir.path().join("garbage.lock");
    fs::write(&garbage, ": not yaml :\n").expect("write");
    assert!(
        matches!(load(&garbage), Err(Error::Validation { code, .. }) if code == "context-lock-malformed"),
        "unparseable YAML is malformed"
    );
}
