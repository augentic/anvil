//! `plan advance` — claim the next eligible slice (RFC-86 D7).

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
/// Claim the next eligible pending entry (`slice.claimed` +
/// `plan.entry.advanced`), or — when nothing pending is eligible —
/// return an existing in-progress entry for mid-slice resume.
/// Concurrent in-progress entries are legal (RFC-86 D23); exclusivity
/// is per-slice claim only. Does not rewrite stored `Entry.status`.
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
