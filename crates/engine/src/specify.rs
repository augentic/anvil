//! The specification-generation operation: extract every bound source,
//! reconcile, synthesise, and commit one generation.

use emery_source::Source;
use omnia_guest::api::{Context, Handler};
use omnia_guest::plugins::Digest;
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore};
use serde::{Deserialize, Serialize};

use crate::extract::extract_all;
use crate::home::{Diff, Home, SpecSet};
use crate::sources::{SourceBinding, validate};
use crate::synthesise::{reconcile, synthesise};

/// Generate a specification generation from source bindings.
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
    /// Committed generation id.
    pub generation: String,
    /// Number of committed requirements.
    pub requirements: usize,
    /// Number of extracted sources.
    pub sources: usize,
    /// Diff from the predecessor; absent on the first run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
    /// Resolved content digests of loader-loaded adapters (local
    /// components and registry packages) — commit one as its
    /// binding's `digest` pin to make the load reproducible
    /// (trust-on-first-use).
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

impl<P: Model + Source + StateStore + BlobStore + Plugins> Handler<P> for Specify {
    type Error = Error;
    type Output = SpecifyBody;

    async fn handle(self, context: Context<'_, P>) -> Result<Self::Output, Self::Error> {
        let Self { bindings } = self;
        validate(&bindings)?;

        let sets = extract_all(context.provider, &bindings).await?;
        let rows = reconcile(&sets);
        let documents = synthesise(context.provider, &sets, &rows).await?;

        let set = SpecSet {
            spec: documents.spec,
            design: documents.design,
        };
        let home = Home::new(context.provider);
        // One observation feeds both the CAS expected value and the
        // re-mine diff, computed in memory and emitted only here.
        let observed = home.observe().await;
        let committed = home.commit(&set, &observed).await?;
        let diff =
            observed.into_outgoing().map(|(from, previous)| Diff::between(from, &previous, &set));
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
            generation: committed.id,
            requirements: rows.len(),
            sources: sets.len(),
            diff,
            digests,
        })
    }
}
