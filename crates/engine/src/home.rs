//! Content-addressed spec generations behind a swapped `current` pointer.

use std::collections::BTreeMap;

use omnia_guest::{BlobStore, CasError, Error, StateStore, server_error};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{spec, storage};

/// Blobstore container for spec generations.
pub const SPEC_CONTAINER: &str = "spec";

/// Keyvalue entry naming the current generation.
pub const CURRENT_KEY: &str = "spec/current";

const GENERATIONS_DIR: &str = "generations";

// Digest order is part of the generation identity.
const FILES: [&str; 2] = ["spec.md", "design.md"];

fn object(id: &str, name: &str) -> String {
    format!("{GENERATIONS_DIR}/{id}/{name}")
}

/// A complete, atomically committed spec set.
///
/// Its content-derived id makes identical runs byte-stable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecSet {
    /// The behavioural specification document.
    pub spec: String,
    /// The rebuild design document.
    pub design: String,
}

impl SpecSet {
    /// Returns documents in generation-digest order.
    #[must_use]
    pub fn files(&self) -> [(&'static str, &str); 2] {
        [(FILES[0], &self.spec), (FILES[1], &self.design)]
    }

    /// Returns the SHA-256 generation id over length-prefixed names and bodies.
    #[must_use]
    pub fn id(&self) -> String {
        let mut hasher = Sha256::new();
        for (name, body) in self.files() {
            hasher.update((name.len() as u64).to_be_bytes());
            hasher.update(name.as_bytes());
            hasher.update((body.len() as u64).to_be_bytes());
            hasher.update(body.as_bytes());
        }
        hex_lower(&hasher.finalize())
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for &byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

/// A generation named by the current pointer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Committed {
    /// Current generation id.
    pub id: String,
}

/// Pointer state observed before a compare-and-swap commit.
///
/// One observation drives both the CAS and advisory diff.
#[derive(Clone, Debug)]
pub struct Observation {
    // Unreadable pointers appear absent so the subsequent CAS fails closed.
    pointer: Option<Vec<u8>>,
    // Advisory diff input; absent when no complete set is readable.
    outgoing: Option<(String, SpecSet)>,
}

impl Observation {
    /// Returns the complete outgoing generation when readable.
    #[must_use]
    pub fn into_outgoing(self) -> Option<(String, SpecSet)> {
        self.outgoing
    }
}

/// An ephemeral re-mine diff against the superseded generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Diff {
    /// The outgoing generation id this run superseded.
    pub from: String,
    /// Changed file names in generation-digest order.
    pub artifacts: Vec<String>,
    /// Requirement subjects present only in the incoming `spec.md`.
    pub added: Vec<String>,
    /// Requirement subjects present only in the outgoing `spec.md`.
    pub removed: Vec<String>,
    /// Requirement subjects whose blocks changed.
    pub changed: Vec<String>,
}

impl Diff {
    /// Diffs `incoming` against `outgoing`, identified by `from`.
    ///
    /// Sections use heading subjects, not positional ids. Because the
    /// diff is advisory, an unparseable old spec leaves section lists empty.
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
        if let (Ok(old), Ok(new)) = (spec::parse(&outgoing.spec), spec::parse(&incoming.spec)) {
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

    /// Returns whether no artifact or section differs.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
            && self.added.is_empty()
            && self.removed.is_empty()
            && self.changed.is_empty()
    }
}

fn subjects(spec: &spec::Spec) -> BTreeMap<&str, &spec::Requirement> {
    spec.requirements.iter().map(|requirement| (requirement.name.as_str(), requirement)).collect()
}

// Positional ids do not define requirement identity.
fn same_block(old: &spec::Requirement, new: &spec::Requirement) -> bool {
    old.status == new.status
        && old.tag == new.tag
        && old.sources == new.sources
        && old.body == new.body
}

/// Spec generations over a deployment's storage capabilities.
#[derive(Clone, Copy, Debug)]
pub struct Home<'p, S> {
    store: &'p S,
}

impl<'p, S: StateStore + BlobStore> Home<'p, S> {
    /// Creates an output home over `store`.
    #[must_use]
    pub const fn new(store: &'p S) -> Self {
        Self { store }
    }

