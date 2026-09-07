//! The specification-generation operation: extract every bound source,
//! reconcile, synthesise, and commit one revision.

use emery_source::Source;
use omnia_guest::api::Context;
use omnia_guest::plugins::Digest;
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore};
use serde::{Deserialize, Serialize};

use crate::extract::extract;
use crate::sources::{SourceBinding, validate};
pub use crate::store::Diff;
use crate::store::Store;
use crate::synthesise::{reconcile, synthesise};

/// Generate a specification revision from source bindings.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Specify {
    /// The run's source bindings, in extraction order.
    pub bindings: Vec<SourceBinding>,
}

/// Successful specification result.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyBody {
    /// Committed revision id.
    pub revision: String,
    /// Number of committed requirements.
    pub requirements: usize,
    /// Number of extracted sources.
    pub sources: usize,
    /// Diff from the predecessor; absent on the first run and when the
    /// superseded revision was unreadable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
    /// Resolved digests of loader-loaded adapters; commit one as its
    /// binding's `digest` pin to make the load reproducible.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub digests: Vec<SourceDigest>,
}

/// One loader-resolved source digest reported by `emery specify`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceDigest {
    /// The binding key.
    pub source: String,
    /// The resolved `sha256:<hex>` content digest.
    pub digest: Digest,
}

#[omnia_guest::handler]
async fn specify<P: Model + Source + StateStore + BlobStore + Plugins>(
    input: Specify, context: Context<'_, P>,
) -> Result<SpecifyBody, Error> {
    let Specify { bindings } = input;
    validate(&bindings)?;

    let sets = extract(context.provider, &bindings).await?;
    let rows = reconcile(&sets);
    let revision = synthesise(context.provider, &sets, &rows).await?;
    let committed = Store::new(context.provider).commit(&revision).await?;

    let digests = sets
        .iter()
        .filter_map(|source| {
            source.digest.clone().map(|digest| SourceDigest {
                source: source.key.clone(),
                digest,
            })
        })
        .collect();

    Ok(SpecifyBody {
        revision: committed.id,
        requirements: rows.len(),
        sources: sets.len(),
        diff: committed.diff,
        digests,
    })
}
