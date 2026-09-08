//! Source extraction
//!
//! The first leg of a `specify` run: every bound source is handed to its
//! adapter, and the adapter returns the claims it found. The result is one
//! [`SourceSet`] per binding, carrying the claims, their authority class,
//! and the digest of the adapter that produced them.
//!
//! Adapters are guests the engine did not write, so their claims are checked
//! against the contract's claim rules before anything downstream trusts them.
//! A source that returns invalid claims stops the run with a typed error
//! rather than seeding a bad specification.

use emery_source::types::{Authority, Claim, ClaimKind};
use emery_source::{Source, claims};
use omnia_guest::plugins::Digest;
use omnia_guest::{Error, Plugins, bad_gateway, bad_request};

use crate::plugin::Loader;
use crate::specify::SourceBinding;

/// Loads, extracts, and validates every source binding.
pub async fn extract<P: Source + Plugins>(
    provider: &P, bindings: &[SourceBinding],
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
            authority: evidence.authority,
            claims: evidence.claims,
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
    /// Claim-set authority class.
    pub authority: Authority,
    /// The validated claims.
    pub claims: Vec<Claim>,
    /// Resolved content digest of a loader-loaded adapter.
    pub digest: Option<Digest>,
}

impl SourceSet {
    /// Every `type` claim.
    pub fn types(&self) -> impl Iterator<Item = &Claim> {
        self.claims.iter().filter(|claim| claim.kind == ClaimKind::Type)
    }

    // Validates claim grammar and required extras fail-closed (A8).
    fn validate(&self) -> Result<(), Error> {
        let findings = claims::findings(&self.claims);
        if !findings.is_empty() {
            let key = &self.key;
            let findings = findings.join("\n");
            return Err(bad_request!("source `{key}` returned invalid claims:\n{findings}"));
        }
        Ok(())
    }
}
