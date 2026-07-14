//! Deterministic fingerprinting for generated context inputs.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use error::Error;

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
    #[must_use]
    pub fn new(project_dir: &Path) -> Self {
        Self {
            project_dir: project_dir.to_path_buf(),
            paths: BTreeMap::new(),
        }
    }

    /// Add a required input file.
    ///
    /// # Errors
    ///
    /// `context-fingerprint-input-outside-project` when `path` does not
    /// live under the project root.
    pub fn add_file(&mut self, path: &Path) -> Result<(), Error> {
        let relative = repo_relative_path(&self.project_dir, path)?;
        self.paths.entry(relative).or_insert_with(|| path.to_path_buf());
        Ok(())
    }

    /// Record an input file under an explicit logical repo-relative
    /// path, decoupling the recorded provenance from the file's physical
    /// location. Used for out-of-tree cache inputs (the manifest
    /// mirror) so context fingerprints stay stable and project-relative
    /// even though the bytes live in the OS cache.
    pub fn add_file_as(&mut self, logical_relative: &str, physical: &Path) {
        self.paths.entry(logical_relative.to_string()).or_insert_with(|| physical.to_path_buf());
    }

    /// Add an input file only when it exists as a regular file.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] when existence cannot be determined, plus the
    /// [`Self::add_file`] errors.
    pub fn add_file_if_present(&mut self, path: &Path) -> Result<(), Error> {
        match path.try_exists() {
            Ok(true) if path.is_file() => self.add_file(path),
            Ok(_) => Ok(()),
            Err(err) => Err(Error::Io(err)),
        }
    }

    /// Add repo-relative input paths captured by another renderer component.
    ///
    /// # Errors
    ///
    /// Propagates the [`Self::add_file`] errors.
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
    ///
    /// # Errors
    ///
    /// [`Error::Io`] when a collected input cannot be read.
    pub fn finalize(&self) -> Result<Vec<InputFingerprint>, Error> {
        self.paths
            .iter()
            .map(|(relative, absolute)| {
                let bytes = fs::read(absolute).map_err(Error::Io)?;
                Ok(InputFingerprint {
                    path: relative.clone(),
                    sha256: diagnostics::digest::sha256_hex(&bytes),
                })
            })
            .collect()
    }
}

/// Build the lock-ready fingerprint structure from input hashes and body bytes.
#[must_use]
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
#[must_use]
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
#[must_use]
pub fn body_sha256(body: &[u8]) -> String {
    prefixed_sha256(body)
}

fn prefixed_sha256(bytes: &[u8]) -> String {
    format!("sha256:{}", diagnostics::digest::sha256_hex(bytes))
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
