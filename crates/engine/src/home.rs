//! The output home — the one module owning every spec-set read/write:
//! content-addressed generations behind one swapped `current` pointer,
//! over the deployment's storage capabilities; reads fail closed.

use std::collections::BTreeMap;

use emery_artifacts::spec::ast;
use emery_error::Error;
use omnia_guest::{BlobStore, StateStore};
use serde::Serialize;

use crate::storage;

/// The blobstore container carrying the output home: generation
/// documents under `generations/<id>/`.
pub const SPEC_CONTAINER: &str = "spec";

/// The keyvalue entry naming the current generation id.
pub const CURRENT_KEY: &str = "spec/current";

// The generation objects' parent inside the spec container.
const GENERATIONS_DIR: &str = "generations";

// Every document of one complete generation, in the fixed order the
// generation digest folds them.
const FILES: [&str; 2] = ["spec.md", "design.md"];

// The spec-container object name of one generation document.
fn object(id: &str, name: &str) -> String {
    format!("{GENERATIONS_DIR}/{id}/{name}")
}

/// One complete spec set, assembled in memory before any write.
///
/// The two reviewable documents commit as a unit or not at all.
/// Because the generation id is the digest of the set's bytes, an
/// identical re-run converges on the same objects and the home
/// stays byte-stable. No document carries a timestamp or log line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecSet {
    /// The behavioural specification document.
    pub spec: String,
    /// The rebuild design document.
    pub design: String,
}

impl SpecSet {
    /// The set's documents as `(file name, body)` pairs, in `FILES`
    /// order.
    #[must_use]
    pub fn files(&self) -> [(&'static str, &str); 2] {
        [(FILES[0], &self.spec), (FILES[1], &self.design)]
    }

    /// The content-addressed generation id: the SHA-256 digest over
    /// every document name and body, length-prefixed so the encoding
    /// is unambiguous.
    #[must_use]
    pub fn id(&self) -> String {
        let mut hasher = emery_diagnostics::digest::Hasher::new();
        for (name, body) in self.files() {
            hasher.update(&(name.len() as u64).to_be_bytes());
            hasher.update(name.as_bytes());
            hasher.update(&(body.len() as u64).to_be_bytes());
            hasher.update(body.as_bytes());
        }
        hasher.finalize_hex()
    }
}

/// A committed generation: the pointer-named id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Committed {
    /// The generation id the `current` pointer names.
    pub id: String,
}

/// One re-mine diff: how an incoming spec set differs from the
/// outgoing generation it supersedes.
///
/// Computed at commit time — the outgoing set is pruned immediately
/// after the swap — and emitted in the `specify` success envelope
/// only; nothing persists. An identical re-run yields
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
    /// empty rather than failing the commit.
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

// Requirement blocks keyed by heading subject, in subject order.
fn subjects(spec: &ast::Spec) -> BTreeMap<&str, &ast::Requirement> {
    spec.requirements.iter().map(|requirement| (requirement.name.as_str(), requirement)).collect()
}

// Block equality minus the positional `REQ-NNN` id.
fn same_block(old: &ast::Requirement, new: &ast::Requirement) -> bool {
    old.status == new.status
        && old.tag == new.tag
        && old.sources == new.sources
        && old.body == new.body
}

/// The output home over one deployment's storage capabilities.
#[derive(Clone, Copy, Debug)]
pub struct Home<'p, S> {
    store: &'p S,
}

impl<'p, S: StateStore + BlobStore> Home<'p, S> {
    /// The output home over `store`.
    #[must_use]
    pub const fn new(store: &'p S) -> Self {
        Self { store }
    }

    /// Commit `set` as the current generation: write the complete
    /// generation objects, swap the `current` pointer to them, then
    /// prune everything the pointer no longer names (crash litter
    /// from an interrupted earlier run included). A crash before the
    /// swap leaves the previous set intact and current.
    ///
    /// # Errors
    ///
    /// Propagates storage failures from the writes, the swap, or the
    /// prune.
    pub async fn commit(&self, set: &SpecSet) -> Result<Committed, Error> {
        let id = set.id();
        for (name, body) in set.files() {
            self.store
                .put(SPEC_CONTAINER, &object(&id, name), body.as_bytes())
                .await
                .map_err(|err| storage::failed("committing a generation document", &err))?;
        }
        self.store
            .set(CURRENT_KEY, format!("{id}\n").as_bytes(), None)
            .await
            .map_err(|err| storage::failed("swapping the generation pointer", &err))?;
        self.prune(&id).await?;
        Ok(Committed { id })
    }

    /// The committed generation the `current` pointer names, or `None`
    /// when no generation has ever been committed (no pointer exists).
    ///
    /// # Errors
    ///
    /// Fails closed with `spec-home-corrupt` when the pointer exists
    /// but names a missing or incomplete generation, and propagates
    /// read failures. Corruption is never an empty result.
    pub async fn current(&self) -> Result<Option<Committed>, Error> {
        let raw = StateStore::get(self.store, CURRENT_KEY)
            .await
            .map_err(|err| storage::failed("reading the generation pointer", &err))?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let id = String::from_utf8_lossy(&raw).trim().to_string();
        for name in FILES {
            let present = self
                .store
                .has(SPEC_CONTAINER, &object(&id, name))
                .await
                .map_err(|err| storage::failed("probing a generation document", &err))?;
            if !present {
                return Err(Error::Diag {
                    code: "spec-home-corrupt",
                    detail: format!(
                        "the generation pointer names `{id}` but `{name}` is missing; re-run \
                         `emery specify` to commit a fresh generation"
                    ),
                });
            }
        }
        Ok(Some(Committed { id }))
    }

    /// The outgoing spec set for a re-mine diff: the id the `current`
    /// pointer names and its complete set, read before the commit
    /// that will prune it.
    ///
    /// Total by design: the diff is advisory reporting, never a gate,
    /// and `specify` must stay the recovery path for a corrupt home —
    /// a missing, incomplete, or unreadable outgoing generation is
    /// `None`, not a failure. The commit itself remains the authority.
    pub async fn outgoing(&self) -> Option<(String, SpecSet)> {
        let committed = self.current().await.ok().flatten()?;
        let mut bodies = Vec::with_capacity(FILES.len());
        for name in FILES {
            let bytes = BlobStore::get(self.store, SPEC_CONTAINER, &object(&committed.id, name))
                .await
                .ok()
                .flatten()?;
            bodies.push(String::from_utf8(bytes).ok()?);
        }
        let design = bodies.pop()?;
        let spec = bodies.pop()?;
        Some((committed.id, SpecSet { spec, design }))
    }

    // Keep only the generation the pointer names — superseded
    // generations and any partial-generation litter a crash left
    // behind are removed. The pointer itself is a keyvalue entry,
    // never an object in the container.
    async fn prune(&self, keep: &str) -> Result<(), Error> {
        let kept = format!("{GENERATIONS_DIR}/{keep}/");
        let names = self
            .store
            .list(SPEC_CONTAINER)
            .await
            .map_err(|err| storage::failed("listing the output home", &err))?;
        for name in names {
            if name.starts_with(&kept) {
                continue;
            }
            BlobStore::delete(self.store, SPEC_CONTAINER, &name)
                .await
                .map_err(|err| storage::failed("pruning a superseded generation", &err))?;
        }
        Ok(())
    }
}
