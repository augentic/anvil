//! `plan status` — the read-only next-action projection.

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::handler::{Anchor, Ctx};
use project::plan::{Plan, StatusBody, plan_status_body};
use serde::{Deserialize, Serialize};

use super::require_file;

/// Wire input for `plan status` (no fields).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct StatusInput {}

/// `specify plan status`.
///
/// Read-only projection of the plan's execution state into a
/// deterministic `next-action`. All projection logic lives in
/// `workflow` (`plan_status_body`); the handler loads the plan and
/// returns the body. No journal emit, no writes.
#[derive(Clone, Copy, Debug)]
pub struct Status;

impl<P: Anchor> Operation<P> for Status {
    type Error = project::handler::Error;
    type Input = StatusInput;
    type Output = StatusBody;

    async fn call(
        _input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let plan_path = require_file(&cx)?;
        let plan = Plan::load(&plan_path)?;
        let body = plan_status_body(&plan, cx.layout())?;
        Ok(body)
    }
}
