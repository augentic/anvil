//! Source extraction and fail-closed claim validation.

use std::path::Path;

use emery_adapter::types::{self, SourceContent, SourceInput, SourceWorkspace};
use emery_adapter::{DispatchError, Source};
use emery_artifacts::evidence::{AuthorityClass, Claim, ClaimKind, validate_claims};
use emery_error::Error;
use omnia_guest::{BlobStore, StateStore};

use crate::handler::{ExecutionPaths, preopen_path};
use crate::resolve::{AdapterSelector, Axis, RoutedId, ensure, metadata};
use crate::sources::{BindingContent, SourceBinding};

// On wasm32, the routed id selects the exporting guest through Omnia.
async fn dispatch<P: Source>(
    provider: &P, id: &str, input: &SourceInput,
) -> Result<types::Evidence, Error> {
    provider.extract(id, input).await.map_err(|err| match err {
        DispatchError::Call(failure) => Error::Diag {
            code: "source-extract-failed",
            detail: format!("source `{id}`: {failure}"),
        },
        extras @ DispatchError::Extras { .. } => Error::Diag {
            code: "claim-extras-malformed",
            detail: format!("source `{id}` {extras}"),
        },
    })
}

/// A validated claim set extracted from one source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSet {
    /// The authored binding key.
    pub key: String,
    /// The routed adapter identity (`source:<name>[@<version>]`).
    pub adapter: String,
    /// Claim-set authority class.
    pub authority: AuthorityClass,
    /// The validated claims.
    pub claims: Vec<Claim>,
}

/// Ensures, extracts, and validates every source binding.
///
/// Ensure runs here — a local component mirrors into the cache on the
/// first `specify` that names it.
///
/// # Errors
///
/// Propagates ensure, resolution, extract, and [`validate_set`] failures.
pub async fn extract_all<P: Source + StateStore + BlobStore>(
    provider: &P, bindings: &[SourceBinding], paths: &ExecutionPaths,
) -> Result<Vec<SourceSet>, Error> {
    let mut sets = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let selector = AdapterSelector::parse(&binding.adapter)?;
        let resolved =
            ensure::source(metadata::runner(provider), &selector, provider, paths).await?;
        let id = RoutedId::new(
            Axis::Source,
            resolved.manifest.name.clone(),
            resolved.manifest.version.clone(),
        )
        .to_string();
        let input = input_for(binding, paths)?;
        let evidence = dispatch(provider, &id, &input).await?;
        let set = SourceSet {
            key: binding.key.clone(),
            adapter: id,
            authority: authority(evidence.authority),
            claims: evidence.claims.into_iter().map(claim).collect(),
        };
        validate_set(&set)?;
        sets.push(set);
    }
    Ok(sets)
}

// `.` spans the project preopen, including `.emery/`, until guest
// capability profiles can exclude the output home.
fn input_for(binding: &SourceBinding, paths: &ExecutionPaths) -> Result<SourceInput, Error> {
    let content = match &binding.content {
        BindingContent::Workspace(relative) => {
            let relative = preopen_path(Path::new(relative), "source path")?;
            let joined = if relative == Path::new(".") {
                paths.project_root().to_path_buf()
            } else {
                paths.project_root().join(&relative)
            };
            SourceContent::Workspace(SourceWorkspace {
                id: binding.key.clone(),
                root: joined.display().to_string(),
            })
        }
        BindingContent::Value(value) => SourceContent::Value(value.clone()),
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
/// Returns `claim-invalid` or `claim-extras-missing`.
pub fn validate_set(set: &SourceSet) -> Result<(), Error> {
    let findings = validate_claims(&set.claims);
    if !findings.is_empty() {
        return Err(Error::validation_failed(
            "claim-invalid",
            format!("source `{}` returned an invalid claim set", set.key),
            findings.join("; "),
        ));
    }
    for claim in &set.claims {
        for key in required_extras(claim.kind) {
            if !claim.extras.contains_key(*key) {
                let label = claim.id.clone().unwrap_or_else(|| claim.kind.to_string());
                return Err(Error::validation_failed(
                    "claim-extras-missing",
                    "required per-kind extras are absent (A8 fail-closed)",
                    format!("source `{}` claim `{label}` is missing `{key}`", set.key),
                ));
            }
        }
    }
    Ok(())
}

const fn authority(record: types::Authority) -> AuthorityClass {
    match record {
        types::Authority::Intent => AuthorityClass::Intent,
        types::Authority::Documentation => AuthorityClass::Documentation,
        types::Authority::Behaviour => AuthorityClass::Behaviour,
    }
}

// Extras cross the contract verbatim.
fn claim(record: types::Claim) -> Claim {
    let mut mapped = Claim::new(kind(record.kind));
    mapped.id = record.id;
    mapped.path = record.path;
    mapped.synopsis = record.synopsis;
    mapped.set_backing(record.backing.map(|backing| match backing {
        types::Backing::Payload(payload) => emery_artifacts::evidence::Backing::Payload(payload),
        types::Backing::Path(path) => emery_artifacts::evidence::Backing::Path(path),
    }));
    mapped.extras = record.extras;
    mapped
}

const fn kind(record: types::ClaimKind) -> ClaimKind {
    match record {
        types::ClaimKind::Intent => ClaimKind::Intent,
        types::ClaimKind::Requirement => ClaimKind::Requirement,
        types::ClaimKind::Criterion => ClaimKind::Criterion,
        types::ClaimKind::Decision => ClaimKind::Decision,
        types::ClaimKind::Section => ClaimKind::Section,
        types::ClaimKind::Diagram => ClaimKind::Diagram,
        types::ClaimKind::Contract => ClaimKind::Contract,
        types::ClaimKind::Example => ClaimKind::Example,
        types::ClaimKind::Excerpt => ClaimKind::Excerpt,
        types::ClaimKind::Type => ClaimKind::Type,
        types::ClaimKind::Call => ClaimKind::Call,
        types::ClaimKind::Region => ClaimKind::Region,
        types::ClaimKind::Container => ClaimKind::Container,
        types::ClaimKind::Leaf => ClaimKind::Leaf,
    }
}
