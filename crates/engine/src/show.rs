//! The review operation: a verifiable, non-authoritative projection of
//! one document from the current generation.

use omnia_guest::api::Context;
use omnia_guest::{BlobStore, Error, StateStore};
use serde::{Deserialize, Serialize};

use crate::home::Home;

/// Read one document of the current generation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Show {
    /// Which document to read.
    pub document: Document,
}

/// The reviewable documents of one generation.
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
    /// Current generation id.
    pub generation: String,
    /// Which document `body` carries.
    pub document: Document,
    /// The document bytes.
    pub body: String,
}

#[omnia_guest::handler]
async fn show<P: StateStore + BlobStore>(
    input: Show, context: Context<'_, P>,
) -> Result<ShowBody, Error> {
    let home = Home::new(context.provider);
    let Some((committed, set)) = home.current_set().await? else {
        return Err(Error::NotFound {
            code: "spec-not-generated".into(),
            description: "no specification generation has been committed".into(),
        });
    };
    let body = match input.document {
        Document::Spec => set.spec,
        Document::Design => set.design,
    };

    Ok(ShowBody {
        generation: committed.id,
        document: input.document,
        body,
    })
}
