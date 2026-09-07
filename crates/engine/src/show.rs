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

use crate::store::{Revision, Store};

/// Read one document of the current revision.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Show {
    /// Which document to read.
    pub document: Document,
}

/// The reviewable documents of one revision.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Document {
    /// The behavioural specification document.
    Spec,
    /// The rebuild design document.
    Design,
}

impl Document {
    // The one place a document variant meets its revision field.
    fn body(self, revision: Revision) -> String {
        match self {
            Self::Spec => revision.spec,
            Self::Design => revision.design,
        }
    }
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

#[omnia_guest::handler]
async fn show<P: StateStore + BlobStore>(
    input: Show, context: Context<'_, P>,
) -> Result<ShowBody, Error> {
    let Show { document } = input;

    let Some(revision) = Store::new(context.provider).current().await? else {
        return Err(Error::NotFound {
            code: "spec-not-generated".into(),
            description: "no specification revision has been committed".into(),
        });
    };

    Ok(ShowBody {
        revision: revision.id(),
        document,
        body: document.body(revision),
    })
}
