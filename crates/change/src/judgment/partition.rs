//! One schema-gated `split | leaf` partition of an open domain.

use error::Error;
use omnia_guest::Model;
use project::judgment::{render_json, repaired};
use project::plan::PartitionResponse;

use crate::judgment::prose;

/// Run the partition judgment over one open domain.
///
/// `check` is the tentative-tree validation (typically apply +
/// [`project::plan::Decomposition::check`]), so an invalid cut is
/// repaired in-loop rather than after the call.
///
/// # Errors
///
/// The mapped model failure, or the last schema / parse / check
/// failure once the repair budget is exhausted.
pub async fn partition<P, F>(
    model: &P, request: &serde_json::Value, mut check: F,
) -> Result<PartitionResponse, Error>
where
    P: Model,
    F: FnMut(&PartitionResponse) -> Result<(), Error>,
{
    let schema = project::answers::render(&project::answers::partition());
    let user = format!(
        "## Partition request\n\n```json\n{}\n```",
        render_json(request, "partition request")?
    );
    repaired(
        model,
        prose::partition(),
        user,
        "partition",
        request.get("domain").and_then(serde_json::Value::as_str),
        &schema,
        project::judgment::Lent::default(),
        |answer| {
            let response: PartitionResponse = serde_json::from_str(answer).map_err(|err| {
                Error::validation_failed(
                    "plan-partition-response-parse",
                    "the partition answer deserialises as a response envelope",
                    format!("failed to parse partition envelope: {err}"),
                )
            })?;
            if response.version != project::plan::PARTITION_VERSION {
                return Err(Error::validation_failed(
                    "plan-partition-version",
                    "the partition answer carries the current wire version",
                    format!(
                        "partition version `{}` is not `{}`",
                        response.version,
                        project::plan::PARTITION_VERSION
                    ),
                ));
            }
            check(&response)?;
            Ok(response)
        },
    )
    .await
}
