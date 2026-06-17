use specify_error::Error;

use super::*;

// ---------------------------------------------------------------------------
// Test 1: happy path — two groups, four tasks, mixed completion.
// ---------------------------------------------------------------------------

const HAPPY_PATH: &str = "\
## 1. Setup

- [x] 1.1 Scaffold
- [ ] 1.2 Configure CI <!-- skill: omnia:crate-writer -->

## 2. Implementation

- [ ] 2.1 Write domain logic
- [ ] 2.2 Add tests
";

#[test]
fn parses_groups_and_tasks() {
    let progress = parse_tasks(HAPPY_PATH);

    assert_eq!(progress.total, 4);
    assert_eq!(progress.complete, 1);
    assert_eq!(progress.tasks.len(), 4);

    assert_eq!(progress.tasks[0].group, "1. Setup");
    assert_eq!(progress.tasks[0].number, "1.1");
    assert_eq!(progress.tasks[0].description, "Scaffold");
    assert!(progress.tasks[0].complete);
    assert_eq!(progress.tasks[0].skill_directive, None);

    assert_eq!(progress.tasks[1].group, "1. Setup");
    assert_eq!(progress.tasks[1].number, "1.2");
    assert_eq!(progress.tasks[1].description, "Configure CI");
    assert!(!progress.tasks[1].complete);
    assert_eq!(
        progress.tasks[1].skill_directive,
        Some(SkillDirective {
            plugin: "omnia".to_string(),
            skill: "crate-writer".to_string(),
        })
    );

    assert_eq!(progress.tasks[2].group, "2. Implementation");
    assert_eq!(progress.tasks[2].number, "2.1");
    assert_eq!(progress.tasks[2].description, "Write domain logic");
    assert!(!progress.tasks[2].complete);

    assert_eq!(progress.tasks[3].group, "2. Implementation");
    assert_eq!(progress.tasks[3].number, "2.2");
    assert_eq!(progress.tasks[3].description, "Add tests");
    assert!(!progress.tasks[3].complete);
}

// ---------------------------------------------------------------------------
// Test 2: mark_complete happy path.
// ---------------------------------------------------------------------------

#[test]
fn mark_complete_flips_checkbox() {
    let out = mark_complete(HAPPY_PATH, "1.2").expect("1.2 exists and is unchecked");

    assert!(out.contains("- [x] 1.2 Configure CI <!-- skill: omnia:crate-writer -->"));

    // Every other line is byte-identical — i.e. the only change is in the
    // `[ ]` → `[x]` substitution on a single line.
    let original_lines: Vec<&str> = HAPPY_PATH.lines().collect();
    let new_lines: Vec<&str> = out.lines().collect();
    assert_eq!(original_lines.len(), new_lines.len());
    let changed: Vec<(usize, &&str, &&str)> = original_lines
        .iter()
        .zip(new_lines.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, (a, b))| (i, a, b))
        .collect();
    assert_eq!(changed.len(), 1, "exactly one line must change");
    assert_eq!(*changed[0].1, "- [ ] 1.2 Configure CI <!-- skill: omnia:crate-writer -->");
    assert_eq!(*changed[0].2, "- [x] 1.2 Configure CI <!-- skill: omnia:crate-writer -->");
}

// ---------------------------------------------------------------------------
// Test 3: mark_complete is idempotent.
// ---------------------------------------------------------------------------

#[test]
fn mark_complete_idempotent() {
    let out = mark_complete(HAPPY_PATH, "1.1").expect("1.1 exists");
    assert_eq!(out, HAPPY_PATH);
    assert_eq!(out.as_bytes(), HAPPY_PATH.as_bytes());
}

// ---------------------------------------------------------------------------
// Test 4: mark_complete with unknown task number errors cleanly.
// ---------------------------------------------------------------------------

