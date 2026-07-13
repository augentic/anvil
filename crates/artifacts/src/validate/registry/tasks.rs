//! Tasks-brief rules.

use crate::validate::{BriefContext, Rule, RuleOutcome, primitives};

fn use_checkbox_format(ctx: &BriefContext<'_>) -> RuleOutcome {
    let Some(tasks) = ctx.tasks else {
        return RuleOutcome::Fail {
            detail: "tasks were not parsed".to_string(),
        };
    };
    if primitives::all_tasks_use_checkbox(tasks, ctx.content) {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: "found `- …` bullets that do not match the `- [ ] X.Y` checkbox format"
                .to_string(),
        }
    }
}

fn grouped_under_headings(ctx: &BriefContext<'_>) -> RuleOutcome {
    let Some(tasks) = ctx.tasks else {
        return RuleOutcome::Fail {
            detail: "tasks were not parsed".to_string(),
        };
    };
    if primitives::tasks_grouped_under_headings(tasks) {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: "one or more tasks appear before any `## ` heading".to_string(),
        }
    }
}

pub(super) const TASKS_RULES: &[Rule] = &[
    Rule {
        id: "tasks.use-checkbox-format",
        description: "All tasks use `- [ ] X.Y` checkbox format",
        check: use_checkbox_format,
    },
    Rule {
        id: "tasks.grouped-under-headings",
        description: "Tasks grouped under `## ` headings",
        check: grouped_under_headings,
    },
];
