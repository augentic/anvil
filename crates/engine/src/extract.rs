//! # Extract
//!
//! Source extraction and claim validation.

use std::collections::BTreeMap;

use emery_source::types::{Authority, Claim};
use emery_source::{DispatchError, Source, claims};
use omnia_guest::plugins::Digest;
use omnia_guest::{Error, Plugins, bad_gateway, bad_request};

use crate::resolve::{self, AdapterSelector};
use crate::sources::SourceBinding;

/// Resolves, extracts, and validates every source binding.
///
/// # Errors
///
/// Propagates resolution, extract, and claim-gate failures.
pub async fn extract_all<P: Source + Plugins>(
    provider: &P, bindings: &[SourceBinding],
) -> Result<Vec<SourceSet>, Error> {
    let mut sets = Vec::with_capacity(bindings.len());
    let mut loaded = BTreeMap::new();

    for binding in bindings {
        let resolved = resolve_once(provider, binding, &mut loaded).await?;
        let input = binding.input()?;
        tracing::info!(source = %binding.key, "extracting");

        let evidence =
            Source::extract(provider, &resolved.id, &input).await.map_err(|err| match err {
                DispatchError::Call(failure) => bad_gateway!("source `{}`: {failure}", resolved.id),
                extras @ DispatchError::Extras { .. } => {
                    bad_gateway!("source `{}` {extras}", resolved.id)
                }
            })?;

        let set = SourceSet {
            key: binding.key.clone(),
            authority: evidence.authority,
            claims: evidence.claims,
            digest: resolved.digest,
        };

        set.validate()?;
        sets.push(set);
    }

    Ok(sets)
}

// The loader registers one identity per run; a second binding that
// reuses the adapter extracts over the already-loaded guest.
async fn resolve_once<P: Source + Plugins>(
    provider: &P, binding: &SourceBinding, loaded: &mut BTreeMap<String, resolve::Resolved>,
) -> Result<resolve::Resolved, Error> {
    let selector = AdapterSelector::parse(&binding.adapter)?;
    let key = selector.load_key()?;

    if let Some(existing) = key.as_ref().and_then(|key| loaded.get(key)) {
        existing.pin_agrees(binding.digest.as_ref())?;
        return Ok(existing.clone());
    }

    let resolved =
        resolve::source(provider, &selector, binding.digest.as_ref(), binding.registry.as_deref())
            .await?;
    if let Some(key) = key {
        loaded.insert(key, resolved.clone());
    }

    Ok(resolved)
}

/// A validated claim set extracted from one source.
#[derive(Debug, Clone)]
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
        if let findings = claims::findings(&self.claims)
            && !findings.is_empty()
        {
            return Err(bad_request!(
                "source `{}` returned an invalid claim set (A8 fail-closed): {}",
                self.key,
                findings.join("; ")
            ));
        }
        Ok(())
    }
}