#[test]
fn mark_complete_missing_task_errors() {
    let err = mark_complete(HAPPY_PATH, "9.9").expect_err("9.9 does not exist");
    match err {
        Error::Diag { detail: msg, .. } => {
            assert!(msg.contains("task 9.9 not found"), "unexpected message: {msg}");
        }
        other => panic!("expected Error::Diag, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test 5: skill directive regex accepts various plugin:skill shapes.
// ---------------------------------------------------------------------------

#[test]
fn skill_directive_parses_variants() {
    let input = "\
## 1. Group

- [ ] 1.1 Generate crate <!-- skill: omnia:crate-writer -->
- [ ] 1.2 Review core <!-- skill: vectis:core-reviewer -->
- [ ] 1.3 Author contract <!-- skill: contract:openapi -->
";
    let progress = parse_tasks(input);
    assert_eq!(progress.tasks.len(), 3);
    assert_eq!(
        progress.tasks[0].skill_directive,
        Some(SkillDirective {
            plugin: "omnia".to_string(),
            skill: "crate-writer".to_string()
        })
    );
    assert_eq!(progress.tasks[0].description, "Generate crate");
    assert_eq!(
        progress.tasks[1].skill_directive,
        Some(SkillDirective {
            plugin: "vectis".to_string(),
            skill: "core-reviewer".to_string()
        })
    );
    assert_eq!(progress.tasks[1].description, "Review core");
    assert_eq!(
        progress.tasks[2].skill_directive,
        Some(SkillDirective {
            plugin: "contract".to_string(),
            skill: "openapi".to_string()
        })
    );
    assert_eq!(progress.tasks[2].description, "Author contract");
}

// ---------------------------------------------------------------------------
// Test 6: non-matching comment stays in the description.
// ---------------------------------------------------------------------------

#[test]
fn non_skill_comment_preserved() {
    let input = "\
## 3. Notes

- [ ] 3.1 Do thing <!-- TODO: reconsider -->
";
    let progress = parse_tasks(input);
    assert_eq!(progress.tasks.len(), 1);
    let task = &progress.tasks[0];
    assert_eq!(task.number, "3.1");
    assert_eq!(task.skill_directive, None);
    assert_eq!(task.description, "Do thing <!-- TODO: reconsider -->");
}

// ---------------------------------------------------------------------------
// Test 7: duplicate task numbers are both parsed; mark_complete targets
// only the first unmarked occurrence.
// ---------------------------------------------------------------------------

#[test]
fn duplicate_task_numbers_are_both_parsed() {
    let input = "\
## 1. Group

- [ ] 1.1 First occurrence
- [ ] 1.1 Second occurrence
";
    let progress = parse_tasks(input);
    assert_eq!(progress.tasks.len(), 2);
    assert_eq!(progress.tasks[0].description, "First occurrence");
    assert_eq!(progress.tasks[1].description, "Second occurrence");
    assert_eq!(progress.total, 2);
    assert_eq!(progress.complete, 0);
}

#[test]
fn mark_complete_first_duplicate() {
    let input = "\
## 1. Group

- [ ] 1.1 First occurrence
- [ ] 1.1 Second occurrence
";
    let out = mark_complete(input, "1.1").expect("1.1 exists");
    assert!(out.contains("- [x] 1.1 First occurrence"));
    assert!(out.contains("- [ ] 1.1 Second occurrence"));
}

#[test]
fn mark_complete_duplicate_complete_noop() {
    let input = "\
## 1. Group

- [x] 1.1 First occurrence
- [ ] 1.1 Second occurrence
";
    let out = mark_complete(input, "1.1").expect("1.1 exists");
    assert_eq!(out, input);
}

// ---------------------------------------------------------------------------
// Test 8: nested headings don't reset the group.
// ---------------------------------------------------------------------------

#[test]
fn nested_headings_do_not_reset_group() {
    let input = "\
## 1. Implementation

- [ ] 1.1 First task

### Subsection

- [ ] 1.2 After nested heading
";
    let progress = parse_tasks(input);
    assert_eq!(progress.tasks.len(), 2);
    assert_eq!(progress.tasks[0].group, "1. Implementation");
    assert_eq!(progress.tasks[1].group, "1. Implementation");
}

// ---------------------------------------------------------------------------
// Test 9: task before any group heading gets an empty group.
// ---------------------------------------------------------------------------

#[test]
fn task_before_any_heading_has_empty_group() {
    let input = "\
- [ ] 0.1 Lonely task

## 1. Later

- [ ] 1.1 Grouped task
";
    let progress = parse_tasks(input);
    assert_eq!(progress.tasks.len(), 2);
    assert_eq!(progress.tasks[0].group, "");
    assert_eq!(progress.tasks[0].number, "0.1");
    assert_eq!(progress.tasks[1].group, "1. Later");
}

// ---------------------------------------------------------------------------
// Test 10: empty input.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_yields_empty_progress() {
    let progress = parse_tasks("");
    assert_eq!(
        progress,
        Progress {
            total: 0,
            complete: 0,
            tasks: vec![]
        }
    );
}

// ---------------------------------------------------------------------------
// Test 11: capital X is accepted as complete.
// ---------------------------------------------------------------------------

#[test]
fn capital_x_parses_as_complete() {
    let input = "\
## 1. Group

- [X] 1.1 foo
";
    let progress = parse_tasks(input);
    assert_eq!(progress.tasks.len(), 1);
    assert!(progress.tasks[0].complete);
    assert_eq!(progress.tasks[0].description, "foo");
    assert_eq!(progress.complete, 1);
}

#[test]
fn mark_complete_is_noop_for_capital_x() {
    let input = "\
## 1. Group

- [X] 1.1 foo
";
    let out = mark_complete(input, "1.1").expect("1.1 exists");
    assert_eq!(out, input);
}

// ---------------------------------------------------------------------------
// Additional edge cases worth locking in.
// ---------------------------------------------------------------------------

#[test]
fn non_task_bullets_are_ignored() {
    let input = "\
## 1. Group

- An explanatory bullet that isn't a task
- [ ] 1.1 Real task
- Not a task either
";
    let progress = parse_tasks(input);
    assert_eq!(progress.tasks.len(), 1);
    assert_eq!(progress.tasks[0].number, "1.1");
}

#[test]
fn deep_task_numbers_preserved() {
    let input = "\
## 1. Deep

- [ ] 1.2.3 Nested numbering
- [x] 1.2.3.4 Very deep
";
    let progress = parse_tasks(input);
    assert_eq!(progress.tasks.len(), 2);
    assert_eq!(progress.tasks[0].number, "1.2.3");
    assert_eq!(progress.tasks[1].number, "1.2.3.4");
    assert!(progress.tasks[1].complete);
}

#[test]
fn mark_complete_preserves_crlf() {
    // CRLF in the input — ensure `mark_complete` edits exactly one byte
    // range and doesn't normalise line endings elsewhere.
    let input = "## 1. Group\r\n\r\n- [ ] 1.1 task\r\n- [ ] 1.2 other\r\n";
    let out = mark_complete(input, "1.1").expect("1.1 exists");
    assert_eq!(
        out, "## 1. Group\r\n\r\n- [x] 1.1 task\r\n- [ ] 1.2 other\r\n",
        "only the targeted line's `[ ]` → `[x]` should change"
    );
}
