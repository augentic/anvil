//! Seam-guidance plumbing for the synthesis judgment leg.

use diagnostics::digest::sha256_hex;
use error::Error;
use omnia_guest::Model;
use project::adapter::{Axis, RoutedId};
use project::identity::{Decision, Surface};
use project::seam::Target;
use project::snapshot::SnapshotId;

use super::seam_failure;
use crate::judgment::synthesize::{Kernel, Synthesized};
use crate::{DependencyContext, DomainDetail, SourceInput, inputs, judgment};

/// The caller-assembled inputs to [`synthesize`], minus the guidance
/// brief the orchestrator fetches through the seam: the per-source
/// Evidence contributions, the baseline context, and the ordered
/// predecessor refinement context.
#[derive(Debug)]
pub struct SynthesizeRequest<'a> {
    /// Slice name the leg synthesises.
    pub slice: &'a str,
    /// The slice's recorded target value (e.g. `omnia@1.0.0`); the
    /// guidance dispatch routes the exact recorded identity.
    pub target: &'a str,
    /// One entry per bound source, carrying its `lead` and the
    /// project-relative `evidence-path` to its Evidence document.
    pub sources: &'a [SourceInput],
    /// The slice's bound project baseline surface.
    pub baseline: &'a [Surface],
    /// Per-domain baseline `REQ` id facts.
    pub baseline_detail: &'a [DomainDetail],
    /// The bound project's accepted baseline Decision Records.
    pub baseline_decisions: &'a [Decision],
    /// Ordered predecessor refinement context (RFC-91 D3).
    pub dependencies: &'a [DependencyContext],
}

/// Run the synthesis judgment leg with the guidance brief read
/// through `seam.guidance(target)`.
///
/// Assembles the inputs envelope and runs the judgment leg; the
/// caller still owns persisting the artifacts and the
/// `slice.synthesize.*` journal bracket. Returns the validated answer
/// plus the content digest of the guidance text consumed — the
/// `target-guidance` identity the refinement manifest records.
///
/// # Errors
///
/// - `seam-dispatch-failed` when the guidance dispatch fails.
/// - propagates the judgment leg's model / schema / kernel failures.
pub async fn synthesize<P: Model, T: Target>(
    model: &P, seam: &T, request: &SynthesizeRequest<'_>, kernel: &Kernel<'_>,
) -> Result<(Synthesized, SnapshotId), Error> {
    let id = RoutedId::recorded(Axis::Target, request.target).to_string();
    let guidance =
        seam.guidance(id.clone()).await.map_err(|err| seam_failure("guidance", &id, &err))?;
    let guidance_digest = SnapshotId::from_digest(&sha256_hex(guidance.as_bytes()));
    let synthesis_inputs = inputs(
        request.slice,
        request.sources,
        &guidance,
        request.baseline,
        request.baseline_detail,
        request.baseline_decisions,
        request.dependencies,
    );
    let synthesized = judgment::synthesize::synthesize(model, &synthesis_inputs, kernel).await?;
    Ok((synthesized, guidance_digest))
}
