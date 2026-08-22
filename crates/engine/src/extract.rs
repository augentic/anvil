//! Source extraction and fail-closed claim validation.

use emery_adapter::seam::{self, SourceContent, SourceInput, SourceWorkspace};
use emery_adapter::{DispatchError, Source};
use emery_artifacts::evidence::{AuthorityClass, Claim, ClaimKind, validate_claims};
use emery_error::Error;
use omnia_guest::{BlobStore, StateStore};

use crate::handler::ExecutionPaths;
use crate::project::{BindingContent, Project, SourceBinding};
use crate::resolve::{AdapterSelector, Axis, RoutedId, metadata, resolver};

// On wasm32, the routed id selects the exporting guest through Omnia.
async fn dispatch<P: Source>(
    provider: &P, id: &str, input: &SourceInput,
) -> Result<seam::Evidence, Error> {
    provider.extract(id, input).await.map_err(|err| match err {
        DispatchError::Seam(seam) => Error::Diag {
            code: "source-extract-failed",
            detail: format!("source `{id}`: {seam}"),
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

/// Resolves, extracts, and validates every source binding.
///
/// # Errors
///
/// Propagates resolution, seam, and [`validate_set`] failures.
pub async fn extract_all<P: Source + StateStore + BlobStore>(
    provider: &P, project: &Project, paths: &ExecutionPaths,
) -> Result<Vec<SourceSet>, Error> {
    let component = resolver::Component::new(metadata::runner(provider));
    let mut sets = Vec::with_capacity(project.sources.len());
    for binding in &project.sources {
        let selector = AdapterSelector::parse(&binding.adapter)?;
        let resolved = component.resolve_source(&selector, provider, paths).await?;
        let id = RoutedId::new(
            Axis::Source,
            resolved.manifest.name.clone(),
            resolved.manifest.version.clone(),
        )
        .to_string();
        let input = input_for(binding, paths);
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
fn input_for(binding: &SourceBinding, paths: &ExecutionPaths) -> SourceInput {
    let content = match &binding.content {
        BindingContent::Workspace(relative) => {
            let root = if relative == "." {
                paths.project_root().to_path_buf()
            } else {
                paths.project_root().join(relative)
            };
            SourceContent::Workspace(SourceWorkspace {
                id: binding.key.clone(),
                root: root.display().to_string(),
            })
        }
        BindingContent::Value(value) => SourceContent::Value(value.clone()),
    };
    SourceInput {
        key: binding.key.clone(),
        content,
    }
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

const fn authority(seam: seam::Authority) -> AuthorityClass {
    match seam {
        seam::Authority::Intent => AuthorityClass::Intent,
        seam::Authority::Documentation => AuthorityClass::Documentation,
        seam::Authority::Behaviour => AuthorityClass::Behaviour,
    }
}

// Extras cross the seam verbatim.
fn claim(seam: seam::Claim) -> Claim {
    let mut mapped = Claim::new(kind(seam.kind));
    mapped.id = seam.id;
    mapped.path = seam.path;
    mapped.synopsis = seam.synopsis;
    mapped.set_backing(seam.backing.map(|backing| match backing {
        seam::Backing::Payload(payload) => emery_artifacts::evidence::Backing::Payload(payload),
        seam::Backing::Path(path) => emery_artifacts::evidence::Backing::Path(path),
    }));
    mapped.extras = seam.extras;
    mapped
}

const fn kind(seam: seam::ClaimKind) -> ClaimKind {
    match seam {
        seam::ClaimKind::Intent => ClaimKind::Intent,
        seam::ClaimKind::Requirement => ClaimKind::Requirement,
        seam::ClaimKind::Criterion => ClaimKind::Criterion,
        seam::ClaimKind::Decision => ClaimKind::Decision,
        seam::ClaimKind::Section => ClaimKind::Section,
        seam::ClaimKind::Diagram => ClaimKind::Diagram,
        seam::ClaimKind::Contract => ClaimKind::Contract,
        seam::ClaimKind::Example => ClaimKind::Example,
        seam::ClaimKind::Excerpt => ClaimKind::Excerpt,
        seam::ClaimKind::Type => ClaimKind::Type,
        seam::ClaimKind::Call => ClaimKind::Call,
        seam::ClaimKind::Region => ClaimKind::Region,
        seam::ClaimKind::Container => ClaimKind::Container,
        seam::ClaimKind::Leaf => ClaimKind::Leaf,
    }
}
