//! WIT-backed capabilities used by the routed operations; the wire
//! mapping lives in `emery_adapter::source::import`.

use std::sync::LazyLock;

use emery_adapter::seam;
use emery_adapter::source::import;
use emery_engine::handler::{Anchor, ExecutionPaths};
use emery_engine::resolve::metadata::{Metadata, Request};
use emery_engine::resolve::{AdapterSelector, Axis, ResolvedSource, Resolver};
use emery_error::Error;

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
        import::extract(id, input).await.map_err(|err| match err {
            import::Error::Seam(seam) => Error::Diag {
                code: "source-extract-failed",
                detail: format!("source `{id}`: {seam}"),
            },
            extras @ import::Error::Extras { .. } => Error::Diag {
                code: "claim-extras-malformed",
                detail: format!("source `{id}` {extras}"),
            },
        })
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
            let record = import::metadata(request.adapter_id);
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
