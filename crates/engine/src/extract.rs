//! Source extraction and fail-closed claim validation.

use std::path::{Path, PathBuf};

use emery_adapter::answers::claim_id_findings;
use emery_adapter::types::{
    Authority, Claim, ClaimKind, SourceContent, SourceInput, SourceWorkspace,
};
use emery_adapter::{DispatchError, Source};
use omnia_guest::plugins::Digest;
use omnia_guest::{Error, Plugins, bad_gateway, bad_request};

use crate::handler::preopen_path;
use crate::resolve::{self, AdapterSelector};
use crate::sources::{BindingContent, SourceBinding};

// On wasm32, the routed id selects the exporting guest through Omnia.
async fn dispatch<P: Source>(
    provider: &P, id: &str, input: &SourceInput,
) -> Result<emery_adapter::types::Evidence, Error> {
    provider.extract(id, input).await.map_err(|err| match err {
        DispatchError::Call(failure) => bad_gateway!("source `{id}`: {failure}",),
        extras @ DispatchError::Extras { .. } => bad_gateway!("source `{id}` {extras}",),
    })
}

/// A validated claim set extracted from one source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSet {
    /// The authored binding key.
    pub key: String,
    /// The routed adapter identity: a package reference for a
    /// registry adapter, `source:<name>` otherwise.
    pub adapter: String,
    /// Claim-set authority class.
    pub authority: Authority,
    /// The validated claims.
    pub claims: Vec<Claim>,
    /// Resolved content digest of a loader-loaded adapter.
    pub digest: Option<Digest>,
}

/// Resolves, extracts, and validates every source binding.
///
/// A local component or a registry package loads through the
/// deployment's loader — a component read fresh on every run, a
/// package fetched from the binding's registry override or the
/// acquirer's default endpoint, either one's optional pin verified
/// host-side — before extract dispatch.
///
/// # Errors
///
/// Propagates resolution, extract, and [`validate_set`] failures.
pub async fn extract_all<P: Source + Plugins>(
    provider: &P, bindings: &[SourceBinding],
) -> Result<Vec<SourceSet>, Error> {
    let mut sets = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let selector = AdapterSelector::parse(&binding.adapter)?;
        let resolved = resolve::source(
            provider,
            &selector,
            binding.digest.as_ref(),
            binding.registry.as_deref(),
        )
        .await?;
        let input = input_for(binding)?;
        let evidence = dispatch(provider, &resolved.id, &input).await?;
        let set = SourceSet {
            key: binding.key.clone(),
            adapter: resolved.id,
            authority: evidence.authority,
            claims: evidence.claims,
            digest: resolved.digest,
        };
        validate_set(&set)?;
        sets.push(set);
    }
    Ok(sets)
}

// `.` spans the project preopen, including `.emery/`, until guest
// capability profiles can exclude the output home.
fn input_for(binding: &SourceBinding) -> Result<SourceInput, Error> {
    let content = match &binding.content {
        BindingContent::Workspace(relative) => {
            let relative = preopen_path(Path::new(relative), "source path")?;
            let root = if relative == Path::new(".") {
                PathBuf::from(".")
            } else {
                Path::new(".").join(&relative)
            };
            SourceContent::Workspace(SourceWorkspace {
                id: binding.key.clone(),
                root: root.display().to_string(),
            })
        }
        BindingContent::Description(text) => SourceContent::Value(text.clone()),
    };
    Ok(SourceInput {
        key: binding.key.clone(),
        content,
    })
}

/// Returns the required extras for a claim kind.
///
/// Widening this closed table is a contract change.
#[must_use]
pub const fn required_extras(kind: ClaimKind) -> &'static [&'static str] {
    match kind {
        ClaimKind::Requirement => &["statement"],
        ClaimKind::Criterion => &["criterion"],
        ClaimKind::Example => &["replay-digest"],
        _ => &[],
    }
}

/// Validates claim grammar and required extras fail-closed.
///
/// # Errors
///
/// Returns a `BadRequest` when claim grammar or required extras fail.
pub fn validate_set(set: &SourceSet) -> Result<(), Error> {
    let findings = claim_id_findings(&set.claims);
    if !findings.is_empty() {
        return Err(bad_request!(
            "source `{}` returned an invalid claim set: {}",
            set.key,
            findings.join("; ")
        ));
    }
    for claim in &set.claims {
        for key in required_extras(claim.kind) {
            if !claim.extras.contains_key(*key) {
                let label = claim.id.clone().unwrap_or_else(|| claim.kind.to_string());
                return Err(bad_request!(
                    "required per-kind extras are absent (A8 fail-closed): source `{}` claim \
                     `{label}` is missing `{key}`",
                    set.key
                ));
            }
        }
    }
    Ok(())
}
