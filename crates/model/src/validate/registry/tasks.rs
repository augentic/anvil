//! Tasks-brief rules.

use crate::validate::{BriefContext, Classification, Rule, RuleOutcome, primitives};

fn tasks_use_checkbox_format(ctx: &BriefContext<'_>) -> RuleOutcome {
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

fn tasks_grouped_under_headings(ctx: &BriefContext<'_>) -> RuleOutcome {
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
        classification: Classification::Structural,
        check: Some(tasks_use_checkbox_format),
    },
    Rule {
        id: "tasks.grouped-under-headings",
        description: "Tasks grouped under `## ` headings",
        classification: Classification::Structural,
        check: Some(tasks_grouped_under_headings),
    },
];
