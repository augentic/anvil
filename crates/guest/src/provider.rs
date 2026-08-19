//! WIT-backed capabilities used by the routed operations; mappings
//! live here so engine code remains wasm-free.

use std::sync::LazyLock;

use emery_adapter::seam;
use emery_engine::handler::{Anchor, ExecutionPaths};
use emery_engine::resolve::metadata::{Metadata, Request};
use emery_engine::resolve::{AdapterSelector, Axis, ResolvedSource, Resolver};
use emery_error::Error;

use crate::bindings::emery::adapter::source;

/// Engine capabilities backed by the world's WIT imports.
#[derive(Clone, Copy, Debug)]
pub struct Provider;

/// The guest's execution paths: the project-root mount preopen at
/// `.` with the store and cache preopens the deployment manifest
/// grants as the carried locations — no environment reads and no
/// project-id keying in-guest.
static PATHS: LazyLock<ExecutionPaths> = LazyLock::new(ExecutionPaths::guest);

impl omnia_guest::Model for Provider {}

impl Anchor for Provider {
    fn paths(&self) -> &ExecutionPaths {
        &PATHS
    }
}

impl Resolver for Provider {
    fn resolve_source(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        emery_engine::resolve::resolver::Component::new(metadata).resolve_source(selector, paths)
    }

    async fn ensure_source(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        emery_engine::resolve::ensure::source(metadata, selector, paths, jiff::Timestamp::now())
    }
}

impl emery_engine::extract::Extract for Provider {
    async fn extract(&self, id: &str, input: &seam::SourceInput) -> Result<seam::Evidence, Error> {
        let wire = wire_input(input);
        let evidence = source::extract(id.to_string(), wire).await.map_err(|err| Error::Diag {
            code: "source-extract-failed",
            detail: format!("source `{id}`: {err:?}"),
        })?;
        seam_evidence(id, evidence)
    }
}

/// Project the seam input onto the WIT import's wire record.
fn wire_input(input: &seam::SourceInput) -> source::Input {
    source::Input {
        key: input.key.clone(),
        content: match &input.content {
            seam::SourceContent::Workspace(view) => source::Content::Workspace(source::Workspace {
                id: view.id.clone(),
                root: view.root.clone(),
            }),
            seam::SourceContent::Value(value) => source::Content::Value(value.clone()),
        },
    }
}

/// Lift a wire evidence record back onto the seam DTOs, parsing each
/// open extra's canonical JSON encoding fail-closed (A8): a value that
/// does not parse is a typed error, never a dropped key.
fn seam_evidence(id: &str, wire: source::Evidence) -> Result<seam::Evidence, Error> {
    let mut claims = Vec::with_capacity(wire.claims.len());
    for claim in wire.claims {
        let mut extras = serde_json::Map::new();
        for (key, encoded) in claim.extras {
            let value = serde_json::from_str(&encoded).map_err(|err| Error::Diag {
                code: "claim-extras-malformed",
                detail: format!(
                    "source `{id}` extra `{key}` is not canonical JSON ({err}): {encoded}"
                ),
            })?;
            extras.insert(key, value);
        }
        claims.push(seam::Claim {
            kind: seam_kind(claim.kind),
            id: claim.id,
            path: claim.path,
            synopsis: claim.synopsis,
            backing: claim.backing.map(|backing| match backing {
                source::Backing::Payload(payload) => seam::Backing::Payload(payload),
                source::Backing::Path(path) => seam::Backing::Path(path),
            }),
            extras,
        });
    }
    Ok(seam::Evidence {
        authority: match wire.authority {
            source::Authority::Intent => seam::Authority::Intent,
            source::Authority::Documentation => seam::Authority::Documentation,
            source::Authority::Behaviour => seam::Authority::Behaviour,
        },
        claims,
    })
}

const fn seam_kind(kind: source::ClaimKind) -> seam::ClaimKind {
    match kind {
        source::ClaimKind::Intent => seam::ClaimKind::Intent,
        source::ClaimKind::Requirement => seam::ClaimKind::Requirement,
        source::ClaimKind::Criterion => seam::ClaimKind::Criterion,
        source::ClaimKind::Decision => seam::ClaimKind::Decision,
        source::ClaimKind::Section => seam::ClaimKind::Section,
        source::ClaimKind::Diagram => seam::ClaimKind::Diagram,
        source::ClaimKind::Contract => seam::ClaimKind::Contract,
        source::ClaimKind::Example => seam::ClaimKind::Example,
        source::ClaimKind::Excerpt => seam::ClaimKind::Excerpt,
        source::ClaimKind::Type => seam::ClaimKind::Type,
        source::ClaimKind::Call => seam::ClaimKind::Call,
        source::ClaimKind::Region => seam::ClaimKind::Region,
        source::ClaimKind::Container => seam::ClaimKind::Container,
        source::ClaimKind::Leaf => seam::ClaimKind::Leaf,
    }
}

/// Resolve metadata through the deployed adapter identified by the request.
///
/// Dispatch is by adapter id rather than component path; deployment
/// assembly uses the same resolver precedence.
///
/// # Errors
///
/// The target axis is deleted from the deployment (ADR-0008): a
/// target-axis metadata request fails typed instead of dispatching.
pub fn metadata(request: &Request<'_>) -> Result<Metadata, Error> {
    match request.axis {
        Axis::Source => {
            let record = source::metadata(request.adapter_id);
            Ok(Metadata {
                emery_floor: record.emery_floor,
            })
        }
        Axis::Target => Err(Error::Diag {
            code: "adapter-axis-removed",
            detail: format!(
                "the target adapter axis is deleted (ADR-0008); `{}` cannot be resolved",
                request.adapter_id
            ),
        }),
    }
}
