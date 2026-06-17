use super::*;

// ---------------------------------------------------------------------------
// Fixture-backed parser tests. Fixtures live at the repo root under
// `tests/fixtures/merge/` and are shared with the merge-engine goldens.
// ---------------------------------------------------------------------------

macro_rules! fixture {
    ($rel:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/merge/", $rel))
    };
}

#[test]
fn parse_baseline_case_01_single_req() {
    let text = fixture!("case-01-single-req/baseline.md");
    let parsed = parse_baseline(text);

    assert_eq!(parsed.requirements.len(), 1);
    let req = &parsed.requirements[0];
    assert_eq!(req.id, "REQ-001");
    assert_eq!(req.name, "User can log in");
    assert_eq!(req.heading, "### Requirement: User can log in");
    assert_eq!(req.scenarios.len(), 2);
    assert_eq!(req.scenarios[0].name, "Valid credentials");
    assert_eq!(req.scenarios[1].name, "Invalid credentials");

    assert!(req.body.starts_with("### Requirement: User can log in"));
    assert!(
        req.body.contains("#### Scenario: Valid credentials"),
        "body should retain scenario headings"
    );

    assert!(!parsed.preamble.is_empty());
    assert!(parsed.preamble.contains("Single-requirement baseline"));
}

#[test]
fn parse_baseline_case_02_multi_req() {
    let text = fixture!("case-02-multi-req/baseline.md");
    let parsed = parse_baseline(text);

    assert_eq!(parsed.requirements.len(), 3);
    let ids: Vec<&str> = parsed.requirements.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["REQ-001", "REQ-002", "REQ-003"]);
    for req in &parsed.requirements {
        assert_eq!(
            req.scenarios.len(),
            1,
            "expected one scenario per requirement, got {:?} for {}",
            req.scenarios.len(),
            req.id
        );
    }
}

#[test]
fn parse_baseline_case_07_all_sections() {
    let text = fixture!("case-07-all-sections/baseline.md");
    let parsed = parse_baseline(text);

    assert_eq!(parsed.requirements.len(), 3);
    let ids: Vec<&str> = parsed.requirements.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["REQ-001", "REQ-002", "REQ-003"]);
}

#[test]
fn parse_delta_case_07_all_sections() {
    let text = fixture!("case-07-all-sections/delta.md");
    let delta = parse_delta(text);

    assert_eq!(delta.renamed.len(), 1);
    assert_eq!(delta.renamed[0].id, "REQ-001");
    assert_eq!(delta.renamed[0].new_name, "User authenticates with email and password");

    assert_eq!(delta.removed.len(), 1);
    assert_eq!(delta.removed[0].id, "REQ-003");

    assert_eq!(delta.modified.len(), 1);
    assert_eq!(delta.modified[0].id, "REQ-002");
    assert_eq!(delta.modified[0].scenarios.len(), 2);

    assert_eq!(delta.added.len(), 1);
    assert_eq!(delta.added[0].id, "REQ-004");
}

#[test]
fn parse_delta_case_03_new_baseline() {
    let text = fixture!("case-03-new-baseline/delta.md");
    assert!(has_delta_headers(text));

    let delta = parse_delta(text);
    assert_eq!(delta.added.len(), 2);
    assert!(delta.renamed.is_empty());
    assert!(delta.removed.is_empty());
    assert!(delta.modified.is_empty());

    let ids: Vec<&str> = delta.added.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["REQ-001", "REQ-002"]);
}

#[test]
fn parse_baseline_09_preserves_oddities() {
    let text = fixture!("case-09-validation-fails/baseline.md");
    let parsed = parse_baseline(text);

    assert_eq!(parsed.requirements.len(), 4);

    assert_eq!(parsed.requirements[0].id, "REQ-001");
    assert_eq!(parsed.requirements[1].id, "REQ-001");
    assert_eq!(
        parsed.requirements[2].id,
        String::new(),
        "missing-ID block should parse with empty id, not be skipped"
    );
    assert_eq!(parsed.requirements[3].id, "REQ-004");
    assert_eq!(
        parsed.requirements[3].scenarios.len(),
        0,
        "block with no scenario heading should have zero scenarios"
    );
}

// ---------------------------------------------------------------------------
// Narrow unit tests
// ---------------------------------------------------------------------------

#[test]
fn has_delta_headers_is_case_insensitive() {
    assert!(has_delta_headers("## added requirements\n"));
    assert!(has_delta_headers("## ADDED Requirements\n"));
    assert!(has_delta_headers("## Modified Requirements\n"));
    assert!(has_delta_headers("# title\n\nsome prose\n\n## REMOVED Requirements\n"));
    assert!(!has_delta_headers("# title\n\njust some prose, no delta headers\n"));
}

#[test]
fn delta_headers_full_line_match() {
    // Prose that merely mentions "## ADDED Requirements" as part of a longer
    // line should not be treated as a delta header.
    assert!(!has_delta_headers("we discussed ## ADDED Requirements at standup\n"));
}

