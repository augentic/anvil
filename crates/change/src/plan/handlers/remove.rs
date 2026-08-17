//! `emery plan remove` — drop one pending plan entry while the plan
//! is still replaceable (pre-execution curation).

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::config::{Mutation, with_state};
use project::handler::{Anchor, Ctx};
use project::plan::{Plan, collect_events, project_ladders, proposal, publication};
use serde::{Deserialize, Serialize};

use super::entry::{Action, EntryBody};
use super::{plan_ref, require_file};

/// Wire input for `plan remove`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RemoveInput {
    /// Kebab-case entry name to remove.
    pub name: String,
}

/// `emery plan remove <name>`.
#[derive(Clone, Copy, Debug)]
pub struct Remove;

impl<P: Anchor> Operation<P> for Remove {
    type Error = project::handler::Error;
    type Input = RemoveInput;
    type Output = EntryBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let name = input.name;
        let plan_path = require_file(&cx)?;
        // A bound-not-authored home may hold a partial decomposition
        // tree; topology edits wait for `plan author` to finish.
        project::plan::ensure_authored(cx.layout(), &Plan::load(&plan_path)?)?;
        let events = collect_events(cx.layout())?;
        let layout = cx.layout();
        let body = with_state::<Plan, _, _>(layout, "plan.yaml", move |plan| {
            let removed = plan
                .entries
                .iter()
                .find(|e| e.name == name)
                .cloned()
                .ok_or_else(|| plan.entry_not_found(&name))?;
            // Materialized publication members lock their target's
            // topology until archive (RFC-95 D11).
            if publication::locked_targets(plan, &events).contains(&removed.target) {
                return Err(publication::locked_err(&removed.target));
            }
            let ladders = project_ladders(plan, &events);
            if proposal::has_tree(layout) && Plan::is_replaceable(&ladders) {
                proposal::remove(layout, plan, &name)?;
            } else {
                plan.remove(&name, &ladders)?;
            }
            Ok(Mutation::changed(EntryBody {
                plan: plan_ref(plan, &plan_path),
                action: Action::Remove,
                entry: removed,
            }))
        })?;

        Ok(body)
    }
}
