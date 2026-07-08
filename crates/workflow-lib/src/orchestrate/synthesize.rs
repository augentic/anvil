//! Seam-guidance plumbing for the synthesis judgment leg.

use specify_error::Error;
use specify_guest_model::Model;

use super::{seam_failure, target_adapter_id};
use crate::judgment;
use crate::judgment::synthesize::{Kernel, Synthesized};
use crate::registry::topology::Surface;
use crate::seam::TargetSeam;
use crate::slice::{BaselineDomainDetail, SynthesisSourceInput, build_synthesis_inputs};

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
    pub sources: &'a [SynthesisSourceInput],
    /// The slice's bound project baseline surface.
    pub baseline: &'a [Surface],
    /// Per-domain baseline `REQ` id facts.
    pub baseline_detail: &'a [BaselineDomainDetail],
}

/// Run the synthesis judgment leg with the guidance brief read
/// through `seam.guidance(target)`.
///
/// Assembles the inputs envelope ([`build_synthesis_inputs`]) and runs
/// the [`judgment::synthesize::synthesize`] leg; per the judgment-leg
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
    let id = target_adapter_id(request.target);
    let guidance =
        seam.guidance(id.clone()).await.map_err(|err| seam_failure("guidance", &id, &err))?;
    let inputs = build_synthesis_inputs(
        request.slice,
        request.sources,
        &guidance,
        request.baseline,
        request.baseline_detail,
    );
    judgment::synthesize::synthesize(model, &inputs, kernel).await
}
