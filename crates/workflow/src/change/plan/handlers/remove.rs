//! `specify plan remove` — drop one pending plan entry while the plan
//! is still replaceable (Gate 1 curation).

use omnia_guest::api::{Context, Handler, Reply};
use serde::{Deserialize, Serialize};

use super::entry::{Action, EntryBody};
use super::{plan_ref, require_file};
use crate::change::Plan;
use crate::config::with_state;
use crate::handler::{Anchor, Ctx, Out};
use crate::schema::validate_plan;

/// Wire input for `plan remove`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RemoveInput {
    /// Kebab-case entry name to remove.
    pub name: String,
}

/// `specify plan remove <name>`.
#[derive(Debug)]
pub struct Remove {
    input: RemoveInput,
}

impl<P: Anchor> Handler<P> for Remove {
    type Error = crate::handler::Error;
    type Input = RemoveInput;
    type Output = Out<EntryBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let name = self.input.name;
        let plan_path = require_file(&cx)?;
        let body = with_state::<Plan, _, _>(cx.layout(), "plan.yaml", move |plan| {
            let removed =
                plan.entries.iter().find(|e| e.name == name).cloned().ok_or_else(|| {
                    error::Error::Diag {
                        code: "plan-entry-not-found",
                        detail: format!("no slice named '{name}' in plan"),
                    }
                })?;
            plan.remove(&name)?;
            validate_plan(plan)?;
            Ok(EntryBody {
                plan: plan_ref(plan, &plan_path),
                action: Action::Remove,
                entry: removed,
            })
        })?;

        Ok(Reply::ok(Out(body)))
    }
}
