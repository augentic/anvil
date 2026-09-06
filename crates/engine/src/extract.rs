//! # Extract
//!
//! Source extraction and claim validation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use emery_source::types::{Authority, Claim, SourceContent, SourceInput, SourceWorkspace};
use emery_source::{DispatchError, Source, claims};
use omnia_guest::plugins::Digest;
use omnia_guest::{Error, Plugins, bad_gateway, bad_request};

use crate::preopen::preopen_path;
use crate::resolve::{self, AdapterSelector};
use crate::sources::{BindingContent, SourceBinding};

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
    /// Validates claim grammar and required extras fail-closed (A8).
    ///
    /// The engine re-runs the contract's [`claims`] gate over the wire
    /// because it cannot trust the guest to have run the SDK tail.
    ///
    /// # Errors
    ///
    /// Returns a `BadRequest` naming every finding.
    pub fn validate(&self) -> Result<(), Error> {
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

/// Resolves, extracts, and validates every source binding.
///
/// A local component or a registry package loads through the
/// deployment's loader — a component read fresh on every run, a
/// package fetched from the binding's registry override or the
/// acquirer's default endpoint, either one's optional pin verified
/// host-side — before extract dispatch. One adapter identity loads
/// once; further bindings reuse the loaded guest.
///
/// # Errors
///
/// Propagates resolution, extract, and [`SourceSet::validate`] failures.
pub async fn extract_all<P: Source + Plugins>(
    provider: &P, bindings: &[SourceBinding],
) -> Result<Vec<SourceSet>, Error> {
    let mut sets = Vec::with_capacity(bindings.len());
    let mut loaded = BTreeMap::new();

    for binding in bindings {
        let resolved = resolve_once(provider, binding, &mut loaded).await?;
        let input = binding.input()?;
        tracing::debug!(source = %binding.key, adapter = %resolved.id, "extracting");
        let evidence = dispatch(provider, &resolved.id, &input).await?;

        let set = SourceSet {
            key: binding.key.clone(),
            authority: evidence.authority,
            claims: evidence.claims,
            digest: resolved.digest,
        };

        set.validate()?;
        tracing::debug!(source = %set.key, claims = set.claims.len(), "extracted");
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

impl SourceBinding {
    // `.` spans the project preopen, including `.emery/`, until guest
    // capability profiles can exclude the output home.
    fn input(&self) -> Result<SourceInput, Error> {
        let content = match &self.content {
            BindingContent::Workspace(relative) => {
                let relative = preopen_path(Path::new(relative))?;
                let root = if relative == Path::new(".") {
                    PathBuf::from(".")
                } else {
                    Path::new(".").join(&relative)
                };
                SourceContent::Workspace(SourceWorkspace {
                    id: self.key.clone(),
                    root: root.display().to_string(),
                })
            }
            BindingContent::Description(text) => SourceContent::Value(text.clone()),
        };
        Ok(SourceInput {
            key: self.key.clone(),
            content,
        })
    }
}

// On wasm32, the routed id selects the exporting guest through Omnia.
async fn dispatch<P: Source>(
    provider: &P, id: &str, input: &SourceInput,
) -> Result<emery_source::types::Evidence, Error> {
    provider.extract(id, input).await.map_err(|err| match err {
        DispatchError::Call(failure) => bad_gateway!("source `{id}`: {failure}",),
        extras @ DispatchError::Extras { .. } => bad_gateway!("source `{id}` {extras}",),
    })
}
