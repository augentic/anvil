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

/// Keyvalue key holding the current revision id.
pub const CURRENT: &str = "current-revision";

/// Blobstore container holding every revision's documents under `<id>/`.
pub const CONTAINER: &str = "revisions";

const SPEC: &str = "spec.md";
const DESIGN: &str = "design.md";

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
    ///
    /// Fails if `observed` is stale or storage refuses the write. A lost
    /// swap leaves the written documents as an inert, unreferenced orphan.
    pub async fn commit(
        &self, revision: &Revision, observed: Observation,
    ) -> Result<String, Error> {
        if !BlobStore::container_exists(self.store, CONTAINER).await? {
            BlobStore::create_container(self.store, CONTAINER).await?;
        }

        let id = revision.id();
        for (name, body) in revision.files() {
            BlobStore::put(self.store, CONTAINER, &format!("{id}/{name}"), body.as_bytes())
                .await
                .context("writing revision document")?;
        }

        StateStore::cas(self.store, CURRENT, observed.token.as_deref(), id.as_bytes())
            .await
            .context("swapping current revision")?;

        // the swap landed, prune the previous revision
        if let Some(previous) = observed.previous().filter(|previous| *previous != id) {
            for (name, _) in revision.files() {
                let _ =
                    BlobStore::delete(self.store, CONTAINER, &format!("{previous}/{name}")).await;
            }
        }

        Ok(id)
    }

    /// Returns the current revision and its id, or `None` before the
    /// first commit. Fails closed for a dangling, incomplete, unreadable,
    /// or tampered revision.
    pub async fn current(&self) -> Result<Option<Committed>, Error> {
        let Some(raw) =
            StateStore::get(self.store, CURRENT).await.context("getting current revision id")?
        else {
            return Ok(None);
        };

        let id = String::from_utf8(raw).context("decoding current revision id")?;
        let revision = self.load(&id).await?;

        Ok(Some(Committed { id, revision }))
    }

    /// Observes the CAS token and the outgoing revision without failing.
    ///
    /// Corrupt or unreadable state suppresses only the advisory diff;
    /// the following CAS remains authoritative and fail-closed.
    pub async fn observe(&self) -> Observation {
        let token = StateStore::get(self.store, CURRENT).await.ok().flatten();

        let outgoing = if let Some(id) = token.as_deref().and_then(|raw| str::from_utf8(raw).ok()) {
            self.load(id).await.ok().map(|revision| Committed {
                id: id.to_string(),
                revision,
            })
        } else {
            None
        };

        Observation { token, outgoing }
    }

    // The store is content-addressed: documents that no longer hash to
    // the id they sit under are corruption, not a revision.
    async fn load(&self, id: &str) -> Result<Revision, Error> {
        let spec = self.read(id, SPEC).await?;
        let design = self.read(id, DESIGN).await?;
        let revision = Revision { spec, design };
        if revision.id() != id {
            return Err(server_error!("revision `{id}` does not match its content"));
        }
        Ok(revision)
    }

    // A named revision whose document is absent or malformed is corruption.
    async fn read(&self, id: &str, name: &str) -> Result<String, Error> {
        let bytes = BlobStore::get(self.store, CONTAINER, &format!("{id}/{name}"))
            .await
            .context("reading revision document")?
            .ok_or_else(|| server_error!("revision `{id}` does not contain `{name}`"))?;
        let body = String::from_utf8(bytes)
            .with_context(|| format!("revision `{id}` contains `{name}` but it is not UTF-8"))?;
        Ok(body)
    }
}

/// A complete, atomically committed specification revision.
///
/// Its content-derived id makes identical runs byte-stable.
#[derive(Debug)]
pub struct Revision {
    /// The behavioural specification document.
    pub spec: String,
    /// The rebuild design document.
    pub design: String,
}

