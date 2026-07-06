//! The native binary's describe runner: adapts `specify-runtime`'s
//! wasmtime-backed `describe` dispatch onto the wasmtime-free seam in
//! `specify_workflow::adapter::describe` (RFC-64). Registered once at
//! process start by [`crate::runtime::run`].

use std::str::FromStr;

use specify_error::Error;
use specify_runtime::describe::{DescribeAxis, DescribeFailure, DescribeValue, describe_adapter};
use specify_workflow::Platform;
use specify_workflow::adapter::describe::{
    DescribeAnswer, DescribeRequest, register_describe_runner,
};
use specify_workflow::adapter::{Axis, BuildInputDeclaration, PlatformsCapability};

/// Register the process-global describe runner (idempotent).
pub fn register() {
    register_describe_runner(run);
}

fn run(request: &DescribeRequest<'_>) -> Result<DescribeAnswer, Error> {
    let axis = match request.axis {
        Axis::Source => DescribeAxis::Source,
        Axis::Target => DescribeAxis::Target,
    };
    match describe_adapter(request.component, axis, request.adapter_id) {
        Ok(value) => project(value, request),
        Err(DescribeFailure::AxisMismatch { expected, found }) => Err(Error::Diag {
            code: "adapter-axis-mismatch",
            detail: format!(
                "adapter component {} exports `{found}`, but this binding expects `{expected}`",
                request.component.display(),
            ),
        }),
        Err(failure @ DescribeFailure::Other(_)) => Err(Error::Diag {
            code: "adapter-describe-failed",
            detail: format!(
                "describe dispatch for `{}` ({}) failed: {failure}",
                request.adapter_id,
                request.component.display(),
            ),
        }),
    }
}

/// Project the string-typed runtime answer onto the typed workflow
/// shape, parsing platform tokens through the closed [`Platform`] enum.
fn project(value: DescribeValue, request: &DescribeRequest<'_>) -> Result<DescribeAnswer, Error> {
    let platforms = value
        .platforms
        .map(|capability| {
            Ok::<_, Error>(PlatformsCapability {
                required: capability.required,
                allowed: parse_platforms(&capability.allowed, request)?,
                default: parse_platforms(&capability.default, request)?,
            })
        })
        .transpose()?;
    Ok(DescribeAnswer {
        specify_floor: value.specify_floor,
        inputs: value
            .inputs
            .into_iter()
            .map(|input| BuildInputDeclaration {
                path: input.path,
                required: input.required,
            })
            .collect(),
        platforms,
    })
}

fn parse_platforms(
    tokens: &[String], request: &DescribeRequest<'_>,
) -> Result<Vec<Platform>, Error> {
    tokens
        .iter()
        .map(|token| {
            Platform::from_str(token).map_err(|_unknown_platform| Error::Diag {
                code: "adapter-describe-failed",
                detail: format!(
                    "describe answer for `{}` ({}) names an unknown platform `{token}`",
                    request.adapter_id,
                    request.component.display(),
                ),
            })
        })
        .collect()
}
