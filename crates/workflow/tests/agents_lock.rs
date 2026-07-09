//! Integration coverage for the AGENTS.md context-lock sidecar codec
//! (`workflow::agents::lock::{load, save}`): the cold-start read (missing →
//! `None`), the save/load round-trip, and `snake_case` serialisation all run
//! through the public codec. The version gate and the test-only `diff_inputs`
//! helper keep their in-`src` unit tests (no public input reaches the latter).

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
