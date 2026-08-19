//! The output home — the one module owning every spec-set read/write
//! (ADR-0001 Option C, ADR-0009 §2): content-addressed generations
//! behind one swapped `current` pointer; reads fail closed.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use artifacts::spec::ast;
use error::Error;
use serde::Serialize;

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

/// One re-mine diff: how an incoming spec set differs from the
/// outgoing generation it supersedes (ADR-0010).
///
/// Computed at commit time — the outgoing set is pruned immediately
/// after the swap — and emitted in the `specify` success envelope
/// only; nothing persists (ADR-0009 §2). An identical re-run yields
/// an [`empty`](Self::is_empty) diff, making "nothing changed" an
/// explicit, reviewable statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Diff {
    /// The outgoing generation id this run superseded.
    pub from: String,
    /// Spec-set file names whose bytes changed, in `FILES` order.
    pub artifacts: Vec<String>,
    /// Requirement subjects present only in the incoming `spec.md`.
    pub added: Vec<String>,
    /// Requirement subjects present only in the outgoing `spec.md`.
    pub removed: Vec<String>,
    /// Requirement subjects whose block changed (status, tag,
    /// sources, or body).
    pub changed: Vec<String>,
}

impl Diff {
    /// Diff `incoming` against the `outgoing` set committed as `from`.
    ///
    /// Section lists compare `spec.md` requirement blocks keyed by
    /// heading subject — the reconciliation join key — ignoring the
    /// positional `REQ-NNN` ids, which shift when rows are inserted
    /// or removed. The outgoing spec parsing fails only across a
    /// binary upgrade (pre-1.0: re-init); the diff is advisory, so
    /// that leaves the artifact list standing and the section lists
    /// empty rather than failing the commit (ADR-0010).
    #[must_use]
    pub fn between(from: String, outgoing: &SpecSet, incoming: &SpecSet) -> Self {
        let artifacts = outgoing
            .files()
            .iter()
            .zip(incoming.files())
            .filter(|((_, old), (_, new))| old != new)
            .map(|((name, _), _)| (*name).to_string())
            .collect();
        let (mut added, mut removed, mut changed) = (Vec::new(), Vec::new(), Vec::new());
        if let (Ok(old), Ok(new)) = (ast::parse(&outgoing.spec), ast::parse(&incoming.spec)) {
            let old = subjects(&old);
            let new = subjects(&new);
            for (subject, block) in &new {
                match old.get(subject) {
                    None => added.push((*subject).to_string()),
                    Some(previous) if !same_block(previous, block) => {
                        changed.push((*subject).to_string());
                    }
                    Some(_) => {}
                }
            }
            removed.extend(
                old.keys().filter(|subject| !new.contains_key(*subject)).map(ToString::to_string),
            );
        }
        Self {
            from,
            artifacts,
            added,
            removed,
            changed,
        }
    }

    /// No artifact or section differs — the byte-stable re-run.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
            && self.added.is_empty()
            && self.removed.is_empty()
            && self.changed.is_empty()
    }
}

/// Requirement blocks keyed by heading subject, in subject order.
fn subjects(spec: &ast::Spec) -> BTreeMap<&str, &ast::Requirement> {
    spec.requirements.iter().map(|requirement| (requirement.name.as_str(), requirement)).collect()
}

/// Block equality minus the positional `REQ-NNN` id.
fn same_block(old: &ast::Requirement, new: &ast::Requirement) -> bool {
    old.status == new.status
        && old.tag == new.tag
        && old.sources == new.sources
        && old.body == new.body
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

    /// The outgoing spec set for a re-mine diff (ADR-0010): the id
    /// the `current` pointer names and its complete set, read before
    /// the commit that will prune it.
    ///
    /// Total by design: the diff is advisory reporting, never a gate,
    /// and `specify` must stay the recovery path for a corrupt home —
    /// a missing, incomplete, or unreadable outgoing generation is
    /// `None`, not a failure. The commit itself remains the authority.
    #[must_use]
    pub fn outgoing(&self) -> Option<(String, SpecSet)> {
        let committed = self.current().ok().flatten()?;
        let read = |name: &str| fs::read_to_string(committed.dir.join(name)).ok();
        let set = SpecSet {
            bindings: read(FILES[0])?,
            receipts: read(FILES[1])?,
            spec: read(FILES[2])?,
            design: read(FILES[3])?,
        };
        Some((committed.id, set))
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
