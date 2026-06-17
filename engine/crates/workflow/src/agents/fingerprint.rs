//! Deterministic fingerprinting for generated context inputs.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use specify_error::Error;

/// One renderer input file and its content digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputFingerprint {
    /// Repo-relative path, using `/` separators.
    pub path: String,
    /// Lowercase hex SHA-256 digest of the input bytes.
    pub sha256: String,
}

/// Fingerprint values persisted in `.specify/context.lock`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextFingerprint {
    /// `sha256:<hex>` digest over the canonical aggregate input.
    pub fingerprint: String,
    /// CLI version included as the first line of the aggregate input.
    pub cli_version: String,
    /// Sorted per-file inputs.
    pub inputs: Vec<InputFingerprint>,
    /// `sha256:<hex>` digest of the bytes between the context fences.
    pub body_sha256: String,
}

/// Collects candidate paths and hashes their bytes in deterministic order.
#[derive(Debug, Clone)]
pub struct InputCollector {
    project_dir: PathBuf,
    paths: BTreeMap<String, PathBuf>,
}

impl InputCollector {
    /// Start collecting inputs for a project root.
    pub fn new(project_dir: &Path) -> Self {
        Self {
            project_dir: project_dir.to_path_buf(),
            paths: BTreeMap::new(),
        }
    }

    /// Add a required input file.
    pub fn add_file(&mut self, path: &Path) -> Result<(), Error> {
        let relative = repo_relative_path(&self.project_dir, path)?;
        self.paths.entry(relative).or_insert_with(|| path.to_path_buf());
        Ok(())
    }

    /// Record an input file under an explicit logical repo-relative
    /// path, decoupling the recorded provenance from the file's physical
    /// location. Used for out-of-tree cache inputs (the manifest mirror,
    /// codex) so context fingerprints stay stable and project-relative
    /// even though the bytes live in the OS cache.
    pub fn add_file_as(&mut self, logical_relative: &str, physical: &Path) {
        self.paths.entry(logical_relative.to_string()).or_insert_with(|| physical.to_path_buf());
    }

    /// Add an input file only when it exists as a regular file.
    pub fn add_file_if_present(&mut self, path: &Path) -> Result<(), Error> {
        match path.try_exists() {
            Ok(true) if path.is_file() => self.add_file(path),
            Ok(_) => Ok(()),
            Err(err) => Err(Error::Io(err)),
        }
    }

    /// Add repo-relative input paths captured by another renderer component.
    pub fn add_relative_files<I, S>(&mut self, paths: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for path in paths {
            self.add_file(&self.project_dir.join(path.as_ref()))?;
        }
        Ok(())
    }

    /// Read and hash every collected input in repo-relative path order.
    pub fn finalize(&self) -> Result<Vec<InputFingerprint>, Error> {
        self.paths
            .iter()
            .map(|(relative, absolute)| {
                let bytes = fs::read(absolute).map_err(Error::Io)?;
                Ok(InputFingerprint {
                    path: relative.clone(),
                    sha256: specify_schema::digest::sha256_hex(&bytes),
                })
            })
            .collect()
    }
}

/// Build the lock-ready fingerprint structure from input hashes and body bytes.
pub fn for_context(
    cli_version: &str, inputs: Vec<InputFingerprint>, body: &[u8],
) -> ContextFingerprint {
    ContextFingerprint {
        fingerprint: aggregate(cli_version, inputs.clone()),
        cli_version: cli_version.to_string(),
        inputs,
        body_sha256: body_sha256(body),
    }
}

/// Hash the canonical aggregate encoding used by `.specify/context.lock`.
pub fn aggregate(cli_version: &str, mut inputs: Vec<InputFingerprint>) -> String {
    inputs.sort_by(|left, right| left.path.cmp(&right.path));

    let mut canonical = String::new();
    canonical.push_str(cli_version);
    canonical.push('\n');
    for input in inputs {
        canonical.push_str(&input.path);
        canonical.push('\t');
        canonical.push_str(&input.sha256);
        canonical.push('\n');
    }

    prefixed_sha256(canonical.as_bytes())
}

/// Hash fenced body bytes with the `sha256:` prefix used by the lock file.
pub fn body_sha256(body: &[u8]) -> String {
    prefixed_sha256(body)
}

fn prefixed_sha256(bytes: &[u8]) -> String {
    format!("sha256:{}", specify_schema::digest::sha256_hex(bytes))
}

