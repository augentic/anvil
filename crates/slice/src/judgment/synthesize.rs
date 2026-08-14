//! The slice synthesis judgment leg.
//!
//! The deterministic tail runs inside the shared repair loop, so an
//! answer the projection kernel would reject is repaired in-loop.

use std::collections::BTreeMap;

use artifacts::evidence::{AuthorityClass, ClaimKind};
use error::Error;
use omnia_guest::Model;
use project::plan::FocusParent;
use project::profile::{Gate, Profile};

use super::{prose, repaired};
use crate::synthesis::wire::{SYNTHESIS_VERSION, SynthesisKind};
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
    /// Bound target profile the assessment is scored against.
    pub profile: Profile,
    /// This leaf's terminal `(source, lead)` pairs.
    pub terminals: Vec<FocusParent>,
}

/// A validated synthesis answer: the parsed response plus, on
/// `proceed`, the kernel-projected model.
#[derive(Debug)]
pub struct Synthesized {
    /// The agent's parsed response envelope.
    pub response: SynthesisResponse,
    /// The kernel-projected model — present only on `proceed`.
    pub projected: Option<SliceModel>,
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
    repaired(model, &system, user, "synthesis", &schema, |answer| check(answer, kernel)).await
}

fn check(answer: &str, kernel: &Kernel<'_>) -> Result<Synthesized, Error> {
    let response: SynthesisResponse = serde_saphyr::from_str(answer).map_err(|err| {
        Error::validation_failed(
            "slice-synthesize-response-parse",
            "the synthesis answer deserialises as a synthesis response",
            format!("failed to parse synthesis response: {err}"),
        )
    })?;
    if response.version != SYNTHESIS_VERSION {
        return Err(Error::validation_failed(
            "slice-synthesize-version",
            "the synthesis answer carries the current wire version",
            format!("synthesis version `{}` is not `{SYNTHESIS_VERSION}`", response.version),
        ));
    }
    kernel.profile.score(&response.assessment)?;
    match response.kind {
        SynthesisKind::Proceed => proceed(response, kernel),
        SynthesisKind::BoundaryEscalation => escalate(response, kernel),
    }
}

fn proceed(response: SynthesisResponse, kernel: &Kernel<'_>) -> Result<Synthesized, Error> {
    let model = response.model.clone().ok_or_else(|| {
        Error::validation_failed(
            "slice-synthesize-proceed-incomplete",
            "a proceed answer carries the structured model",
            "proceed omitted `model`",
        )
    })?;
    if response.artifacts.is_none() {
        return Err(Error::validation_failed(
            "slice-synthesize-proceed-incomplete",
            "a proceed answer carries the prose artifacts",
            "proceed omitted `artifacts`",
        ));
    }
    let projected = project(
        model,
        kernel.header.clone(),
        kernel.authority,
        kernel.overrides,
        kernel.evidence_claims,
        kernel.baseline_index,
    )?;
    Ok(Synthesized {
        response,
        projected: Some(projected),
    })
}

fn escalate(response: SynthesisResponse, kernel: &Kernel<'_>) -> Result<Synthesized, Error> {
    if !kernel.profile.exceeds(&response.assessment, Gate::SliceSplit)? {
        return Err(Error::validation_failed(
            "slice-synthesize-escalation-below-threshold",
            "boundary-escalation requires a score above the slice-split threshold",
            "assessment does not exceed the bound profile's slice-split threshold",
        ));
    }
    if response.affected.is_empty() {
        return Err(Error::validation_failed(
            "slice-synthesize-escalation-empty",
            "boundary-escalation names at least one terminal pair",
            "affected is empty",
        ));
    }
    let rationale = response.rationale.as_deref().map_or("", str::trim);
    if rationale.is_empty() {
        return Err(Error::validation_failed(
            "slice-synthesize-escalation-incomplete",
            "boundary-escalation carries a typed rationale",
            "rationale is empty",
        ));
    }
    for parent in &response.affected {
        if !kernel
            .terminals
            .iter()
            .any(|terminal| terminal.source == parent.source && terminal.lead == parent.lead)
        {
            return Err(Error::validation_failed(
                "slice-synthesize-escalation-unknown-terminal",
                "affected pairs are this leaf's bound terminals",
                format!("`{}` / `{}` is not a terminal of this leaf", parent.source, parent.lead),
            ));
        }
    }
    Ok(Synthesized {
        response,
        projected: None,
    })
}
