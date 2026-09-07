//! Content-addressed specification revisions
//!
//! The store manages a deployment's specification revisions: committing a
//! new revision, reading the current one, and pruning its predecessor. A
//! revision is identified by its content digest, never a sequence number.

use anyhow::Context;
use omnia_guest::{BlobStore, Error, StateStore, server_error};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::spec;

// Blob container holding `<id>/<doc>` objects.
const CONTAINER: &str = "revisions";
pub const CURRENT: &str = "revisions/current";
const DOCS: [&str; 2] = ["spec.md", "design.md"];

/// Revisions over a deployment's storage capabilities.
#[derive(Clone, Copy, Debug)]
pub struct Store<'a, S> {
    store: &'a S,
}

impl<'a, S: StateStore + BlobStore> Store<'a, S> {
    /// Creates a revision store over `store`.
    #[must_use]
    pub const fn new(store: &'a S) -> Self {
        Self { store }
    }

    /// Commits `revision` by writing its documents, swapping the current
    /// id, and pruning its predecessor; returns the committed revision id.
    /// Fails if the observation is stale.
    pub async fn commit(
        &self, revision: &Revision, observed: &Observation,
    ) -> Result<String, Error> {
        self.ensure_container().await?;
        let id = revision.id();

        for (name, body) in revision.files() {
            BlobStore::put(self.store, CONTAINER, &format!("{id}/{name}"), body.as_bytes())
                .await
                .context("committing a revision document")?;
        }

        let expected = observed.current.as_deref().map(str::as_bytes);
        StateStore::cas(self.store, CURRENT, expected, id.as_bytes())
            .await
            .context("swapping the current revision id")?;

        self.prune(observed, &id).await?;

        Ok(id)
    }

    /// Returns the current revision id and its complete document set, or
    /// `None` before the first commit. Fails closed for a dangling,
    /// incomplete, or unreadable revision.
    pub async fn current(&self) -> Result<Option<(String, Revision)>, Error> {
        let raw = StateStore::get(self.store, CURRENT)
            .await
            .context("reading the current revision id")?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let id = String::from_utf8(raw).map_err(|err| {
            server_error!(
                "the current revision id is not UTF-8 ({}); re-run `emery specify` to commit a \
                 fresh revision",
                err
            )
        })?;
        let revision = self.load(&id).await?;
        Ok(Some((id, revision)))
    }

    /// Observes CAS input and the outgoing revision without failing.
    ///
    /// Corrupt or unreadable state suppresses only the advisory diff;
    /// the following CAS remains authoritative and fail-closed.
    pub async fn observe(&self) -> Observation {
        let current = StateStore::get(self.store, CURRENT)
            .await
            .ok()
            .flatten()
            .and_then(|raw| String::from_utf8(raw).ok());
        let mut outgoing = None;

        if let Some(id) = &current {
            match self.load(id).await {
                Ok(revision) => outgoing = Some(revision),
                Err(err) => tracing::warn!(revision = %id, %err, "diff suppressed"),
            }
        }

        Observation { current, outgoing }
    }

    // `wasi:blobstore` writes need an existing container; only some
    // backends create one on `get-container`.
    async fn ensure_container(&self) -> Result<(), Error> {
        if !BlobStore::container_exists(self.store, CONTAINER)
            .await
            .context("checking the revision container")?
        {
            BlobStore::create_container(self.store, CONTAINER)
                .await
                .context("creating the revision container")?;
        }

        Ok(())
    }

    async fn load(&self, id: &str) -> Result<Revision, Error> {
        let spec = self.read(id, DOCS[0]).await?;
        let design = self.read(id, DOCS[1]).await?;
        Ok(Revision { spec, design })
    }

    // A named revision whose document is absent or malformed is corruption.
    async fn read(&self, id: &str, name: &str) -> Result<String, Error> {
        let bytes = BlobStore::get(self.store, CONTAINER, &format!("{id}/{name}"))
            .await
            .context("reading a revision document")?
            .ok_or_else(|| {
                server_error!(
                    "the current revision id names `{}` but `{}` is missing; re-run `emery \
                     specify` to commit a fresh revision",
                    id,
                    name
                )
            })?;
        String::from_utf8(bytes).map_err(|err| {
            server_error!(
                "the current revision id names `{}` but `{}` is not UTF-8 ({}); re-run `emery \
                 specify` to commit a fresh revision",
                id,
                name,
                err
            )
        })
    }

