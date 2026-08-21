//! The extract leg: dispatch each authored binding over the source
//! seam, fail closed on the required-extras table, and record one
//! receipt per source.

use emery_adapter::seam::{self, SourceContent, SourceInput, SourceWorkspace};
use emery_adapter::{DispatchError, Source};
use emery_artifacts::evidence::{AuthorityClass, Claim, ClaimKind, validate_claims};
use emery_error::Error;
use serde::Serialize;

use crate::handler::ExecutionPaths;
use crate::project::{BindingContent, Project, SourceBinding};
use crate::resolve::{AdapterSelector, Axis, RoutedId, metadata, resolver};

// Dispatch one `extract` over the provider's source-seam capability
// (on wasm32, Omnia's host-mediated link routes to the exporting
// guest by the routed `id`), mapping seam failures onto operator codes.
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

/// One extracted source: the binding key, the routed adapter identity
/// it dispatched by, and the validated claim set in the persisted
/// Evidence dialect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSet {
    /// The authored binding key.
    pub key: String,
    /// The routed adapter identity (`source:<name>[@<version>]`).
    pub adapter: String,
    /// Document-level authority class of the claim set.
    pub authority: AuthorityClass,
    /// The validated claims.
    pub claims: Vec<Claim>,
}

/// One extract receipt persisted into the generation: source identity
/// plus the claim-set digest. No timestamps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Receipt {
    /// The authored binding key.
    pub key: String,
    /// The routed adapter identity the extract dispatched by.
    pub adapter: String,
    /// Document-level authority class.
    pub authority: AuthorityClass,
    /// Number of claims extracted.
    pub claims: usize,
    /// `sha256:<hex>` over the claim set's canonical JSON.
    pub digest: String,
}

impl Receipt {
    /// The receipt of one extracted source.
    #[must_use]
    pub fn of(set: &SourceSet) -> Self {
        let body = serde_json::json!({
            "authority": set.authority,
            "claims": set.claims,
        });
        let canonical = emery_diagnostics::fingerprint::canonical_json(&body);
        Self {
            key: set.key.clone(),
            adapter: set.adapter.clone(),
            authority: set.authority,
            claims: set.claims.len(),
            digest: format!(
                "sha256:{}",
                emery_diagnostics::digest::sha256_hex(canonical.as_bytes())
            ),
        }
    }
}

/// Extract every authored binding: resolve on the source axis,
/// dispatch over the provider's seam capability, and validate
/// fail-closed.
///
/// # Errors
///
/// Resolution failures, seam failures, and the typed validation
/// refusals from [`validate_set`].
pub async fn extract_all<P: Source>(
    provider: &P, project: &Project, paths: &ExecutionPaths,
) -> Result<Vec<SourceSet>, Error> {
    let component = resolver::Component::new(metadata::runner(provider));
    let mut sets = Vec::with_capacity(project.sources.len());
    for binding in &project.sources {
        let selector = AdapterSelector::parse(&binding.adapter)?;
        let resolved = component.resolve_source(&selector, paths)?;
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

// The typed seam input for one binding. A `.`-rooted view spans the
// project preopen, `.emery/` included — the per-guest output-home
// exclusion is a deferred capability profile.
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

/// The closed required-extras table per claim kind. Widening it is a
/// contract change gated by the decision log.
#[must_use]
pub const fn required_extras(kind: ClaimKind) -> &'static [&'static str] {
    match kind {
        ClaimKind::Requirement => &["statement"],
        ClaimKind::Criterion => &["criterion"],
        ClaimKind::Example => &["replay-digest"],
        _ => &[],
    }
}

/// Fail-closed claim-set validation.
///
/// Id grammar and presence per the persisted dialect, plus the
/// required-extras table: a violating claim is a typed error naming
/// source, claim, and missing key — never a synopsis fallback.
///
/// # Errors
///
/// `claim-invalid` for id violations; `claim-extras-missing` for an
/// absent required extra.
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

// Map the seam authority onto the persisted dialect.
const fn authority(seam: seam::Authority) -> AuthorityClass {
    match seam {
        seam::Authority::Intent => AuthorityClass::Intent,
        seam::Authority::Documentation => AuthorityClass::Documentation,
        seam::Authority::Behaviour => AuthorityClass::Behaviour,
    }
}

// Map one seam claim onto the persisted dialect, conserving extras
// verbatim.
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

// Map the seam claim kind onto the persisted dialect.
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
