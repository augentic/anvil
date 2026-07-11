//! `plan next` — the only writer of per-entry `in-progress`.

use std::io::Write;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use crate::adapter::Resolver;
use crate::change::{NextBody, NextReason, claim_next};
use crate::handler::{Anchor, Ctx, Render};

/// Wire input for `plan next` (no fields).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct NextInput {}

/// `specify plan next`.
///
/// Return the active in-progress entry, or transition the next
/// eligible `Pending` entry to `InProgress` and return it. The only
/// writer of per-entry `in-progress` per workflow §CLI surface. The
/// projection, persist decision, and conditional `plan.entry.advanced`
/// event all live in the shared [`claim_next`] kernel.
#[derive(Clone, Copy, Debug)]
pub struct Next;

impl<P: Anchor + Resolver> Operation<P> for Next {
    type Error = crate::handler::Error;
    type Input = NextInput;
    type Output = NextBody;

    async fn call(
        _input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        Ok(claim_next(context.provider, cx.layout(), cx.now(), &cx.config)?)
    }
}

impl Render for NextBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if let Some(active) = &self.active {
            writeln!(w, "Active change in progress: {active}")
        } else if let Some(name) = &self.next {
            writeln!(w, "{name}")
        } else if self.reason == Some(NextReason::Drained) {
            writeln!(w, "Plan drained — no per-entry pending or in-progress remains.")
        } else {
            writeln!(
                w,
                "No eligible changes \u{2014} remaining entries are waiting on unmet dependencies."
            )
        }
    }
}
