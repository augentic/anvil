//! YAML sidecar for init-time AGENTS.md generation fingerprints.

#[cfg(test)]
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};
use specify_error::Error;
use specify_model::atomic::yaml_write;

use super::fingerprint::ContextFingerprint;

const CURRENT_LOCK_VERSION: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextLock {
    pub version: u64,
    pub fingerprint: String,
    pub cli_version: String,
    pub inputs: Vec<Input>,
    pub fences: Fences,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fences {
    pub body_sha256: String,
}

#[derive(Debug, Deserialize)]
struct Version {
    version: u64,
}

impl ContextLock {
    // YAML sidecar persisted to disk — not a wire DTO, so it keeps a
    // named constructor rather than a `From`-for-Body/Row impl.
    pub fn from_fingerprint(fingerprint: &ContextFingerprint) -> Self {
        Self {
            version: CURRENT_LOCK_VERSION,
            fingerprint: fingerprint.fingerprint.clone(),
            cli_version: fingerprint.cli_version.clone(),
            inputs: fingerprint
                .inputs
                .iter()
                .map(|input| Input {
                    path: input.path.clone(),
                    sha256: input.sha256.clone(),
                })
                .collect(),
            fences: Fences {
                body_sha256: fingerprint.body_sha256.clone(),
            },
        }
    }
}

pub fn load(path: &Path) -> Result<Option<ContextLock>, Error> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(Error::Io(err)),
    };

    let version: Version = serde_saphyr::from_str(&contents).map_err(|err| {
        validation_error(
            "context-lock-malformed",
            format!("context-lock-malformed: failed to read lock version: {err}"),
        )
    })?;
    if version.version > CURRENT_LOCK_VERSION {
        return Err(validation_error(
            "context-lock-version-too-new",
            format!(
                "context-lock-version-too-new: lock version {} > supported {CURRENT_LOCK_VERSION}",
                version.version
            ),
        ));
    }
    if version.version != CURRENT_LOCK_VERSION {
        return Err(validation_error(
            "context-lock-malformed",
            format!(
                "context-lock-malformed: unsupported lock version {}; expected \
                 {CURRENT_LOCK_VERSION}",
                version.version
            ),
        ));
    }

    let lock: ContextLock = serde_saphyr::from_str(&contents).map_err(|err| {
        validation_error("context-lock-malformed", format!("context-lock-malformed: {err}"))
    })?;
    Ok(Some(lock))
}

pub fn save(path: &Path, lock: &ContextLock) -> Result<(), Error> {
    // ContextLock isn't a Plan/Registry/ProjectConfig sibling; its load
    // path returns a typed Validation envelope rather than `Option<Self>`,
    // so it doesn't fit the AtomicYaml shape.
    yaml_write(path, lock)
}

fn validation_error(rule_id: &'static str, detail: String) -> Error {
    Error::validation_failed(rule_id, "context.lock must be a supported context lock file", detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct InputDiff {
        pub changed: Vec<String>,
        pub added: Vec<String>,
        pub removed: Vec<String>,
    }

    pub fn diff_inputs(expected: &[Input], actual: &[Input]) -> InputDiff {
        let expected_by_path = inputs_by_path(expected);
        let actual_by_path = inputs_by_path(actual);

        let changed = expected_by_path
            .iter()
            .filter_map(|(path, expected_sha)| {
                actual_by_path
                    .get(path)
                    .filter(|actual_sha| *actual_sha != expected_sha)
                    .map(|_actual_sha| path.clone())
            })
            .collect();
        let added = actual_by_path
            .keys()
            .filter(|path| !expected_by_path.contains_key(*path))
            .cloned()
            .collect();
        let removed = expected_by_path
            .keys()
            .filter(|path| !actual_by_path.contains_key(*path))
            .cloned()
            .collect();

        InputDiff {
            changed,
            added,
            removed,
        }
    }

    fn inputs_by_path(inputs: &[Input]) -> BTreeMap<String, String> {
        inputs.iter().map(|input| (input.path.clone(), input.sha256.clone())).collect()
    }

    fn input(path: &str, sha256: &str) -> Input {
        Input {
            path: path.to_string(),
            sha256: sha256.to_string(),
        }
    }

    #[test]
    fn input_diff_sorts_by_path() {
        let expected = vec![input("a.txt", "old"), input("b.txt", "same"), input("d.txt", "gone")];
        let actual = vec![input("a.txt", "new"), input("b.txt", "same"), input("c.txt", "added")];

        let diff = diff_inputs(&expected, &actual);

        assert_eq!(diff.changed, vec!["a.txt"]);
        assert_eq!(diff.added, vec!["c.txt"]);
        assert_eq!(diff.removed, vec!["d.txt"]);
    }

    #[test]
    fn lock_serializes_snake_case_keys() {
        let lock = ContextLock {
            version: 1,
            fingerprint: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            cli_version: "0.2.0".to_string(),
            inputs: vec![input(
                ".specify/project.yaml",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            )],
            fences: Fences {
                body_sha256:
                    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                        .to_string(),
            },
        };

        let yaml = serde_saphyr::to_string(&lock).expect("serialize lock");

        assert!(yaml.contains("cli_version: 0.2.0"), "{yaml}");
        assert!(yaml.contains("body_sha256: sha256:cccc"), "{yaml}");
        assert!(!yaml.contains("cli-version"), "{yaml}");
        assert!(!yaml.contains("body-sha256"), "{yaml}");
    }

    fn sample_lock() -> ContextLock {
        ContextLock {
            version: CURRENT_LOCK_VERSION,
            fingerprint: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            cli_version: "0.2.0".to_string(),
            inputs: vec![input(
                ".specify/project.yaml",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            )],
            fences: Fences {
                body_sha256:
                    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                        .to_string(),
            },
        }
    }

    // A missing lock file is the cold-start path and must read as
    // `Ok(None)`, not an error.
    #[test]
    fn load_missing_is_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("context.lock");
        assert_eq!(load(&path).expect("missing lock is ok"), None);
    }

    // `save` then `load` must round-trip a lock byte-for-byte through the
    // YAML codec.
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
}
