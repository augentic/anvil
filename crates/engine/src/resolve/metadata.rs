//! Adapter metadata dispatch.

use emery_error::Error;

use super::core::Axis;
use super::routed::RoutedId;

/// A source adapter's metadata answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    /// Optional Emery CLI compatibility floor.
    pub emery_floor: Option<String>,
}

/// Metadata dispatch request.
#[derive(Debug)]
pub struct Request<'a> {
    /// Adapter axis.
    pub axis: Axis,
    /// Exact routed adapter id.
    pub adapter_id: &'a str,
}

/// Deployment-supplied metadata dispatch.
pub trait Runner: Fn(&Request<'_>) -> Result<Metadata, Error> + Send + Sync {}

impl<F: Fn(&Request<'_>) -> Result<Metadata, Error> + Send + Sync> Runner for F {}

/// Creates metadata dispatch over the provider's `Source` capability.
///
/// Target requests return `adapter-axis-removed`.
pub fn runner<P: emery_adapter::Source>(provider: &P) -> impl Runner + '_ {
    move |request: &Request<'_>| match request.axis {
        Axis::Source => {
            let record = provider.metadata(request.adapter_id);
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

/// Dispatches metadata without guest-visible component access.
pub(super) fn dispatch(
    runner: &impl Runner, axis: Axis, name: &str, version: Option<&semver::Version>,
) -> Result<Metadata, Error> {
    let adapter_id = RoutedId::new(axis, name, version.cloned()).to_string();
    runner(&Request {
        axis,
        adapter_id: &adapter_id,
    })
}
