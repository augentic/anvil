//! `plan gaps` — the read-only typed gap inventory projection.

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::handler::{Anchor, Ctx};
use project::plan::{GapsBody, Plan, collect_events, plan_gaps_body};
use serde::{Deserialize, Serialize};

use super::require_file;

/// Wire input for `plan gaps` (no fields).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct GapsInput {}

/// `emery plan gaps`.
///
/// Read-only projection of in-scope typed requirement statuses
/// (`unknown` / `conflict` / `divergence`) with a presentation-only
/// shared-lead rollup (RFC-86 Gaps / D18 / D19 / D24). Writes nothing.
#[derive(Clone, Copy, Debug)]
pub struct Gaps;

impl<P: Anchor> Operation<P> for Gaps {
    type Error = project::handler::Error;
    type Input = GapsInput;
    type Output = GapsBody;

    async fn call(
        _input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let plan_path = require_file(&cx)?;
        let plan = Plan::load(&plan_path)?;
        project::plan::ensure_authored(cx.layout(), &plan)?;
        let events = collect_events(cx.layout())?;
        let body = plan_gaps_body(&plan, cx.layout(), &events)?;
        Ok(body)
    }
}
