//! The revision store
//!
//! Where committed specifications live. A revision is the `spec.md` and
//! `design.md` pair one `specify` run produced; the store commits a new
//! revision, reads the current one, and reports how it differs from the one
//! it replaced.
//!
//! A revision is identified by the digest of its content, never a sequence
//! number, so the same documents always have the same id and a document that
//! no longer matches its id is recognised as corruption. Only the current
//! revision is kept, which keeps the store small and its meaning simple.

use std::collections::BTreeMap;
use std::fmt::Display;

use anyhow::Context;
use omnia_guest::{BlobStore, Error, StateStore, server_error};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::artifact::{Design, Spec};

/// Keyvalue key holding the current revision id.
pub const CURRENT: &str = "current-revision";

/// Blobstore container holding every revision's documents under `<id>/`.
pub const CONTAINER: &str = "revisions";

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

    /// Commits `revision` — diff against the readable predecessor, write,
    /// swap the current id, prune — returning the id with the diff.
    ///
    /// Fails if another run swapped the id first or storage refuses the write.
    pub async fn commit(&self, revision: &Revision) -> Result<Committed, Error> {
        // One observation feeds both the advisory diff and the CAS.
        let observed = self.observe().await;
        let diff = observed.outgoing.as_ref().map(|outgoing| Diff::between(outgoing, revision));
        let id = self.swap(revision, observed).await?;

        Ok(Committed { id, diff })
    }

    // Writes the documents and swaps the current id against `observed`;
    // a lost swap leaves the documents as an inert, unreferenced orphan.
    async fn swap(&self, revision: &Revision, observed: Observation) -> Result<String, Error> {
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

    /// Returns the current revision, or `None` before the first commit.
    /// Fails closed for a dangling, incomplete, unreadable, or tampered
    /// revision.
    pub async fn current(&self) -> Result<Option<Revision>, Error> {
        let Some(raw) =
            StateStore::get(self.store, CURRENT).await.context("getting current revision id")?
        else {
            return Ok(None);
        };

        let id = String::from_utf8(raw).context("decoding current revision id")?;
        let revision = self.load(&id).await?;

        Ok(Some(revision))
    }

    // Observes the CAS token and outgoing revision without failing; bad
    // state suppresses only the advisory diff, never the fail-closed CAS.
    async fn observe(&self) -> Observation {
        let token = StateStore::get(self.store, CURRENT).await.ok().flatten();

        let outgoing = if let Some(id) = token.as_deref().and_then(|raw| str::from_utf8(raw).ok()) {
            self.load(id).await.ok()
        } else {
            None
        };

        Observation { token, outgoing }
    }

    // The store is content-addressed: documents that no longer hash to
    // the id they sit under are corruption, not a revision.
    async fn load(&self, id: &str) -> Result<Revision, Error> {
        let spec = self.read(id, Spec::NAME).await?;
        let design = self.read(id, Design::NAME).await?;
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
            .with_context(|| format!("revision `{id}`: `{name}` is not UTF-8"))?;
        Ok(body)
    }
}

/// A complete specification revision.
///
/// The id is a function of the documents alone, so identical runs are
/// byte-stable and a revision read back from storage is verified
/// against the id it was stored under (`Store::load`).
#[derive(Debug)]
pub struct Revision {
    /// The behavioural specification document.
    pub spec: String,
    /// The rebuild design document.
    pub design: String,
}

impl Revision {
    /// The content-addressed revision id: SHA-256 over a domain tag,
    /// then the length-prefixed document names and bodies.
    #[must_use]
    pub fn id(&self) -> String {
        let mut hasher = Sha256::new();
        // hasher.update(b"emery-revision/1");
        for (name, body) in self.files() {
            hasher.update((name.len() as u64).to_be_bytes());
            hasher.update(name.as_bytes());
            hasher.update((body.len() as u64).to_be_bytes());
            hasher.update(body.as_bytes());
        }
        hex::encode(hasher.finalize())
    }

