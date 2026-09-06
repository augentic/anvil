//! Content-addressed specifications
//!
//! The store manages the content-addressed specifications of a deployment.
//! It is responsible for committing new specifications, reading the current
//! specification, and pruning old specifications.

use std::fmt::Display;

use omnia_guest::{BlobStore, CasError, Error, StateStore, server_error};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::spec;

const CONTAINER: &str = "spec";
const CURRENT: &str = "spec/current";
const SPECS: [&str; 2] = ["spec.md", "design.md"];

/// Spec generations over a deployment's storage capabilities.
#[derive(Clone, Copy, Debug)]
pub struct Store<'p, S> {
    store: &'p S,
}

impl<'p, S: StateStore + BlobStore> Store<'p, S> {
    /// Creates a generation store over `store`.
    #[must_use]
    pub const fn new(store: &'p S) -> Self {
        Self { store }
    }

    /// Commits `set` by writing its generation, swapping the pointer, and
    /// pruning its predecessor; returns the committed generation id.
    ///
    /// # Errors
    ///
    /// Fails if the observation is stale.
    /// Propagates write, swap, and prune failures.
    pub async fn commit(&self, set: &SpecSet, observed: &Observation) -> Result<String, Error> {
        let id = set.id();
        self.ensure_container().await?;
        for (name, body) in set.files() {
            BlobStore::put(self.store, CONTAINER, &object(&id, name), body.as_bytes())
                .await
                .map_err(|err| failed("committing a generation document", &err))?;
        }

        let value = format!("{id}\n");
        match StateStore::cas(self.store, CURRENT, observed.pointer.as_deref(), value.as_bytes())
            .await
        {
            Ok(()) => {}
            Err(CasError::Conflict(_)) => {
                return Err(server_error!(
                    "a concurrent `emery specify` committed first and swapped the generation \
                     pointer; re-run `emery specify` to commit against the new current generation"
                ));
            }
            Err(CasError::Store(message)) => {
                return Err(failed("swapping the generation pointer", &message));
            }
        }
        self.prune(observed, &id).await?;

        Ok(id)
    }

    /// Returns the current generation id and its complete document set,
    /// or `None` before the first commit.
    ///
    /// # Errors
    ///
    /// Fails closed for a dangling, incomplete, or unreadable
    /// generation. Propagates read failures.
    pub async fn current(&self) -> Result<Option<(String, SpecSet)>, Error> {
        let raw = StateStore::get(self.store, CURRENT)
            .await
            .map_err(|err| failed("reading the generation pointer", &err))?;
        let Some(id) = raw.as_deref().map(pointer_id) else {
            return Ok(None);
        };
        let set = self.load(&id).await?;
        Ok(Some((id, set)))
    }

    /// Observes CAS input and the outgoing set without failing.
    ///
    /// Corrupt or unreadable state suppresses only the advisory diff;
    /// the following CAS remains authoritative and fail-closed.
    pub async fn observe(&self) -> Observation {
        let pointer = StateStore::get(self.store, CURRENT).await.ok().flatten();
        let superseded = pointer.as_deref().map(pointer_id);
        let mut outgoing = None;
        if let Some(id) = &superseded {
            match self.load(id).await {
                Ok(set) => outgoing = Some(set),
                Err(err) => tracing::warn!(generation = %id, %err, "diff suppressed"),
            }
        }
        Observation {
            pointer,
            superseded,
            outgoing,
        }
    }

    // `wasi:blobstore` writes need an existing container; only some
    // backends create one on `get-container`.
    async fn ensure_container(&self) -> Result<(), Error> {
        let exists = BlobStore::container_exists(self.store, CONTAINER)
            .await
            .map_err(|err| failed("checking the generation container", &err))?;
        if !exists {
            BlobStore::create_container(self.store, CONTAINER)
                .await
                .map_err(|err| failed("creating the generation container", &err))?;
        }
        Ok(())
    }

    async fn load(&self, id: &str) -> Result<SpecSet, Error> {
        let spec = self.read(id, SPECS[0]).await?;
        let design = self.read(id, SPECS[1]).await?;
        Ok(SpecSet { spec, design })
    }

    // A named generation whose document is absent or malformed is corruption.
    async fn read(&self, id: &str, name: &str) -> Result<String, Error> {
        let bytes = BlobStore::get(self.store, CONTAINER, &object(id, name))
            .await
            .map_err(|err| failed("reading a generation document", &err))?;
        let Some(bytes) = bytes else {
            return Err(server_error!(
                "the generation pointer names `{}` but `{}` is missing; re-run `emery specify` \
                 to commit a fresh generation",
                id,
                name
            ));
        };
        String::from_utf8(bytes).map_err(|err| {
            server_error!(
                "the generation pointer names `{}` but `{}` is not UTF-8 ({}); re-run `emery \
                 specify` to commit a fresh generation",
                id,
                name,
                err
            )
        })
    }

    // Only the observed predecessor is pruned; other orphaned objects are inert.
    async fn prune(&self, observed: &Observation, keep: &str) -> Result<(), Error> {
        let Some(superseded) = &observed.superseded else {
            return Ok(());
        };
        if superseded == keep || superseded.is_empty() {
            return Ok(());
        }
        for name in SPECS {
            BlobStore::delete(self.store, CONTAINER, &object(superseded, name))
                .await
                .map_err(|err| failed("pruning the superseded generation", &err))?;
        }
        Ok(())
    }
}

/// A complete, atomically committed spec set.
///
/// Its content-derived id makes identical runs byte-stable.
#[derive(Clone, Debug)]
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
        [(SPECS[0], &self.spec), (SPECS[1], &self.design)]
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
        hex::encode(hasher.finalize())
    }
}

/// Pointer state observed before a compare-and-swap commit.
///
/// One observation drives both the CAS and advisory diff.
#[derive(Clone, Debug)]
pub struct Observation {
    // The CAS expectation, byte-exact; an unreadable pointer appears
    // absent so the subsequent CAS fails closed.
    pointer: Option<Vec<u8>>,
    // The generation the pointer names, which `commit` prunes.
    superseded: Option<String>,
    // Advisory diff input; absent when no complete set is readable.
    outgoing: Option<SpecSet>,
}

impl Observation {
    /// Returns the outgoing generation id and documents when readable.
    #[must_use]
    pub fn into_outgoing(self) -> Option<(String, SpecSet)> {
        self.superseded.zip(self.outgoing)
    }
}

/// An ephemeral re-mine diff against the superseded generation.
#[derive(Debug, Clone, Serialize)]
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
            let old = old.subjects();
            let new = new.subjects();
            for (subject, block) in &new {
                let bucket = match old.get(subject) {
                    None => &mut added,
                    Some(previous) if !previous.same_as(block) => &mut changed,
                    Some(_) => continue,
                };
                bucket.push((*subject).to_string());
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

// `{:#}` prints an `anyhow` chain in full; plain messages are unaffected.
fn failed(action: &str, err: &impl Display) -> Error {
    server_error!("storage-failed: {}: {:#}", action, err)
}

fn object(id: &str, name: &str) -> String {
    format!("generations/{id}/{name}")
}

fn pointer_id(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw).trim().to_string()
}
