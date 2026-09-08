//! The `specify` operation
//!
//! Emery's central operation: given a list of source bindings, extract each
//! source's claims, reconcile them under authority precedence, synthesise
//! `spec.md` and `design.md`, and commit the pair as one new revision.
//!
//! The result reports what was committed — the revision id, the counts, the
//! resolved adapter digests an operator can pin, and the diff against the
//! superseded revision — so a caller can see what changed without reading
//! the documents.

mod extract;
mod synthesise;

use emery_source::Source;
use omnia_guest::api::Context;
use omnia_guest::plugins::Digest;
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore};
use serde::{Deserialize, Serialize};

use self::extract::extract;
use self::synthesise::{reconcile, synthesise};
use crate::sources::{SourceBinding, validate};
pub use crate::store::Diff;
use crate::store::Store;

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

/// Run one `specify` over the context's provider.
///
/// # Errors
///
/// Returns `BadRequest` for a binding the rules refuse or a claim the gate
/// rejects, and passes through the extract, synthesis, and store failures.
pub async fn specify<P: Model + Source + StateStore + BlobStore + Plugins>(
    input: Specify, context: Context<P>,
) -> Result<SpecifyBody, Error> {
    let Specify { bindings } = input;
    validate(&bindings)?;

    let provider = context.provider();
    let sets = extract(provider, &bindings).await?;
    let rows = reconcile(&sets);
    let revision = synthesise(provider, &sets, &rows).await?;
    let committed = Store::new(provider).commit(&revision).await?;

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
