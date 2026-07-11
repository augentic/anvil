//! `plan status` — the read-only next-action projection.

use std::io::Write;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use super::require_file;
use crate::change::{NextActionKind, Plan, StatusBody, drained_line, plan_status_body};
use crate::handler::{Anchor, Ctx, Render};

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
    type Error = crate::handler::Error;
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

/// Text rendering for `plan status`: a plan/entries header, then the
/// next-action line. Stops render the stop-conditions block shape
/// (`stop: <reason>` + indented context + `hint:`); drained renders
/// the literal stop-conditions drained string.
impl Render for StatusBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "plan: {} ({})", self.plan, self.lifecycle)?;
        writeln!(
            w,
            "entries: {} done / {} in-progress / {} pending",
            self.counts.done, self.counts.in_progress, self.counts.pending
        )?;
        match (self.action, &self.stop) {
            (NextActionKind::Drained, _) => writeln!(w, "{}", drained_line(&self.plan))?,
            (NextActionKind::Stop, Some(stop)) => {
                writeln!(w, "stop: {}", stop.reason)?;
                if let Some(slice) = &self.slice {
                    writeln!(w, "  slice: {slice}")?;
                    writeln!(w, "  project: {}", self.project.as_deref().unwrap_or("-"))?;
                }
                if let Some(detail) = &stop.detail {
                    writeln!(w, "  detail: {detail}")?;
                }
                writeln!(w, "hint: {}", stop.hint)?;
            }
            _ => writeln!(w, "next-action: {}", self.next_action)?,
        }
        if self.action != NextActionKind::Drained
            && let Some(resume) = &self.resume
        {
            writeln!(w, "resume: {resume}")?;
        }
        Ok(())
    }
}
