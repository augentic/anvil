//! Source extraction
//!
//! The first leg of a `specify` run: every bound source is handed to its
//! adapter, and the adapter returns the claims it found. The result is one
//! [`SourceSet`] per binding, carrying the evidence document and the digest
//! of the adapter that produced it.
//!
//! Adapters are guests the engine did not write, so their claims are checked
//! against the contract's claim rules before anything downstream trusts them.
//! A source that returns invalid claims stops the run with a typed error
//! rather than seeding a bad specification.

use emery_source::Source;
use emery_source::types::{self, Claim, Evidence};
use omnia_guest::plugins::Digest;
use omnia_guest::{Error, Plugins, bad_gateway, bad_request};

use crate::plugin::Loader;
use crate::specify::SourceConfig;

/// Loads, extracts, and validates every source binding.
pub async fn extract<P: Source + Plugins>(
    provider: &P, bindings: &[SourceConfig],
) -> Result<Vec<SourceSet>, Error> {
    let mut sets = Vec::with_capacity(bindings.len());
    let loader = Loader::new(provider);

    for binding in bindings {
        let input = binding.input()?;
        let adapter = binding.load(&loader).await?;

        tracing::debug!(source = %binding.key, "extracting");
        let id = &adapter.id;
        let evidence = Source::extract(provider, id, &input)
            .await
            .map_err(|err| bad_gateway!("source `{id}`: {err}"))?;

        let set = SourceSet {
            key: binding.key.clone(),
            evidence,
            digest: adapter.digest,
        };
        set.validate()?;
        sets.push(set);
    }

    Ok(sets)
}

/// A validated claim set extracted from one source.
#[derive(Debug)]
pub struct SourceSet {
    /// The authored binding key.
    pub key: String,
    /// The validated evidence document.
    pub evidence: Evidence,
    /// Resolved content digest of a loader-loaded adapter.
    pub digest: Option<Digest>,
}

impl SourceSet {
    /// Every `type` claim.
    pub fn types(&self) -> impl Iterator<Item = &Claim> {
        self.evidence.types()
    }

    // Re-runs the contract's claim gate fail-closed (A8); the guest's own
    // check cannot be trusted over the wire.
    fn validate(&self) -> Result<(), Error> {
        self.evidence.validate().map_err(|err| {
            let key = &self.key;
            let (types::Error::Internal(detail)
            | types::Error::InvalidRequest(detail)
            | types::Error::Io(detail)) = err;
            bad_request!("source `{key}` returned {detail}")
        })
    }
}