#[test]
fn parse_empty_inputs() {
    let baseline = parse_baseline("");
    assert_eq!(baseline.preamble, String::new());
    assert!(baseline.requirements.is_empty());

    let delta = parse_delta("");
    assert!(delta.renamed.is_empty());
    assert!(delta.removed.is_empty());
    assert!(delta.modified.is_empty());
    assert!(delta.added.is_empty());
}

#[test]
fn parse_baseline_preamble_only() {
    let text = "# Title\n\nIntro text.\n";
    let parsed = parse_baseline(text);
    assert!(!parsed.preamble.is_empty());
    assert!(parsed.preamble.contains("# Title"));
    assert!(parsed.preamble.contains("Intro text."));
    assert!(parsed.requirements.is_empty());
}

#[test]
fn scenario_splitting_round_trips() {
    let req_text = "\
### Requirement: Inline three-scenario req

ID: REQ-042

Some description text.

#### Scenario: First
- GIVEN a
- WHEN b
- THEN c

#### Scenario: Second
- GIVEN d
- WHEN e
- THEN f

#### Scenario: Third
- GIVEN g
- WHEN h
- THEN i
";

    let parsed = parse_baseline(req_text);
    assert_eq!(parsed.requirements.len(), 1);
    let req = &parsed.requirements[0];
    assert_eq!(req.id, "REQ-042");
    assert_eq!(req.scenarios.len(), 3);
    assert_eq!(req.scenarios[0].name, "First");
    assert_eq!(req.scenarios[1].name, "Second");
    assert_eq!(req.scenarios[2].name, "Third");

    for scenario in &req.scenarios {
        assert!(
            scenario.body.starts_with(SCENARIO_HEADING),
            "scenario body should start with the scenario heading, got:\n{}",
            scenario.body
        );
    }

    // The scenario bodies, joined back together, should reconstruct the tail
    // of the requirement body from the first scenario heading onwards. That
    // confirms no lines were dropped by the splitter and trailing context is
    // retained on the last scenario.
    let first_scenario_offset = req
        .body
        .find(SCENARIO_HEADING)
        .expect("requirement body should contain a scenario heading");
    let scenario_tail = &req.body[first_scenario_offset..];

    let bodies: Vec<&str> = req.scenarios.iter().map(|s| s.body.as_str()).collect();
    let rejoined = bodies.join("\n");
    assert_eq!(rejoined, scenario_tail);
}

#[test]
fn parse_baseline_block_without_id_line() {
    let text = "\
### Requirement: No ID here

No ID line follows. This exercises the empty-string id convention.

#### Scenario: Placeholder
- GIVEN nothing
- WHEN validated
- THEN id is empty string
";
    let parsed = parse_baseline(text);
    assert_eq!(parsed.requirements.len(), 1);
    assert_eq!(parsed.requirements[0].id, String::new());
    assert_eq!(parsed.requirements[0].name, "No ID here");
    assert_eq!(parsed.requirements[0].scenarios.len(), 1);
}

#[test]
fn parse_baseline_body_starts_at_heading() {
    // `ReqBlock.body` is the concatenation of all lines from the
    // requirement heading through the end of the block, joined by "\n"
    // with no trailing newline.
    let text = "\
preamble line

### Requirement: Check body layout

ID: REQ-100

Body paragraph.

#### Scenario: Example
- GIVEN a
- WHEN b
- THEN c
";
    let parsed = parse_baseline(text);
    assert_eq!(parsed.requirements.len(), 1);
    let req = &parsed.requirements[0];
    assert!(req.body.starts_with("### Requirement: Check body layout"));
    assert!(req.body.ends_with("- THEN c\n") || req.body.ends_with("- THEN c"));
    assert_eq!(req.heading, "### Requirement: Check body layout");
}

#[test]
fn req_id_grammar_boundaries() {
    assert!(is_req_id("REQ-001"));
    assert!(is_req_id("REQ-999"));
    assert!(!is_req_id("req-001"));
    assert!(!is_req_id("REQ-1"));
    assert!(!is_req_id("REQ-"));
    assert!(!is_req_id("REQ-0012"));
    assert!(!is_req_id("REQ-00a"));
    assert!(!is_req_id("xREQ-001"));
    assert!(!is_req_id("REQ-001\n"));
    assert!(!is_req_id("TASK-001"));
}

#[test]
fn task_id_grammar_boundaries() {
    assert!(is_task_id("TASK-001"));
    assert!(is_task_id("TASK-010"));
    assert!(!is_task_id("task-001"));
    assert!(!is_task_id("TASK-1"));
    assert!(!is_task_id("TASK-"));
    assert!(!is_task_id("TASK-0012"));
    assert!(!is_task_id("TASK-00a"));
    assert!(!is_task_id("REQ-001"));
    assert!(!is_task_id("TASK-001\n"));
}