fn repo_relative_path(project_dir: &Path, path: &Path) -> Result<String, Error> {
    let relative = path.strip_prefix(project_dir).map_err(|_err| Error::Diag {
        code: "context-fingerprint-input-outside-project",
        detail: format!(
            "context fingerprint input {} is outside project root {}",
            path.display(),
            project_dir.display()
        ),
    })?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(path: &str, sha256: &str) -> InputFingerprint {
        InputFingerprint {
            path: path.to_string(),
            sha256: sha256.to_string(),
        }
    }

    #[test]
    fn aggregate_hash_sorts_inputs() {
        let inputs = vec![
            input(
                "registry.yaml",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            ),
            input(
                ".specify/project.yaml",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
        ];

        let fingerprint = aggregate("0.2.0", inputs);

        assert_eq!(
            fingerprint,
            "sha256:96f096c433da7e43d6ab7ce7aa305882f3eb2933fa160d00640af8a0df17e73f"
        );
    }

    #[test]
    fn aggregate_hash_order_stable() {
        let alpha = input(
            ".specify/project.yaml",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        let beta = input(
            "registry.yaml",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        let gamma =
            input("Cargo.toml", "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");

        let left = aggregate("0.2.0", vec![alpha.clone(), beta.clone(), gamma.clone()]);
        let right = aggregate("0.2.0", vec![gamma, alpha, beta]);

        assert_eq!(left, right);
    }

    #[test]
    fn body_hash_changes_with_body() {
        let original = body_sha256(b"\n## Runtime\n- detected: Rust.\n\n");
        let edited = body_sha256(b"\n## Runtime\n- detected: Node.js.\n\n");

        assert_ne!(original, edited);
    }

    // The collector keys inputs by repo-relative path and dedups, then
    // `finalize` hashes content in sorted path order. A regression that
    // dropped the dedup or the sort would shuffle the canonical aggregate
    // and break lock stability across runs.
    #[test]
    fn collector_dedups_and_sorts() {
        let project = tempfile::tempdir().expect("tempdir");
        let root = project.path();
        fs::write(root.join("z.txt"), b"zed").expect("write z");
        fs::create_dir_all(root.join("sub")).expect("sub");
        fs::write(root.join("sub/a.txt"), b"aaa").expect("write a");

        let mut collector = InputCollector::new(root);
        collector.add_file(&root.join("z.txt")).expect("add z");
        collector.add_file(&root.join("sub/a.txt")).expect("add a");
        // Adding the same file again must not produce a second entry.
        collector.add_file(&root.join("z.txt")).expect("re-add z");

        let inputs = collector.finalize().expect("finalize");
        assert_eq!(
            inputs.iter().map(|i| i.path.as_str()).collect::<Vec<_>>(),
            vec!["sub/a.txt", "z.txt"]
        );
        assert_eq!(inputs[1].sha256, specify_schema::digest::sha256_hex(b"zed"));
    }

    // `add_file_if_present` is the soft variant: a missing path and a
    // directory are both silently skipped; only a real file is recorded.
    #[test]
    fn add_if_present_skips_non_files() {
        let project = tempfile::tempdir().expect("tempdir");
        let root = project.path();
        fs::create_dir_all(root.join("a-dir")).expect("dir");
        fs::write(root.join("real.txt"), b"x").expect("file");

        let mut collector = InputCollector::new(root);
        collector.add_file_if_present(&root.join("missing.txt")).expect("missing skipped");
        collector.add_file_if_present(&root.join("a-dir")).expect("dir skipped");
        collector.add_file_if_present(&root.join("real.txt")).expect("real recorded");

        let inputs = collector.finalize().expect("finalize");
        assert_eq!(inputs.iter().map(|i| i.path.as_str()).collect::<Vec<_>>(), vec!["real.txt"]);
    }

    // An input path outside the project root is a programmer error and
    // must surface as the typed `context-fingerprint-input-outside-project`
    // diagnostic rather than producing a bogus relative path.
    #[test]
    fn input_outside_project_errors() {
        let project = tempfile::tempdir().expect("project");
        let other = tempfile::tempdir().expect("other");
        fs::write(other.path().join("stray.txt"), b"x").expect("write stray");

        let mut collector = InputCollector::new(project.path());
        let err = collector.add_file(&other.path().join("stray.txt")).expect_err("outside project");
        assert!(
            matches!(
                err,
                Error::Diag {
                    code: "context-fingerprint-input-outside-project",
                    ..
                }
            ),
            "{err}"
        );
    }

    // The CLI version is the first line of the canonical aggregate, so a
    // version bump alone must change the fingerprint even when every input
    // digest is identical — otherwise an upgrade would not re-trigger
    // regeneration.
    #[test]
    fn aggregate_depends_on_cli_version() {
        let inputs = vec![input(
            ".specify/project.yaml",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )];
        assert_ne!(aggregate("0.2.0", inputs.clone()), aggregate("0.3.0", inputs));
    }

    #[test]
    fn for_context_wires_fields() {
        let inputs = vec![input(
            "registry.yaml",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )];
        let body = b"\n## Runtime\n- detected: Rust.\n\n";
        let fp = for_context("0.2.0", inputs.clone(), body);

        assert_eq!(fp.cli_version, "0.2.0");
        assert_eq!(fp.inputs, inputs.clone());
        assert_eq!(fp.fingerprint, aggregate("0.2.0", inputs));
        assert_eq!(fp.body_sha256, body_sha256(body));
    }
}
