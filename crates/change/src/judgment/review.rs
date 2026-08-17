//! One schema-gated boundary review after a provisional score exceeds
//! the slice-split threshold.

use error::Error;
use omnia_guest::Model;
use project::judgment::{render_json, repaired};
use project::plan::BoundaryReview;

use crate::judgment::prose;

/// Run the bounded boundary-review judgment.
///
/// # Errors
///
/// The mapped model failure, or the last schema / parse / check
/// failure once the repair budget is exhausted.
pub async fn review<P, F>(
    model: &P, request: &serde_json::Value, mut check: F,
) -> Result<BoundaryReview, Error>
where
    P: Model,
    F: FnMut(&BoundaryReview) -> Result<(), Error>,
{
    let schema = project::answers::render(&project::answers::boundary_review());
    let user = format!(
        "## Boundary review\n\n```json\n{}\n```",
        render_json(request, "boundary-review request")?
    );
    repaired(
        model,
        prose::review(),
        user,
        "boundary-review",
        request.get("domain").and_then(serde_json::Value::as_str),
        &schema,
        project::judgment::Lent::default(),
        |answer| {
            let response: BoundaryReview = serde_json::from_str(answer).map_err(|err| {
                Error::validation_failed(
                    "plan-review-response-parse",
                    "the boundary-review answer deserialises as a response envelope",
                    format!("failed to parse boundary-review envelope: {err}"),
                )
            })?;
            if response.version != project::plan::PARTITION_VERSION {
                return Err(Error::validation_failed(
                    "plan-review-version",
                    "the boundary-review answer carries the current wire version",
                    format!(
                        "boundary-review version `{}` is not `{}`",
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
