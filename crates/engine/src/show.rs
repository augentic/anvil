//! The `show` operation
//!
//! Reads one document — `spec.md` or `design.md` — from the current
//! specification revision so an operator, or a skill acting for one, can
//! review what the last `specify` committed.
//!
//! Review goes through this operation rather than the filesystem so the
//! revision store stays the engine's own: callers see a document paired with
//! the revision id it belongs to, and never the storage layout beneath it.

use omnia_guest::api::Context;
use omnia_guest::{BlobStore, Error, StateStore};
use serde::{Deserialize, Serialize};

pub use crate::artifact::Document;
use crate::store::Store;

/// Read one document of the current revision.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Show {
    /// Which document to read.
    pub document: Document,
}

/// Successful review result.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowBody {
    /// Current revision id.
    pub revision: String,
    /// Which document `body` carries.
    pub document: Document,
    /// The document body.
    pub body: String,
}

/// Read one document of the current revision over the context's provider.
///
/// # Errors
///
/// Returns `NotFound` (`spec-not-generated`) when no revision has been
/// committed, and passes through the store's failures.
pub async fn show<P: StateStore + BlobStore>(
    input: Show, context: Context<P>,
) -> Result<ShowBody, Error> {
    let Show { document } = input;

    let Some(revision) = Store::new(context.provider()).current().await? else {
        return Err(Error::NotFound {
            code: "spec-not-generated".into(),
            description: "no specification revision has been committed".into(),
        });
    };

    Ok(ShowBody {
        revision: revision.id(),
        document,
        body: revision.into_body(document),
    })
}