    /// Commits `set` by writing its generation, swapping the pointer, and pruning its predecessor.
    ///
    /// # Errors
    ///
    /// Fails if the observation is stale.
    /// Propagates write, swap, and prune failures.
    pub async fn commit(&self, set: &SpecSet, observed: &Observation) -> Result<Committed, Error> {
        let id = set.id();
        for (name, body) in set.files() {
            self.store
                .put(SPEC_CONTAINER, &object(&id, name), body.as_bytes())
                .await
                .map_err(|err| storage::failed("committing a generation document", &err))?;
        }
        let value = format!("{id}\n");
        match self.store.cas(CURRENT_KEY, observed.pointer.as_deref(), value.as_bytes()).await {
            Ok(()) => {}
            Err(CasError::Conflict(_)) => {
                return Err(server_error!(
                    "a concurrent `emery specify` committed first and swapped the generation \
                     pointer; re-run `emery specify` to commit against the new current generation"
                ));
            }
            Err(CasError::Store(message)) => {
                return Err(storage::failed(
                    "swapping the generation pointer",
                    &anyhow::anyhow!(message),
                ));
            }
        }
        self.prune(observed, &id).await?;
        Ok(Committed { id })
    }

    /// Returns the current generation, or `None` before the first commit.
    ///
    /// # Errors
    ///
    /// Fails closed for a dangling or incomplete generation.
    /// Propagates read failures.
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
                return Err(server_error!(
                    "the generation pointer names `{id}` but `{name}` is missing; re-run \
                     `emery specify` to commit a fresh generation",
                ));
            }
        }
        Ok(Some(Committed { id }))
    }

    /// Returns the current generation and its complete document set,
    /// or `None` before the first commit.
    ///
    /// # Errors
    ///
    /// Fails closed for a dangling, incomplete, or unreadable
    /// generation. Propagates read failures.
    pub async fn current_set(&self) -> Result<Option<(Committed, SpecSet)>, Error> {
        let Some(committed) = self.current().await? else {
            return Ok(None);
        };
        let Some((_, set)) = self.load(&committed.id).await else {
            return Err(server_error!(
                "the generation pointer names `{}` but its documents cannot be read; re-run \
                 `emery specify` to commit a fresh generation",
                committed.id
            ));
        };
        Ok(Some((committed, set)))
    }

    /// Observes CAS input and the outgoing set without failing.
    ///
    /// Corrupt or unreadable state suppresses only the advisory diff;
    /// the following CAS remains authoritative and fail-closed.
    pub async fn observe(&self) -> Observation {
        let pointer = StateStore::get(self.store, CURRENT_KEY).await.ok().flatten();
        let outgoing = match &pointer {
            Some(raw) => self.load(String::from_utf8_lossy(raw).trim()).await,
            None => None,
        };
        Observation { pointer, outgoing }
    }

    // Advisory reads collapse incomplete or unreadable generations to `None`.
    async fn load(&self, id: &str) -> Option<(String, SpecSet)> {
        let mut bodies = Vec::with_capacity(FILES.len());
        for name in FILES {
            let bytes = BlobStore::get(self.store, SPEC_CONTAINER, &object(id, name))
                .await
                .ok()
                .flatten()?;
            bodies.push(String::from_utf8(bytes).ok()?);
        }
        let design = bodies.pop()?;
        let spec = bodies.pop()?;
        Some((id.to_string(), SpecSet { spec, design }))
    }

    // Only the observed predecessor is pruned; other orphaned objects are inert.
    async fn prune(&self, observed: &Observation, keep: &str) -> Result<(), Error> {
        let Some(raw) = &observed.pointer else {
            return Ok(());
        };
        let superseded = String::from_utf8_lossy(raw).trim().to_string();
        if superseded == keep || superseded.is_empty() {
            return Ok(());
        }
        for name in FILES {
            BlobStore::delete(self.store, SPEC_CONTAINER, &object(&superseded, name))
                .await
                .map_err(|err| storage::failed("pruning the superseded generation", &err))?;
        }
        Ok(())
    }
}
