use tempfile::tempdir;

use super::append::{DROPPED_FILE_NAME, append_dropped};
use super::{wire_shapes, *};

fn read_lines(layout: Layout<'_>) -> Vec<String> {
    let raw = std::fs::read_to_string(path(layout)).expect("read journal");
    raw.lines().map(str::to_owned).collect()
}

#[test]
fn append_creates_specify_dir_when_missing() {
    let dir = tempdir().expect("tempdir");
    let layout = Layout::new(dir.path());
    assert!(!layout.specify_dir().exists(), "precondition: .specify must not exist yet");

    let event = Event::new(
        test_timestamp("2026-05-21T20:02:00Z"),
        EventKind::SliceTransitionRefined {
            slice_name: "checkout".into(),
        },
    );
    append_batch(layout, std::slice::from_ref(&event)).expect("append ok");

    assert!(layout.specify_dir().is_dir(), ".specify/ must exist after first append");
    assert!(path(layout).is_file(), "journal.jsonl must exist after first append");
}

#[test]
fn append_batch_writes_in_order() {
    // auto-approve Gate-1 contract: `specify plan create --auto-approve
    // --authority-override` may emit both `plan.transition.approved`
    // and `plan.amend.authority-override` in a single fsynced append.
    // Exercise the batched helper to lock ordering.
    let dir = tempdir().expect("tempdir");
    let layout = Layout::new(dir.path());
    let events = vec![
        Event::new(
            test_timestamp("2026-05-22T13:30:00Z"),
            EventKind::PlanTransitionApproved {
                plan_name: "fresh".into(),
                actor: Actor::Operator,
            },
        ),
        Event::new(
            test_timestamp("2026-05-22T13:30:00Z"),
            EventKind::PlanAmendAuthorityOverride {
                plan_name: "fresh".into(),
                slice_name: "checkout".into(),
                action: AuthorityOverrideAction::Set,
                claim_kind: Some("criterion".to_string()),
                source: Some("runtime".to_string()),
            },
        ),
    ];
    append_batch(layout, &events).expect("append_batch ok");

    let lines = read_lines(layout);
    assert_eq!(lines.len(), 2, "expected two journal lines, got {}", lines.len());
    assert!(
        lines[0].contains(r#""event":"plan.transition.approved""#),
        "first line must be plan.transition.approved, got:\n{}",
        lines[0]
    );
    assert!(
        lines[1].contains(r#""event":"plan.amend.authority-override""#),
        "second line must be plan.amend.authority-override, got:\n{}",
        lines[1]
    );
}

#[test]
fn append_batch_empty_slice_is_no_op() {
    // Callers (e.g. `plan create` without `--auto-approve` and
    // without `--authority-override`) build the batch
    // unconditionally; an empty input must not create the
    // journal file on disk.
    let dir = tempdir().expect("tempdir");
    let layout = Layout::new(dir.path());
    append_batch(layout, &[]).expect("empty batch ok");
    assert!(
        !path(layout).exists(),
        "empty append_batch must not create journal.jsonl, found: {}",
        path(layout).display()
    );
}

#[test]
fn append_failure_records_dropped_sidecar() {
    // Journal events are observability, not the source of truth: a failed
    // primary append must not crash the verb, but it must not vanish either.
    // Force `append_batch` to fail by making `journal.jsonl` a directory so
    // the append `open()` errors, then assert `emit_best_effort` swallowed
    // the failure AND captured the event in the `.specify/journal.dropped`
    // sidecar.
    let dir = tempdir().expect("tempdir");
    let layout = Layout::new(dir.path());
    std::fs::create_dir_all(path(layout)).expect("create journal.jsonl as a directory");

    emit_best_effort(
        layout,
        test_timestamp("2026-05-21T20:02:00Z"),
        EventKind::SliceMergeSucceeded {
            slice_name: "checkout".into(),
        },
        "slice.merge",
    );

    let sidecar = layout.specify_dir().join(DROPPED_FILE_NAME);
    let raw = std::fs::read_to_string(&sidecar).expect("dropped sidecar written on append failure");
    assert!(
        raw.contains(r#""event":"slice.merge.succeeded""#),
        "sidecar must capture the dropped event line; got:\n{raw}"
    );
    assert!(raw.ends_with('\n'), "sidecar line must be newline-terminated; got:\n{raw}");
}

#[test]
fn dropped_sidecar_appends_event_lines() {
    // The sidecar writer is append-only: repeated drops accumulate one
    // newline-terminated JSON line each, so the recovery trail preserves
    // every dropped event in order.
    let dir = tempdir().expect("tempdir");
    let layout = Layout::new(dir.path());
    let event = Event::new(
        test_timestamp("2026-05-21T20:02:00Z"),
        EventKind::SliceTransitionRefined {
            slice_name: "checkout".into(),
        },
    );
    append_dropped(layout, &event).expect("first sidecar append");
    append_dropped(layout, &event).expect("second sidecar append");

    let raw = std::fs::read_to_string(layout.specify_dir().join(DROPPED_FILE_NAME))
        .expect("read sidecar");
    assert_eq!(raw.lines().count(), 2, "two appends must yield two sidecar lines; got:\n{raw}");
}

#[test]
fn event_wire_shapes_match_contract() {
    wire_shapes::check_contract_part1();
    wire_shapes::check_contract_part2();
    wire_shapes::check_contract_part3();
    wire_shapes::check_contract_part4();
}

#[test]
fn approved_actor_defaults_on_legacy_lines() {
    // Journal lines written before the `actor` field existed carry
    // only `plan-name`; `#[serde(default)]` must parse them as
    // `actor: operator` so historic journals stay readable.
    let legacy = r#"{"timestamp":"2026-05-21T20:00:00Z","event":"plan.transition.approved","payload":{"plan-name":"platform-v2"}}"#;
    let event: Event = serde_json::from_str(legacy).expect("legacy line parses");
    assert_eq!(
        event.kind,
        EventKind::PlanTransitionApproved {
            plan_name: "platform-v2".into(),
            actor: Actor::Operator,
        },
        "absent actor must default to operator"
    );
}

// Per-family CLI-emitted round-trips were trimmed: every variant's wire
// id is locked by `wire_event_ids_match_serde_renames`, field casing by
// the snake-case probe, the exact-line rows by the `wire_shapes` contract
// tables, and the emission paths by the binary goldens in
// `tests/journal.rs`. Agent-emitted rows (`slice.replay.completed`,
// `plan.amend.authority-override`) stay in the contract tables.

#[test]
fn synthesize_omits_empty_artifacts() {
    // `artifacts` carries `skip_serializing_if = "Vec::is_empty"`
    // so an empty list does not reach the wire at all.
    let event = Event::new(
        test_timestamp("2026-05-22T13:15:00Z"),
        EventKind::SliceSynthesizeCompleted {
            slice_name: "identity-user-registration".into(),
            artifacts: vec![],
        },
    );
    let json = serde_json::to_string(&event).expect("serialise");
    assert!(!json.contains("artifacts"), "empty artifacts must not reach the wire; got:\n{json}");
    let round: Event = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(round, event, "round-trip must preserve the empty artifacts list");
}

#[test]
fn lint_completed_round_trips() {
    // The lint-completed payload uses snake_case wire fields
    // (`duration_ms`, `false_positive`, `exit_code`) so the JSON
    // matches the payload example verbatim. The wire id itself
    // stays dotted-kebab.
    let event = Event::new(
        test_timestamp("2026-05-22T13:15:00Z"),
        EventKind::LintCompleted(LintCompletedPayload {
            scope: LintScope {
                target: Some("omnia".to_string()),
                slice: None,
                artifact: None,
            },
            duration_ms: 824,
            counts: LintCounts {
                open: 12,
                ignored: 4,
                false_positive: 0,
            },
            exit_code: 2,
        }),
    );

    let json = serde_json::to_string(&event).expect("serialise lint-completed");
    let round_tripped: Event = serde_json::from_str(&json).expect("deserialise lint-completed");
    assert_eq!(round_tripped, event, "round-trip must preserve every field");

    for needle in [
        r#""event":"lint-completed""#,
        r#""scope":{"target":"omnia","slice":null,"artifact":null}"#,
        r#""duration_ms":824"#,
        r#""open":12"#,
        r#""ignored":4"#,
        r#""false_positive":0"#,
        r#""exit_code":2"#,
    ] {
        assert!(
            json.contains(needle),
            "lint-completed wire form must contain `{needle}`; got:\n{json}"
        );
    }

    // Guard against an accidental rename_all = "kebab-case" on the payload structs.
    for forbidden in [r#""duration-ms""#, r#""false-positive""#, r#""exit-code""#] {
        assert!(
            !json.contains(forbidden),
            "lint-completed wire form must NOT contain kebab-case `{forbidden}`; got:\n{json}"
        );
    }
}

#[test]
fn no_snake_case_leaks_to_wire() {
    let dir = tempdir().expect("tempdir");
    let layout = Layout::new(dir.path());
    wire_shapes::run_snake_case_probe(layout);
}

/// Collect every line `for_each_line_rev` yields, restored to file order.
fn collect_chronological(path: &Path, chunk: usize) -> Vec<String> {
    let mut newest_first: Vec<String> = Vec::new();
    for_each_line_rev(path, chunk, |line| {
        newest_first.push(line.to_owned());
        true
    })
    .expect("for_each_line_rev");
    newest_first.reverse();
    newest_first
}

#[test]
fn for_each_line_rev_matches_str_lines() {
    // The tail reader must agree with `str::lines` for every line-boundary
    // shape — trailing newline or not, interior and trailing blanks, empty
    // file — across chunk sizes that force partial-line splits at the edge,
    // including multi-byte UTF-8 straddling a chunk boundary.
    let cases = [
        "",
        "a",
        "a\n",
        "a\nb\nc",
        "a\nb\nc\n",
        "\n",
        "\n\n",
        "a\n\nb\n",
        "a\n\n",
        "\na\n",
        "αβ\nγδ\nεζ",
        "αβ\nγδ\nεζ\n",
    ];
    let dir = tempdir().expect("tempdir");
    for (idx, content) in cases.iter().enumerate() {
        let path = dir.path().join(format!("case-{idx}.jsonl"));
        std::fs::write(&path, content).expect("write case");
        let expected: Vec<String> = content.lines().map(str::to_owned).collect();
        for chunk in [1_usize, 2, 3, 5, 8192] {
            assert_eq!(
                collect_chronological(&path, chunk),
                expected,
                "content {content:?} at chunk {chunk} must match str::lines"
            );
        }
    }
}

#[test]
fn for_each_line_rev_missing_file() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("absent.jsonl");
    let mut visited = 0_usize;
    for_each_line_rev(&path, 8192, |_| {
        visited += 1;
        true
    })
    .expect("missing file is not an error");
    assert_eq!(visited, 0, "a missing file must yield no lines");
}

#[test]
fn for_each_line_rev_early_stop() {
    // Returning `false` halts the backward scan; only the tail is touched.
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("early.jsonl");
    std::fs::write(&path, "first\nsecond\nthird\n").expect("write");
    let mut seen: Vec<String> = Vec::new();
    for_each_line_rev(&path, 8192, |line| {
        seen.push(line.to_owned());
        false
    })
    .expect("for_each_line_rev");
    assert_eq!(seen, vec!["third".to_string()], "early stop must yield only the newest line");
}

fn write_archive_summaries(layout: Layout<'_>, summaries: &[&str]) {
    for summary in summaries {
        let event = Event::new(
            test_timestamp("2026-05-22T13:15:00Z"),
            EventKind::SliceArchiveCreated {
                slice_name: "slice".into(),
                touched_specs: vec![],
                outcome_summary: (*summary).to_string(),
                merge_sha: None,
                decisions: vec![],
            },
        );
        append_batch(layout, std::slice::from_ref(&event)).expect("append archive event");
    }
}

#[test]
fn read_recent_last_n_matching() {
    let dir = tempdir().expect("tempdir");
    let layout = Layout::new(dir.path());
    // Interleave non-matching events so the tail must filter, not just slice
    // the last N raw lines.
    for idx in 0..5 {
        write_archive_summaries(layout, &[&format!("merge-{idx}")]);
        let noise = Event::new(
            test_timestamp("2026-05-22T13:15:00Z"),
            EventKind::SliceBuildStarted {
                slice_name: "slice".into(),
            },
        );
        append_batch(layout, std::slice::from_ref(&noise)).expect("append noise");
    }

    let recent: Vec<String> = read_recent(layout, 3, |event| match event.kind {
        EventKind::SliceArchiveCreated { outcome_summary, .. } => Some(outcome_summary),
        _ => None,
    })
    .expect("read_recent");
    assert_eq!(
        recent,
        vec!["merge-2".to_string(), "merge-3".to_string(), "merge-4".to_string()],
        "must keep the last N matching summaries in append order"
    );
}

#[test]
fn read_recent_short_missing_and_zero_limit() {
    let dir = tempdir().expect("tempdir");
    let layout = Layout::new(dir.path());

    // Missing journal -> empty.
    let absent: Vec<String> = read_recent(layout, 10, |event| match event.kind {
        EventKind::SliceArchiveCreated { outcome_summary, .. } => Some(outcome_summary),
        _ => None,
    })
    .expect("read_recent missing");
    assert!(absent.is_empty(), "missing journal must yield no summaries");

    write_archive_summaries(layout, &["only"]);

    // Fewer matches than the limit -> all of them, in order.
    let short: Vec<String> = read_recent(layout, 10, |event| match event.kind {
        EventKind::SliceArchiveCreated { outcome_summary, .. } => Some(outcome_summary),
        _ => None,
    })
    .expect("read_recent short");
    assert_eq!(short, vec!["only".to_string()], "fewer-than-limit must return all matches");

    // Zero limit -> empty, no read needed.
    let none: Vec<String> = read_recent(layout, 0, |event| match event.kind {
        EventKind::SliceArchiveCreated { outcome_summary, .. } => Some(outcome_summary),
        _ => None,
    })
    .expect("read_recent zero");
    assert!(none.is_empty(), "a zero limit must yield no summaries");
}

mod show {
    use super::*;

    fn seed(layout: Layout<'_>) {
        // Three families so prefix filtering has neighbours to exclude:
        // 2x slice.build.*, 1x slice.merge.*, 5x slice.archive.created.
        let build_started = Event::new(
            test_timestamp("2026-05-22T13:15:00Z"),
            EventKind::SliceBuildStarted {
                slice_name: "checkout".into(),
            },
        );
        let build_succeeded = Event::new(
            test_timestamp("2026-05-22T13:16:00Z"),
            EventKind::SliceBuildSucceeded {
                slice_name: "checkout".into(),
            },
        );
        let merge_started = Event::new(
            test_timestamp("2026-05-22T13:17:00Z"),
            EventKind::SliceMergeStarted {
                slice_name: "checkout".into(),
            },
        );
        append_batch(layout, &[build_started, build_succeeded, merge_started])
            .expect("append seed events");
        write_archive_summaries(layout, &["m-0", "m-1", "m-2", "m-3", "m-4"]);
    }

    fn ids(events: &[Event]) -> Vec<String> {
        events.iter().map(|event| wire_id(&event.kind)).collect()
    }

    #[test]
    fn unfiltered_returns_all_in_order() {
        let dir = tempdir().expect("tempdir");
        let layout = Layout::new(dir.path());
        seed(layout);

        let events = show(layout, None, None).expect("show");
        assert_eq!(events.len(), 8, "all parseable events must be returned");
        assert_eq!(
            ids(&events)[..3],
            ["slice.build.started", "slice.build.succeeded", "slice.merge.started"],
            "append (file) order must be preserved"
        );
    }

    #[test]
    fn filter_is_id_prefix() {
        let dir = tempdir().expect("tempdir");
        let layout = Layout::new(dir.path());
        seed(layout);

        let builds = show(layout, Some("slice.build"), None).expect("show filtered");
        assert_eq!(
            ids(&builds),
            ["slice.build.started", "slice.build.succeeded"],
            "the dotted prefix must keep the whole family and nothing else"
        );

        let exact = show(layout, Some("slice.merge.started"), None).expect("show exact");
        assert_eq!(ids(&exact), ["slice.merge.started"], "a full id is its own prefix");

        let none = show(layout, Some("workspace."), None).expect("show no match");
        assert!(none.is_empty(), "an unmatched prefix must yield no events");
    }

    #[test]
    fn limit_tails_matches() {
        let dir = tempdir().expect("tempdir");
        let layout = Layout::new(dir.path());
        seed(layout);

        let recent = show(layout, Some("slice.archive.created"), Some(2)).expect("show limited");
        let summaries: Vec<&str> = recent
            .iter()
            .map(|event| match &event.kind {
                EventKind::SliceArchiveCreated { outcome_summary, .. } => outcome_summary.as_str(),
                other => panic!("filter must only keep archive events, got {other:?}"),
            })
            .collect();
        assert_eq!(summaries, ["m-3", "m-4"], "limit keeps the most recent matches, in order");
    }

    #[test]
    fn missing_journal_yields_empty() {
        let dir = tempdir().expect("tempdir");
        let layout = Layout::new(dir.path());
        let events = show(layout, None, Some(10)).expect("show on missing journal");
        assert!(events.is_empty(), "a missing journal must yield no events");
    }
}

/// One sample per [`EventKind`] variant (composed from the per-family
/// helpers below). The exhaustive match in
/// [`wire_event_ids_match_serde_renames`] fails to compile when a
/// variant is added, prompting a new sample here and a new id in
/// [`WIRE_EVENT_IDS`].
fn sample_event_kinds() -> Vec<EventKind> {
    let mut samples = sample_plan_kinds();
    samples.extend(sample_slice_kinds());
    samples.extend(sample_runtime_kinds());
    samples
}

fn sample_plan_kinds() -> Vec<EventKind> {
    use crate::change::{Divergence, Status};

    vec![
        EventKind::PlanTransitionApproved {
            plan_name: "plan".into(),
            actor: Actor::Operator,
        },
        EventKind::PlanTransitionUndone {
            plan_name: "plan".into(),
            slice_name: "slice".into(),
            from: Status::Done,
            to: Status::InProgress,
        },
        EventKind::PlanEntryAdvanced {
            plan_name: "plan".into(),
            slice_name: "slice".into(),
        },
        EventKind::PlanAmendDivergence {
            plan_name: "plan".into(),
            slice_name: "slice".into(),
            from: Divergence::None,
            to: Divergence::Likely,
        },
        EventKind::PlanAmendAuthorityOverride {
            plan_name: "plan".into(),
            slice_name: "slice".into(),
            action: AuthorityOverrideAction::Set,
            claim_kind: None,
            source: None,
        },
        EventKind::PlanReconcileCompleted {
            plan_name: "plan".into(),
            slice_count: 0,
            slice_names: vec![],
        },
    ]
}

fn sample_slice_kinds() -> Vec<EventKind> {
    vec![
        EventKind::SliceTransitionRefined {
            slice_name: "slice".into(),
        },
        EventKind::SliceExtractCompleted {
            slice_name: "slice".into(),
            source: "src".to_string(),
        },
        EventKind::SliceSynthesisConflict {
            slice_name: "slice".into(),
            requirement_id: "REQ-001".to_string(),
        },
        EventKind::SliceSynthesisDivergence {
            slice_name: "slice".into(),
            requirement_id: "REQ-001".to_string(),
        },
        EventKind::SliceSynthesisUnknown {
            slice_name: "slice".into(),
            requirement_id: "REQ-001".to_string(),
        },
        EventKind::SliceSynthesizeStarted {
            slice_name: "slice".into(),
        },
        EventKind::SliceSynthesizeAgent {
            slice_name: "slice".into(),
        },
        EventKind::SliceSynthesizeCompleted {
            slice_name: "slice".into(),
            artifacts: vec![],
        },
        EventKind::SliceSynthesizeFailed {
            slice_name: "slice".into(),
            reason: "r".to_string(),
        },
        EventKind::SliceBuildStarted {
            slice_name: "slice".into(),
        },
        EventKind::SliceBuildSucceeded {
            slice_name: "slice".into(),
        },
        EventKind::SliceBuildFailed {
            slice_name: "slice".into(),
            reason: "r".to_string(),
        },
        EventKind::SliceMergeStarted {
            slice_name: "slice".into(),
        },
        EventKind::SliceMergeSucceeded {
            slice_name: "slice".into(),
        },
        EventKind::SliceMergeFailed {
            slice_name: "slice".into(),
            reason: "r".to_string(),
        },
        EventKind::SliceReplayCompleted {
            slice_name: "slice".into(),
            runner: "runner".to_string(),
            passed: 1,
            failed: 0,
            skipped: 0,
        },
        EventKind::SliceArchiveCreated {
            slice_name: "slice".into(),
            touched_specs: vec![],
            outcome_summary: "ok".to_string(),
            merge_sha: None,
            decisions: vec![],
        },
    ]
}

fn sample_runtime_kinds() -> Vec<EventKind> {
    use crate::adapter::operation::SourceOperation;

    vec![
        EventKind::SourceSurveyCompleted {
            source: "src".to_string(),
            adapter: "adp".to_string(),
        },
        EventKind::SourceExecutionAgent {
            source: "src".to_string(),
            adapter: "adp".to_string(),
            operation: SourceOperation::Survey,
        },
        EventKind::TargetExecutionAgent {
            slice: "slice".into(),
            target: "omnia".to_string(),
        },
        EventKind::CliUpgraded {
            from: "0.1.0".to_string(),
            to: "0.2.0".to_string(),
            channel: "cargo".to_string(),
        },
        EventKind::PluginsRefreshed {
            deleted_paths: vec![],
            marketplace: "marketplace.json".to_string(),
        },
        EventKind::WorkspaceSyncCompleted { projects: vec![] },
        EventKind::WorkspacePushCompleted {
            plan_name: "plan".into(),
            branch: "specify/plan".to_string(),
            projects: vec![],
        },
        EventKind::LintCompleted(LintCompletedPayload {
            scope: LintScope {
                target: None,
                slice: None,
                artifact: None,
            },
            duration_ms: 0,
            counts: LintCounts {
                open: 0,
                ignored: 0,
                false_positive: 0,
            },
            exit_code: 0,
        }),
    ]
}

#[test]
fn wire_event_ids_match_serde_renames() {
    // Compile-time exhaustiveness: listing every variant with no
    // wildcard arm makes this match fail to compile when a variant is
    // added — the prompt to extend `sample_event_kinds` and
    // `WIRE_EVENT_IDS` together.
    let samples = sample_event_kinds();
    for kind in &samples {
        match kind {
            EventKind::PlanTransitionApproved { .. }
            | EventKind::PlanTransitionUndone { .. }
            | EventKind::PlanEntryAdvanced { .. }
            | EventKind::PlanAmendDivergence { .. }
            | EventKind::SliceTransitionRefined { .. }
            | EventKind::SliceExtractCompleted { .. }
            | EventKind::SliceSynthesisConflict { .. }
            | EventKind::SliceSynthesisDivergence { .. }
            | EventKind::SliceSynthesisUnknown { .. }
            | EventKind::SliceSynthesizeStarted { .. }
            | EventKind::SliceSynthesizeAgent { .. }
            | EventKind::SliceSynthesizeCompleted { .. }
            | EventKind::SliceSynthesizeFailed { .. }
            | EventKind::SliceBuildStarted { .. }
            | EventKind::SliceBuildSucceeded { .. }
            | EventKind::SliceBuildFailed { .. }
            | EventKind::SliceMergeStarted { .. }
            | EventKind::SliceMergeSucceeded { .. }
            | EventKind::SliceMergeFailed { .. }
            | EventKind::SourceSurveyCompleted { .. }
            | EventKind::SourceExecutionAgent { .. }
            | EventKind::TargetExecutionAgent { .. }
            | EventKind::SliceReplayCompleted { .. }
            | EventKind::PlanAmendAuthorityOverride { .. }
            | EventKind::PlanReconcileCompleted { .. }
            | EventKind::SliceArchiveCreated { .. }
            | EventKind::CliUpgraded { .. }
            | EventKind::PluginsRefreshed { .. }
            | EventKind::WorkspaceSyncCompleted { .. }
            | EventKind::WorkspacePushCompleted { .. }
            | EventKind::LintCompleted(_) => {}
        }
    }

    let mut serialised: Vec<String> = samples
        .iter()
        .map(|kind| {
            let event = Event::new(test_timestamp("2026-05-21T20:00:00Z"), kind.clone());
            let value = serde_json::to_value(&event).expect("event serialises");
            value["event"].as_str().expect("event id is a string").to_string()
        })
        .collect();
    serialised.sort();
    serialised.dedup();
    assert_eq!(
        serialised, WIRE_EVENT_IDS,
        "WIRE_EVENT_IDS must equal the serde renames of every EventKind variant"
    );

    for window in WIRE_EVENT_IDS.windows(2) {
        assert!(
            window[0] < window[1],
            "WIRE_EVENT_IDS must stay sorted: `{}` >= `{}`",
            window[0],
            window[1]
        );
    }
}
