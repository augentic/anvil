//! The slice synthesis judgment leg.
//!
//! The deterministic tail runs inside the shared repair loop, so an
//! answer the projection kernel would reject is repaired in-loop.

use std::collections::BTreeMap;

use artifacts::evidence::{AuthorityClass, ClaimKind};
use error::Error;
use omnia_guest::Model;

use super::{prose, repaired};
use crate::{
    BaselineIndex, ProjectionHeader, SliceModel, SynthesisInputs, SynthesisResponse, project,
};

/// The deterministic projection context the kernel runs against inside
/// the repair loop, distilled from the plan entry and on-disk Evidence.
#[derive(Debug)]
pub struct Kernel<'a> {
    /// The `version` / `slice` / `project` header stamped on projection.
    pub header: ProjectionHeader,
    /// Per-source document-level `authority` map.
    pub authority: &'a BTreeMap<String, AuthorityClass>,
    /// Per-slice per-kind authority overrides.
    pub overrides: &'a BTreeMap<ClaimKind, String>,
    /// `(source, id) → kind` claim anchor index.
    pub evidence_claims: &'a BTreeMap<(String, String), ClaimKind>,
    /// Baseline requirement-id index for baseline-aware id allocation.
    pub baseline_index: &'a BaselineIndex,
}

/// A validated synthesis answer: the parsed response plus the
/// kernel-projected model that already passed the projection inside the
/// repair loop.
#[derive(Debug)]
pub struct Synthesized {
    /// The agent's parsed response envelope (artifacts + raw model).
    pub response: SynthesisResponse,
    /// The kernel-projected model — ids, status, winners, and rendered
    /// sources derived; header stamped.
    pub projected: SliceModel,
}

/// Run the slice synthesis judgment leg over an assembled inputs
/// envelope.
///
/// # Errors
///
/// The mapped model failure, or the last schema / parse / kernel
/// failure once the repair budget is exhausted.
pub async fn synthesize<P: Model>(
    model: &P, inputs: &SynthesisInputs, kernel: &Kernel<'_>,
) -> Result<Synthesized, Error> {
    let schema = project::answers::render(&crate::answers::synthesis());
    let system = prose::synthesize_system();
    let user = format!(
        "## Synthesis inputs\n\n```json\n{}\n```",
        super::render_json(inputs, "synthesis inputs")?
    );
    repaired(model, &system, user, "synthesis", &schema, |answer| {
        let response: SynthesisResponse = serde_saphyr::from_str(answer).map_err(|err| {
            Error::validation_failed(
                "slice-synthesize-response-parse",
                "the synthesis answer deserialises as a synthesis response",
                format!("failed to parse synthesis response: {err}"),
            )
        })?;
        let projected = project(
            response.model.clone(),
            kernel.header.clone(),
            kernel.authority,
            kernel.overrides,
            kernel.evidence_claims,
            kernel.baseline_index,
        )?;
        Ok(Synthesized { response, projected })
    })
    .await
}
