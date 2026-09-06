//! Source extraction: resolve each binding, extract over the `Source`
//! capability, and re-run the A8 claim gate fail-closed.

use emery_source::types::{Authority, Claim};
use emery_source::{Source, claims};
use omnia_guest::plugins::Digest;
use omnia_guest::{Error, Plugins, bad_gateway, bad_request};

use crate::resolve::Resolver;
use crate::sources::SourceBinding;

/// Resolves, extracts, and validates every source binding.
pub async fn extract<P: Source + Plugins>(
    provider: &P, bindings: &[SourceBinding],
) -> Result<Vec<SourceSet>, Error> {
    let mut sets = Vec::with_capacity(bindings.len());
    let resolver = Resolver::new(provider);

    for binding in bindings {
        let input = binding.input()?;
        let adapter = binding.resolve(&resolver).await?;

        tracing::debug!(source = %binding.key, "extracting");
        let evidence = Source::extract(provider, &adapter.id, &input)
            .await
            .map_err(|err| bad_gateway!("source `{}`: {err}", adapter.id))?;

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
    // Validates claim grammar and required extras fail-closed (A8).
    fn validate(&self) -> Result<(), Error> {
        let findings = claims::findings(&self.claims);
        if !findings.is_empty() {
            return Err(bad_request!(
                "source `{}` returned an invalid claim set (A8 fail-closed):\n{}",
                self.key,
                findings.join("\n")
            ));
        }
        Ok(())
    }
}
