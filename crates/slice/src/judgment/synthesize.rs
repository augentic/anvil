//! The slice synthesis judgment leg.
//!
//! The deterministic tail runs inside the shared repair loop, so an
//! answer the projection kernel would reject is repaired in-loop.

use std::collections::BTreeMap;

use artifacts::evidence::{AuthorityClass, ClaimKind};
use error::Error;
use omnia_guest::Model;
use omnia_guest::model::McpGrant;
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
/// `proceed`, the staged artifacts and the kernel-projected model.
#[derive(Debug)]
pub struct Synthesized {
    /// The agent's parsed response envelope.
    pub response: SynthesisResponse,
    /// The kernel-projected model — present only on `proceed`.
    pub projected: Option<SliceModel>,
    /// The prose artifacts read from the staged tree — present only
    /// on `proceed` (RFC-96 D10).
    pub artifacts: Option<crate::synthesis::wire::SynthesisArtifacts>,
}

/// Run the slice synthesis judgment leg over an assembled inputs
/// envelope.
///
/// With a shelf URL (RFC-96 D9) the prompt keeps only the measured
/// inline minimum and the shelf is granted as an MCP server; without
/// one the full playbook inlines as before. `stage` is the lent
/// writable staged tree (RFC-96 D10): the agent writes the bundle
/// there, the deterministic tail validates the full tree, and a tail
/// failure re-prompts the same agent over the same stage.
///
/// # Errors
///
/// The mapped model failure, or the last schema / parse / staged-tree
/// / kernel failure once the repair budget is exhausted.
pub async fn synthesize<P: Model>(
    model: &P, inputs: &SynthesisInputs, kernel: &Kernel<'_>, shelf: Option<String>, stage: &str,
) -> Result<Synthesized, Error> {
    let schema = project::answers::render(&crate::answers::synthesis());
    let system = prose::synthesize_system(shelf.as_deref());
    let grants: Vec<McpGrant> = shelf
        .map(|url| McpGrant {
            name: crate::shelf::SERVER.to_string(),
            tools: Vec::new(),
            url,
        })
        .into_iter()
        .collect();
    let user = format!(
        "## Synthesis inputs\n\n```json\n{}\n```",
        super::render_json(inputs, "synthesis inputs")?
    );
    let root = std::path::PathBuf::from(stage);
    let lent = project::judgment::Lent {
        grants,
        workspace: Some(stage.to_string()),
    };
    let slice = kernel.header.slice.clone();
    repaired(model, &system, user, "synthesis", Some(slice.as_str()), &schema, lent, |answer| {
        check(answer, kernel, &root)
    })
    .await
}

fn check(answer: &str, kernel: &Kernel<'_>, stage: &std::path::Path) -> Result<Synthesized, Error> {
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
        SynthesisKind::Proceed => proceed(response, kernel, stage),
        SynthesisKind::BoundaryEscalation => escalate(response, kernel),
    }
}

/// Validate a `proceed` answer's staged tree (RFC-96 D10): read the
/// full bundle the agent wrote, then run the kernel projection over
/// the staged model — any miss repairs in-loop over the same stage.
fn proceed(
    response: SynthesisResponse, kernel: &Kernel<'_>, stage: &std::path::Path,
) -> Result<Synthesized, Error> {
    let bundle = crate::synthesis::stage::read(stage)?;
    let projected = project(
        bundle.model,
        kernel.header.clone(),
        kernel.authority,
        kernel.overrides,
        kernel.evidence_claims,
        kernel.baseline_index,
    )?;
    Ok(Synthesized {
        response,
        projected: Some(projected),
        artifacts: Some(bundle.artifacts),
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
        artifacts: None,
    })
}
