//! WIT-backed capabilities used by the routed operations; mappings
//! live here so engine code remains wasm-free.

use std::sync::LazyLock;

use error::Error;
use project::adapter::metadata::{Metadata, Request};
use project::adapter::{AdapterSelector, Axis, ResolvedSource, ResolvedTarget, Resolver};
use project::handler::{Anchor, ExecutionPaths};

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
        project::adapter::resolver::Component::new(metadata).resolve_source(selector, paths)
    }

    fn resolve_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        project::adapter::resolver::Component::new(metadata).resolve_target(selector, paths)
    }

    async fn ensure_source(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        project::adapter::ensure::source(metadata, selector, paths, jiff::Timestamp::now())
    }

    async fn ensure_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        project::adapter::ensure::target(metadata, selector, paths, jiff::Timestamp::now())
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
                inputs: Vec::new(),
                platforms: None,
                writable_artifacts: Vec::new(),
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
