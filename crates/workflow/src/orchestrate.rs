//! Guest workflow orchestrators: one entry point per phase, each
//! dispatching across the [`crate::seam`] capability traits and owning
//! its validate-before-visible tail.
//!
//! Survey fan-out feeds `Discovery::merge_survey`, extract persists
//! schema-gated Evidence, build runs the finalize tail (report schema
//! gate, `enforce_report_*`, the `built` transition, the
//! `slice.build.*` bracket), and merge is deterministic-only. `specify
//! source survey/extract`, `specify slice build`, and `specify slice
//! merge run` route here through the guest.
//!
//! Time is injected: every orchestrator takes the caller's `now`
//! (architecture.md §"Time injection"); library code never reads the
//! clock.

mod author;
mod execute;
mod merge;
mod refine;
mod routing;
mod source;
mod synthesize;
mod target;

use error::Error;

pub use self::author::author;
pub use self::execute::{ExecuteOutcome, execute};
pub use self::merge::merge;
pub use self::refine::{refine, refine_breakout};
pub use self::source::{SurveyedSource, extract, survey, survey_all};
pub use self::synthesize::synthesize;
pub use self::target::build;
use crate::seam;

/// The borrowed capability bundle one orchestration run dispatches
/// across: model judgment, source-axis seam, target-axis seam, and
/// adapter resolver.
///
/// The four capabilities stay independent type parameters so tests
/// bind independent mocks per seam; the shipped provider satisfies
/// all four at once, so handlers bundle it with
/// [`Capabilities::provider`]. Phases that use a subset simply leave
/// the unused parameter unbounded (plan authoring never dispatches
/// the target seam).
#[derive(Debug)]
pub struct Capabilities<'a, P, S, T, R> {
    /// Judgment-leg model dispatch.
    pub model: &'a P,
    /// Source-axis seam (survey / extract).
    pub sources: &'a S,
    /// Target-axis seam (guidance / build).
    pub targets: &'a T,
    /// Adapter resolver.
    pub resolver: &'a R,
}

impl<'a, Provider> Capabilities<'a, Provider, Provider, Provider, Provider> {
    /// Bundle one provider that carries every capability — the
    /// handler-side constructor over `context.provider`.
    pub const fn provider(provider: &'a Provider) -> Self {
        Self {
            model: provider,
            sources: provider,
            targets: provider,
            resolver: provider,
        }
    }
}

impl<'a, P, S, T, R> Capabilities<'a, P, S, T, R> {
    /// Drop the target seam for phases that never dispatch it
    /// ([`author`] surveys and reconciles but builds nothing).
    #[must_use]
    pub const fn sans_targets(self) -> Capabilities<'a, P, S, (), R> {
        Capabilities {
            model: self.model,
            sources: self.sources,
            targets: &(),
            resolver: self.resolver,
        }
    }
}

// Manual `Copy`/`Clone`: the bundle is four shared borrows, copyable
// regardless of whether the capability types themselves are.
impl<P, S, T, R> Clone for Capabilities<'_, P, S, T, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, S, T, R> Copy for Capabilities<'_, P, S, T, R> {}

/// Map a seam dispatch failure onto the wire contract.
///
/// `operation` is the seam method (`survey`, `extract`, `guidance`,
/// `build`); `id` is the routed adapter id (e.g. `source:typescript`).
fn seam_failure(operation: &'static str, id: &str, err: &seam::Error) -> Error {
    Error::Diag {
        code: "seam-dispatch-failed",
        detail: format!("seam `{operation}` dispatch to `{id}` failed: {err}"),
    }
}

/// The plan-bound adapter id routing a source dispatch
/// (`source:<adapter>`).
fn source_adapter_id(adapter: &str) -> String {
    format!("source:{adapter}")
}

/// The plan-bound adapter id routing a target dispatch
/// (`target:<name>`).
fn target_adapter_id(name: &str) -> String {
    format!("target:{name}")
}