impl Revision {
    // The one place a document name meets its field; digest order.
    fn files(&self) -> [(&'static str, &str); 2] {
        [(SPEC, &self.spec), (DESIGN, &self.design)]
    }

    // SHA-256 over the domain tag, then length-prefixed names and bodies.
    fn id(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"emery-revision/1");
        for (name, body) in self.files() {
            hasher.update((name.len() as u64).to_be_bytes());
            hasher.update(name.as_bytes());
            hasher.update((body.len() as u64).to_be_bytes());
            hasher.update(body.as_bytes());
        }
        hex::encode(hasher.finalize())
    }
}

/// A revision read back under the id storage names it by.
///
/// The id is the stored compare-and-swap token, kept beside the
/// documents rather than derived from them so a reader sees what
/// storage said; `Store::load` has already checked the two agree.
#[derive(Debug)]
pub struct Committed {
    /// The stored revision id.
    pub id: String,
    /// The revision's documents.
    pub revision: Revision,
}

/// The current revision observed before a compare-and-swap commit.
///
/// One observation drives one CAS and its advisory diff.
#[derive(Debug)]
pub struct Observation {
    // The raw CAS token exactly as storage holds it. Absent before the
    // first commit; also absent when storage could not be read, so the
    // subsequent CAS fails closed against a present key.
    token: Option<Vec<u8>>,
    // Advisory diff input; absent when no complete revision is readable.
    outgoing: Option<Committed>,
}

impl Observation {
    /// The complete outgoing revision, when one was readable.
    #[must_use]
    pub const fn outgoing(&self) -> Option<&Committed> {
        self.outgoing.as_ref()
    }

    // The predecessor the token names; a non-UTF-8 token names no blobs.
    fn previous(&self) -> Option<&str> {
        self.token.as_deref().and_then(|raw| str::from_utf8(raw).ok())
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
    // Sections key on heading subjects, not positional ids. The diff is
    // advisory: an outgoing spec that fails the grammar leaves the section
    // lists empty, and the incoming spec was already parsed by synthesis.
    pub(crate) fn between(outgoing: &Committed, incoming: &Revision) -> Self {
        let artifacts = outgoing
            .revision
            .files()
            .iter()
            .zip(incoming.files())
            .filter(|((_, old), (_, new))| old != new)
            .map(|((name, _), _)| (*name).to_string())
            .collect();

        let (mut added, mut removed, mut changed) = (Vec::new(), Vec::new(), Vec::new());
        if let (Ok(old), Ok(new)) =
            (spec::parse(&outgoing.revision.spec), spec::parse(&incoming.spec))
        {
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
            from: outgoing.id.clone(),
            artifacts,
            added,
            removed,
            changed,
        }
    }

    /// Returns whether the revisions are byte-identical; identical bytes
    /// cannot yield section differences.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

// Keep (entry-point-unreachable): two runs racing one current id cannot
// be arranged through the CLI, which observes and commits inside a single
// `specify`. Everything else the store does is owned by the root scenarios.
#[cfg(test)]
mod tests {
    use omnia_test::guest::Memory;

    use super::{CONTAINER, Revision, Store};

    #[tokio::test]
    async fn concurrent_commit_conflicts() {
        let memory = Memory::default();
        let store = Store::new(&memory);

        // Both runs observe the empty store; the winner swaps first.
        let stale = store.observe().await;
        let observed = store.observe().await;
        let winner = store.commit(&revision("# Spec winner\n"), observed).await.expect("commit");

        let err = store
            .commit(&revision("# Spec loser\n"), stale)
            .await
            .expect_err("a stale observation must never last-write-wins over the swapped id");
        assert_eq!(err.code(), "server_error", "typed failure");
        assert!(
            err.description().contains(&format!("lost the swap to `{winner}`")),
            "the failure names the winner: {}",
            err.description()
        );
        let current = store.current().await.expect("current").expect("committed");
        assert_eq!(current.id, winner, "the current id still names the winner");
        let spec = memory.object(CONTAINER, &format!("{winner}/spec.md")).expect("winning spec");
        assert_eq!(spec, b"# Spec winner\n", "the winning revision is intact");
    }

    fn revision(spec: &str) -> Revision {
        Revision {
            spec: spec.to_string(),
            design: "# Design\n".to_string(),
        }
    }
}
