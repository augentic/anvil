//! `specify plan remove` handler — drop one pending plan entry while
//! the plan is still replaceable (Gate 1 curation).

use specify_error::Result;
use specify_workflow::change::Plan;
use specify_workflow::config::with_state;
use specify_workflow::schema::validate_plan;

use super::entry::{Action, EntryBody, write_entry_text};
use super::{plan_ref, require_file};
use crate::runtime::context::Ctx;

pub(super) fn remove(ctx: &Ctx, name: String) -> Result<()> {
    let plan_path = require_file(ctx)?;
    let body = with_state::<Plan, _, _>(ctx.layout(), "plan.yaml", move |plan| {
        let removed = plan.entries.iter().find(|e| e.name == name).cloned().ok_or_else(|| {
            specify_error::Error::Diag {
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

    ctx.write(&body, write_entry_text)?;
    Ok(())
}