    // The one place a document name meets its field; digest order.
    fn files(&self) -> [(&'static str, &str); 2] {
        [(Spec::NAME, &self.spec), (Design::NAME, &self.design)]
    }
}

/// A committed revision: its id and the advisory re-mine diff against
/// the revision it superseded, when one was readable.
#[derive(Debug)]
pub struct Committed {
    /// The committed revision id.
    pub id: String,
    /// Absent on the first commit and when the predecessor was unreadable.
    pub diff: Option<Diff>,
}

// The current revision observed before a compare-and-swap; one
// observation drives one CAS and its advisory diff.
#[derive(Debug)]
struct Observation {
    // The raw CAS token exactly as storage holds it. Absent before the
    // first commit; also absent when storage could not be read, so the
    // subsequent CAS fails closed against a present key.
    token: Option<Vec<u8>>,
    // Advisory diff input; absent when no complete revision is readable.
    outgoing: Option<Revision>,
}

impl Observation {
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
    /// Requirement subjects that changed in `spec.md`.
    pub spec: Changes,
    /// Section headings that changed in `design.md`.
    pub design: Changes,
}

impl Diff {
    // Sections key on heading names, not positions. The diff is advisory:
    // an outgoing document that fails its grammar leaves the lists empty,
    // and the incoming documents were already parsed by synthesis.
    fn between(outgoing: &Revision, incoming: &Revision) -> Self {
        let artifacts = outgoing
            .files()
            .iter()
            .zip(incoming.files())
            .filter(|((_, old), (_, new))| old != new)
            .map(|((name, _), _)| (*name).to_string())
            .collect();

        let spec = match (outgoing.spec.parse::<Spec>(), incoming.spec.parse::<Spec>()) {
            (Ok(old), Ok(new)) => Changes::between(&old.by_subject(), &new.by_subject()),
            _ => Changes::default(),
        };
        let design = match (outgoing.design.parse::<Design>(), incoming.design.parse::<Design>()) {
            (Ok(old), Ok(new)) => Changes::between(&old.by_kind(), &new.by_kind()),
            _ => Changes::default(),
        };

        Self {
            from: outgoing.id(),
            artifacts,
            spec,
            design,
        }
    }

    /// Returns whether the revisions are byte-identical; identical bytes
    /// cannot yield section differences.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

/// The sections of one document that differ between two revisions, keyed
/// by heading name.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Changes {
    /// Headings present only in the incoming document.
    pub added: Vec<String>,
    /// Headings present only in the outgoing document.
    pub removed: Vec<String>,
    /// Headings whose sections changed.
    pub changed: Vec<String>,
}

impl Changes {
    // A section that only moved is not a change.
    fn between<K: Display + Ord, S: PartialEq>(
        old: &BTreeMap<K, &S>, new: &BTreeMap<K, &S>,
    ) -> Self {
        let mut changes = Self::default();
        for (heading, section) in new {
            let bucket = match old.get(heading) {
                None => &mut changes.added,
                Some(previous) if *previous != *section => &mut changes.changed,
                Some(_) => continue,
            };
            bucket.push(heading.to_string());
        }
        changes.removed.extend(
            old.keys().filter(|heading| !new.contains_key(*heading)).map(ToString::to_string),
        );
        changes
    }
}

// Keep (entry-point-unreachable): two runs racing one current id cannot
// be arranged through the CLI, whose `commit` observes and swaps as one;
// everything else the store does is owned by the root scenarios.
#[cfg(test)]
mod tests {
    use omnia_test::guest::Memory;

    use crate::store::{CONTAINER, Revision, Store};

    #[tokio::test]
    async fn concurrent_commit_conflicts() {
        let memory = Memory::default();
        let store = Store::new(&memory);

        // Both runs observe the empty store; the winner swaps first.
        let stale = store.observe().await;
        let observed = store.observe().await;
        let winner = store.swap(&revision("# Spec winner\n"), observed).await.expect("commit");

        let err = store
            .swap(&revision("# Spec loser\n"), stale)
            .await
            .expect_err("a stale observation must never last-write-wins over the swapped id");
        assert_eq!(err.code(), "server_error", "typed failure");
        assert!(
            err.description().contains("swapping current revision"),
            "typed failure: {}",
            err.description()
        );
        let current = store.current().await.expect("current").expect("committed");
        assert_eq!(current.id(), winner, "the current id still names the winner");
        let spec = memory.object(CONTAINER, &format!("{winner}/spec.md")).expect("winning spec");
        assert_eq!(spec, b"# Spec winner\n", "the winning revision is intact");
    }

    fn revision(spec: &str) -> Revision {
        Revision {
            spec: spec.to_string(),
            design: "# Design\n\n## Overview\n\nOne endpoint.\n".to_string(),
        }
    }
}