    // Only the observed predecessor is pruned; other orphaned objects are inert.
    async fn prune(&self, observed: &Observation, keep: &str) -> Result<(), Error> {
        let Some(superseded) = &observed.current else {
            return Ok(());
        };
        if superseded == keep {
            return Ok(());
        }
        for name in DOCS {
            BlobStore::delete(self.store, CONTAINER, &format!("{superseded}/{name}"))
                .await
                .context("pruning the superseded revision")?;
        }
        Ok(())
    }
}

/// A complete, atomically committed specification revision.
///
/// Its content-derived id makes identical runs byte-stable.
#[derive(Clone, Debug)]
pub struct Revision {
    /// The behavioural specification document.
    pub spec: String,
    /// The rebuild design document.
    pub design: String,
}

impl Revision {
    // Documents in digest order.
    fn files(&self) -> [(&'static str, &str); 2] {
        [(DOCS[0], &self.spec), (DOCS[1], &self.design)]
    }

    // SHA-256 over length-prefixed names and bodies.
    fn id(&self) -> String {
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

/// The current revision observed before a compare-and-swap commit.
///
/// One observation drives both the CAS and advisory diff.
#[derive(Clone, Debug)]
pub struct Observation {
    // The current revision id, which `commit` expects and then prunes.
    // Absent before the first commit; an unreadable or non-UTF-8 value
    // also reads as absent so the subsequent CAS fails closed.
    current: Option<String>,
    // Advisory diff input; absent when no complete revision is readable.
    outgoing: Option<Revision>,
}

impl Observation {
    pub fn into_outgoing(self) -> Option<(String, Revision)> {
        self.current.zip(self.outgoing)
    }
}

/// An ephemeral re-mine diff against the superseded revision.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Diff {
    /// The outgoing revision id this run superseded.
    pub from: String,
    /// Changed file names in digest order.
    pub artifacts: Vec<String>,
    /// Requirement subjects present only in the incoming `spec.md`.
    pub added: Vec<String>,
    /// Requirement subjects present only in the outgoing `spec.md`.
    pub removed: Vec<String>,
    /// Requirement subjects whose blocks changed.
    pub changed: Vec<String>,
}

impl Diff {
    // Sections key on heading subjects, not positional ids. Because the
    // diff is advisory, an unparseable old spec leaves section lists empty.
    pub(crate) fn between(from: String, outgoing: &Revision, incoming: &Revision) -> Self {
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

// Keep (entry-point-unreachable): two runs racing one current id cannot
// be arranged through the CLI, which observes and commits inside a single
// `specify`. Everything else the store does is owned by the root scenarios.
#[cfg(test)]
mod tests {
    use omnia_test::guest::Memory;

    use super::{Revision, Store};

    #[tokio::test]
    async fn concurrent_commit_conflicts() {
        let memory = Memory::default();
        let store = Store::new(&memory);

        // Both runs observe the empty store; the winner swaps first.
        let stale = store.observe().await;
        let observed = store.observe().await;
        let winner = store.commit(&revision("# Spec winner\n"), &observed).await.expect("commit");

        let err = store
            .commit(&revision("# Spec loser\n"), &stale)
            .await
            .expect_err("a stale observation must never last-write-wins over the swapped id");
        assert_eq!(err.code(), "server_error", "typed failure");
        assert!(
            err.description().contains("swapping the current revision id"),
            "typed failure: {}",
            err.description()
        );
        let (current, _) = store.current().await.expect("current").expect("committed");
        assert_eq!(current, winner, "the current id still names the winner");
        let spec =
            memory.object("revisions", &format!("{winner}/spec.md")).expect("winning spec");
        assert_eq!(spec, b"# Spec winner\n", "the winning revision is intact");
    }

    fn revision(spec: &str) -> Revision {
        Revision {
            spec: spec.to_string(),
            design: "# Design\n".to_string(),
        }
    }
}
