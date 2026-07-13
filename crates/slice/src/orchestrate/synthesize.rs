//! Seam-guidance plumbing for the synthesis judgment leg.

use error::Error;
use omnia_guest::Model;
use project::registry::topology::{Decision, Surface};
use project::seam::TargetSeam;

use super::{seam_failure, target_id};
use crate::judgment::synthesize::{Kernel, Synthesized};
use crate::{DomainDetail, SourceInput, inputs, judgment};

/// The caller-assembled inputs to [`synthesize`], minus the guidance
/// brief the orchestrator fetches through the seam: the per-source
/// Evidence contributions and the baseline context.
#[derive(Debug)]
pub struct SynthesizeRequest<'a> {
    /// Slice name the leg synthesises.
    pub slice: &'a str,
    /// Bound target adapter name (bare, e.g. `omnia`).
    pub target: &'a str,
    /// One entry per bound source, carrying its inline `lead` and
    /// `claims`.
    pub sources: &'a [SourceInput],
    /// The slice's bound project baseline surface.
    pub baseline: &'a [Surface],
    /// Per-domain baseline `REQ` id facts.
    pub baseline_detail: &'a [DomainDetail],
    /// The bound project's accepted baseline Decision Records.
    pub baseline_decisions: &'a [Decision],
}

/// Run the synthesis judgment leg with the guidance brief read
/// through `seam.guidance(target)`.
///
/// Assembles the inputs envelope (`inputs`) and runs
/// the `judgment::synthesize::synthesize` leg; per the judgment-leg
/// contract, the caller still owns staging and persisting the
/// synthesized artifacts and the `slice.synthesize.*` journal bracket
/// (the refine loop's concern).
///
/// # Errors
///
/// - `seam-dispatch-failed` when the guidance dispatch fails.
/// - propagates the judgment leg's model / schema / kernel failures.
pub async fn synthesize<P: Model, T: TargetSeam>(
    model: &P, seam: &T, request: &SynthesizeRequest<'_>, kernel: &Kernel<'_>,
) -> Result<Synthesized, Error> {
    let id = target_id(request.target);
    let guidance =
        seam.guidance(id.clone()).await.map_err(|err| seam_failure("guidance", &id, &err))?;
    let synthesis_inputs = inputs(
        request.slice,
        request.sources,
        &guidance,
        request.baseline,
        request.baseline_detail,
        request.baseline_decisions,
    );
    judgment::synthesize::synthesize(model, &synthesis_inputs, kernel).await
}
