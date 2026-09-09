//! Source extraction
//!
//! The first leg of a `specify` run: every source is handed to its
//! adapter, and the adapter returns the claims it found. The result is one
//! [`SourceEvidence`] per source — the evidence document under the key the
//! documents cite it by.
//!
//! Adapters are guests the engine did not write, so their claims are checked
//! against the contract's claim rules before anything downstream trusts them.
//! A source that returns invalid claims stops the run with a typed error
//! rather than seeding a bad specification.

use emery_source::Source;
use emery_source::types::{self, Evidence};
use omnia_guest::{Error, Plugins, bad_gateway, bad_request};

use crate::plugin::Loader;
use crate::specify::SourceConfig;

/// Loads, extracts, and validates every source.
pub async fn evidence<P: Source + Plugins>(
    provider: &P, sources: &[SourceConfig],
) -> Result<Vec<SourceEvidence>, Error> {
    let mut extracted = Vec::with_capacity(sources.len());
    let loader = Loader::new(provider);

    for source in sources {
        let input = source.input()?;
        let id = source.load(&loader).await?;

        let key = &source.key;
        tracing::debug!(source = %key, "extracting");

        let evidence = Source::extract(provider, &id, &input)
            .await
            .map_err(|err| bad_gateway!("source `{id}`: {err}"))?;

        // re-validate claims against the contract's rules
        evidence.validate().map_err(|err| {
            let (types::Error::Internal(detail)
            | types::Error::InvalidRequest(detail)
            | types::Error::Io(detail)) = err;
            bad_request!("source `{key}` returned {detail}")
        })?;

        extracted.push(SourceEvidence {
            key: key.clone(),
            evidence,
        });
    }

    Ok(extracted)
}

/// One source's evidence, under the key the documents cite it by.
#[derive(Debug)]
pub struct SourceEvidence {
    /// The authored source key.
    pub key: String,
    /// The validated evidence document.
    pub evidence: Evidence,
}
