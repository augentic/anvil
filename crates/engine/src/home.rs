//! The output home — the one module owning every spec-set read/write
//! (ADR-0001 Option C, ADR-0009 §2): content-addressed generations
//! behind one swapped `current` pointer; reads fail closed.

use std::fs;
use std::path::{Path, PathBuf};

use error::Error;

/// The output-home directory under `.emery/`.
const SPEC_DIR: &str = "spec";

/// The generation-pointer document at the output-home root.
const CURRENT_FILE: &str = "current";

/// The generation directories' parent under the output home.
const GENERATIONS_DIR: &str = "generations";

/// Every document of one complete generation, in the fixed on-disk
/// order the generation digest folds them.
const FILES: [&str; 4] = ["bindings.yaml", "receipts.yaml", "spec.md", "design.md"];

/// One complete spec set, assembled in memory before any write.
///
/// The resolved-bindings snapshot, the extract receipts, and the two
/// reviewable documents commit as a unit or not at all. Because the
/// generation id is the digest of the set's bytes, an identical
/// re-run converges on the same directory and the home stays
/// byte-stable. No document carries a timestamp or log line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecSet {
    /// Canonical YAML of the bindings this run resolved.
    pub bindings: String,
    /// Canonical YAML of the per-source extract receipts.
    pub receipts: String,
    /// The behavioural specification document.
    pub spec: String,
    /// The rebuild design document.
    pub design: String,
}

impl SpecSet {
    /// The set's documents as `(file name, body)` pairs, in `FILES`
    /// order.
    #[must_use]
    pub fn files(&self) -> [(&'static str, &str); 4] {
        [
            (FILES[0], &self.bindings),
            (FILES[1], &self.receipts),
            (FILES[2], &self.spec),
            (FILES[3], &self.design),
        ]
    }

    /// The content-addressed generation id: the SHA-256 digest over
    /// every document name and body, length-prefixed so the encoding
    /// is unambiguous.
    #[must_use]
    pub fn id(&self) -> String {
        let mut hasher = diagnostics::digest::Hasher::new();
        for (name, body) in self.files() {
            hasher.update(&(name.len() as u64).to_be_bytes());
            hasher.update(name.as_bytes());
            hasher.update(&(body.len() as u64).to_be_bytes());
            hasher.update(body.as_bytes());
        }
        hasher.finalize_hex()
    }
}

/// A committed generation: the pointer-named id and its directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Committed {
    /// The generation id the `current` pointer names.
    pub id: String,
    /// The generation directory carrying the complete spec set.
    pub dir: PathBuf,
}

/// The output home rooted at one project's `.emery/spec/`.
#[derive(Clone, Debug)]
pub struct Home {
    root: PathBuf,
}

impl Home {
    /// The output home under `project_dir`'s `.emery/` tree.
    #[must_use]
    pub fn new(project_dir: &Path) -> Self {
        Self {
            root: project_dir.join(".emery").join(SPEC_DIR),
        }
    }

    /// Commit `set` as the current generation: write the complete
    /// generation directory, atomically swap the `current` pointer to
    /// it, then prune everything the pointer no longer names (crash
    /// litter from an interrupted earlier run included). A crash
    /// before the swap leaves the previous set intact and current.
    ///
    /// # Errors
    ///
    /// Propagates filesystem failures from the writes, the swap, or
    /// the prune.
    pub fn commit(&self, set: &SpecSet) -> Result<Committed, Error> {
        let id = set.id();
        let dir = self.root.join(GENERATIONS_DIR).join(&id);
        for (name, body) in set.files() {
            artifacts::atomic::bytes_write(&dir.join(name), body.as_bytes())?;
        }
        artifacts::atomic::bytes_write(
            &self.root.join(CURRENT_FILE),
            format!("{id}\n").as_bytes(),
        )?;
        self.prune(&id)?;
        Ok(Committed { id, dir })
    }

    /// The committed generation the `current` pointer names, or `None`
    /// when no generation has ever been committed (no pointer exists).
    ///
    /// # Errors
    ///
    /// Fails closed with `spec-home-corrupt` when the pointer exists
    /// but names a missing or incomplete generation, and propagates
    /// read failures. Corruption is never an empty result.
    pub fn current(&self) -> Result<Option<Committed>, Error> {
        let path = self.root.join(CURRENT_FILE);
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(Error::Io(err)),
        };
        let id = raw.trim().to_string();
        let dir = self.root.join(GENERATIONS_DIR).join(&id);
        for name in FILES {
            let document = dir.join(name);
            if !document.is_file() {
                return Err(Error::Diag {
                    code: "spec-home-corrupt",
                    detail: format!(
                        "the generation pointer names `{id}` but `{}` is missing; re-run `emery \
                         specify` to commit a fresh generation",
                        document.display()
                    ),
                });
            }
        }
        Ok(Some(Committed { id, dir }))
    }

    /// Keep only the `current` pointer and the generation it names:
    /// remove every other entry at the home root and under
    /// `generations/` — superseded generations and any temp-file or
    /// partial-directory litter a crash left behind.
    fn prune(&self, keep: &str) -> Result<(), Error> {
        for entry in fs::read_dir(&self.root)? {
            let path = entry?.path();
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
            if path.is_dir() {
                if name != GENERATIONS_DIR {
                    fs::remove_dir_all(&path)?;
                }
            } else if name != CURRENT_FILE {
                fs::remove_file(&path)?;
            }
        }
        for entry in fs::read_dir(self.root.join(GENERATIONS_DIR))? {
            let path = entry?.path();
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
            if name != keep {
                if path.is_dir() { fs::remove_dir_all(&path)? } else { fs::remove_file(&path)? }
            }
        }
        Ok(())
    }
}
