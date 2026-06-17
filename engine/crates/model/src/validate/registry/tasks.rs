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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{tasks_grouped_under_headings, tasks_use_checkbox_format};
    use crate::task::{Progress, parse_tasks};
    use crate::validate::{BriefContext, RuleOutcome};

    fn ctx<'a>(content: &'a str, tasks: Option<&'a Progress>) -> BriefContext<'a> {
        BriefContext {
            id: "tasks",
            content,
            parsed_spec: None,
            tasks,
            slice_dir: Path::new("."),
            specs_dir: Path::new("."),
        }
    }

    #[test]
    fn checkbox_passes_well_formed() {
        let content = "## 1. Setup\n- [ ] 1.1 Do thing\n";
        let progress = parse_tasks(content);
        assert_eq!(tasks_use_checkbox_format(&ctx(content, Some(&progress))), RuleOutcome::Pass);
    }

    #[test]
    fn checkbox_rule_fails_on_bare_bullets() {
        let content = "## 1. Setup\n- [ ] 1.1 Do thing\n- bare bullet\n";
        let progress = parse_tasks(content);
        assert!(matches!(
            tasks_use_checkbox_format(&ctx(content, Some(&progress))),
            RuleOutcome::Fail { .. }
        ));
    }

    #[test]
    fn rules_fail_when_tasks_were_not_parsed() {
        assert!(matches!(
            tasks_use_checkbox_format(&ctx("", None)),
            RuleOutcome::Fail { detail } if detail.contains("not parsed")
        ));
        assert!(matches!(
            tasks_grouped_under_headings(&ctx("", None)),
            RuleOutcome::Fail { detail } if detail.contains("not parsed")
        ));
    }

    #[test]
    fn grouping_fails_task_before_heading() {
        let content = "- [ ] 1.1 Do thing\n";
        let progress = parse_tasks(content);
        assert!(matches!(
            tasks_grouped_under_headings(&ctx(content, Some(&progress))),
            RuleOutcome::Fail { .. }
        ));
    }
}
