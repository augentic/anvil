//! # Show operation
//!
//! The `Show` operation reads a document from the current specification
//! revision.

use omnia_guest::api::Context;
use omnia_guest::{BlobStore, Error, StateStore};
use serde::{Deserialize, Serialize};

use crate::store::Store;

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

/// Successful review result.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowBody {
    /// Current revision id.
    pub revision: String,
    /// Which document `body` carries.
    pub document: Document,
    /// The document bytes.
    pub body: String,
}

#[omnia_guest::handler]
async fn show<P: StateStore + BlobStore>(
    input: Show, context: Context<'_, P>,
) -> Result<ShowBody, Error> {
    let store = Store::new(context.provider);

    let Some(revision) = store.current().await? else {
        return Err(Error::NotFound {
            code: "spec-not-generated".into(),
            description: "no specification revision has been committed".into(),
        });
    };

    let id = revision.id();
    let body = match input.document {
        Document::Spec => revision.spec,
        Document::Design => revision.design,
    };

    Ok(ShowBody {
        revision: id,
        document: input.document,
        body,
    })
}
