//! `plan advance` — the only writer of per-entry `in-progress`.

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::adapter::Resolver;
use project::handler::{Anchor, Ctx};
use project::plan::{AdvanceBody, advance_next};
use serde::{Deserialize, Serialize};

/// Wire input for `plan advance` (no fields).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct AdvanceInput {}

/// `emery plan advance`.
///
/// Return the active in-progress entry, or advance the next eligible
/// `Pending` entry to `InProgress` and return it. The only writer of
/// per-entry `in-progress` per workflow §CLI surface. The projection,
/// persist decision, and conditional `plan.entry.advanced` event all
/// live in the shared `advance_next` kernel.
#[derive(Clone, Copy, Debug)]
pub struct Advance;

impl<P: Anchor + Resolver> Operation<P> for Advance {
    type Error = project::handler::Error;
    type Input = AdvanceInput;
    type Output = AdvanceBody;

    async fn call(
        _input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        Ok(advance_next(context.provider, &cx.paths, cx.now(), &cx.config)?)
    }
}
