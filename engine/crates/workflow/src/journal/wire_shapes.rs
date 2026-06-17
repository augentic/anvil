//! Journal event wire-shape contract tables (split for line limits).

use super::{Event, EventKind, test_timestamp, *};
use crate::adapter::operation::SourceOperation;
use crate::change::Divergence;

pub fn assert_wire_rows(rows: &[(EventKind, &[&str])]) {
    for (kind, required) in rows {
        let event = Event::new(test_timestamp("2026-05-22T13:15:00Z"), kind.clone());
        let json = serde_json::to_string(&event).expect("serialise event");
        for needle in *required {
            assert!(json.contains(needle), "wire form must contain `{needle}`; got:\n{json}");
        }
        let round: Event = serde_json::from_str(&json).expect("deserialise event");
        assert_eq!(round, event, "round-trip must preserve every field");
    }
}

pub fn check_contract_part1() {
    let rows: &[(EventKind, &[&str])] = &[
        (
            EventKind::SliceExtractCompleted {
                slice_name: "identity-user-registration".into(),
                source: "runtime".to_string(),
            },
            &[
                r#"{"timestamp":"2026-05-22T13:15:00Z","event":"slice.extract.completed","payload":{"slice-name":"identity-user-registration","source":"runtime"}}"#,
            ],
        ),
        (
            EventKind::SliceReplayCompleted {
                slice_name: "identity-user-registration".into(),
                runner: "omnia-target@1.4 (cargo nextest)".to_string(),
                passed: 47,
                failed: 0,
                skipped: 0,
            },
            &[
                r#""event":"slice.replay.completed""#,
                r#""passed":47"#,
                r#""failed":0"#,
                r#""skipped":0"#,
                r#""runner":"omnia-target@1.4 (cargo nextest)""#,
            ],
        ),
    ];
    assert_wire_rows(rows);
}

pub fn check_contract_part2() {
    let rows: &[(EventKind, &[&str])] = &[
        (
            EventKind::PlanAmendAuthorityOverride {
                plan_name: "identity-revamp".into(),
                slice_name: "identity-user-registration".into(),
                action: AuthorityOverrideAction::Set,
                claim_kind: Some("criterion".to_string()),
                source: Some("runtime".to_string()),
            },
            &[
                r#""event":"plan.amend.authority-override""#,
                r#""action":"set""#,
                r#""claim-kind":"criterion""#,
                r#""source":"runtime""#,
            ],
        ),
        (
            EventKind::SourceSurveyCompleted {
                source: "runtime".to_string(),
                adapter: "captures".to_string(),
            },
            &[
                r#"{"timestamp":"2026-05-22T13:15:00Z","event":"source.survey.completed","payload":{"source":"runtime","adapter":"captures"}}"#,
            ],
        ),
    ];
    assert_wire_rows(rows);
}

pub fn check_contract_part3() {
    let rows: &[(EventKind, &[&str])] = &[
        (
            EventKind::SourceExecutionAgent {
                source: "runtime".to_string(),
                adapter: "captures".to_string(),
                operation: SourceOperation::Survey,
            },
            &[
                r#""event":"source.execution.agent""#,
                r#""operation":"survey""#,
                r#""source":"runtime""#,
                r#""adapter":"captures""#,
            ],
        ),
        (
            EventKind::PlanReconcileCompleted {
                plan_name: "identity-revamp".into(),
                slice_count: 3,
                slice_names: vec![
                    "identity-contracts".into(),
                    "identity-service".into(),
                    "password-reset".into(),
                ],
            },
            &[
                r#""event":"plan.reconcile.completed""#,
                r#""plan-name":"identity-revamp""#,
                r#""slice-count":3"#,
                r#""slice-names":["identity-contracts","identity-service","password-reset"]"#,
            ],
        ),
        (
            EventKind::SliceArchiveCreated {
                slice_name: "identity-service".into(),
                touched_specs: vec!["identity".to_string()],
                outcome_summary: "identity: 2 modified".to_string(),
                merge_sha: Some("a1b2c3d".to_string()),
                decisions: Vec::new(),
            },
            &[
                r#""event":"slice.archive.created""#,
                r#""slice-name":"identity-service""#,
                r#""touched-specs":["identity"]"#,
                r#""outcome-summary":"identity: 2 modified""#,
                r#""merge-sha":"a1b2c3d""#,
            ],
        ),
    ];
    assert_wire_rows(rows);
}

pub fn check_contract_part4() {
    let rows: &[(EventKind, &[&str])] = &[
        (
            EventKind::PlanTransitionApproved {
                plan_name: "identity-revamp".into(),
                actor: Actor::Operator,
            },
            &[
                r#"{"timestamp":"2026-05-22T13:15:00Z","event":"plan.transition.approved","payload":{"plan-name":"identity-revamp","actor":"operator"}}"#,
            ],
        ),
        (
            EventKind::PlanTransitionApproved {
                plan_name: "identity-revamp".into(),
                actor: Actor::Agent,
            },
            &[r#""event":"plan.transition.approved""#, r#""actor":"agent""#],
        ),
        (
            EventKind::PlanEntryAdvanced {
                plan_name: "identity-revamp".into(),
                slice_name: "identity-contracts".into(),
            },
            &[
                r#"{"timestamp":"2026-05-22T13:15:00Z","event":"plan.entry.advanced","payload":{"plan-name":"identity-revamp","slice-name":"identity-contracts"}}"#,
            ],
        ),
        (
            EventKind::WorkspaceSyncCompleted {
                projects: vec!["alpha".to_string(), "beta".to_string()],
            },
            &[
                r#"{"timestamp":"2026-05-22T13:15:00Z","event":"workspace.sync.completed","payload":{"projects":["alpha","beta"]}}"#,
            ],
        ),
        (
            EventKind::WorkspacePushCompleted {
                plan_name: "identity-revamp".into(),
                branch: "specify/identity-revamp".to_string(),
                projects: vec!["alpha".to_string(), "beta".to_string()],
            },
            &[
                r#""event":"workspace.push.completed""#,
                r#""plan-name":"identity-revamp""#,
                r#""branch":"specify/identity-revamp""#,
                r#""projects":["alpha","beta"]"#,
            ],
        ),
        (
            EventKind::CliUpgraded {
                from: "1.4.0".to_string(),
                to: "1.5.0".to_string(),
                channel: "brew".to_string(),
            },
            &[
                r#""event":"cli.upgraded""#,
                r#""from":"1.4.0""#,
                r#""to":"1.5.0""#,
                r#""channel":"brew""#,
            ],
        ),
        (
            EventKind::PluginsRefreshed {
                deleted_paths: vec![".cursor/plugins/cache/augentic/spec".to_string()],
                marketplace: ".cursor-plugin/marketplace.json".to_string(),
            },
            &[
                r#""event":"plugins.refreshed""#,
                r#""deleted-paths":[".cursor/plugins/cache/augentic/spec"]"#,
                r#""marketplace":".cursor-plugin/marketplace.json""#,
            ],
        ),
    ];
    assert_wire_rows(rows);
}

pub fn append_snake_probe_events(layout: Layout<'_>, kinds: &[EventKind]) {
    for kind in kinds {
        let kind = kind.clone();
        append_batch(
            layout,
            std::slice::from_ref(&Event::new(test_timestamp("2026-05-21T20:05:00Z"), kind)),
        )
        .expect("append ok");
    }
}

pub fn assert_no_snake_on_wire(layout: Layout<'_>) {
    let raw = std::fs::read_to_string(path(layout)).expect("read journal");
    for needle in [
        "plan_name",
        "slice_name",
        "slice_count",
        "slice_names",
        "requirement_id",
        "in_progress",
        "touched_specs",
        "outcome_summary",
        "merge_sha",
    ] {
        assert!(
            !raw.contains(needle),
            "snake_case `{needle}` must not appear on the wire; raw:\n{raw}"
        );
    }
}

fn probe_kinds_part1() -> Vec<EventKind> {
    vec![
        EventKind::PlanTransitionApproved {
            plan_name: "p".into(),
            actor: Actor::Operator,
        },
        EventKind::PlanEntryAdvanced {
            plan_name: "p".into(),
            slice_name: "s".into(),
        },
        EventKind::PlanAmendDivergence {
            plan_name: "p".into(),
            slice_name: "s".into(),
            from: Divergence::None,
            to: Divergence::Accepted,
        },
        EventKind::SliceTransitionRefined {
            slice_name: "s".into(),
        },
        EventKind::SliceSynthesizeStarted {
            slice_name: "s".into(),
        },
        EventKind::SliceSynthesizeAgent {
            slice_name: "s".into(),
        },
        EventKind::SliceSynthesizeCompleted {
            slice_name: "s".into(),
            artifacts: vec!["proposal.md".to_string()],
        },
        EventKind::SliceSynthesizeFailed {
            slice_name: "s".into(),
            reason: "spec-requirement-missing-sources".to_string(),
        },
        EventKind::SliceBuildStarted {
            slice_name: "s".into(),
        },
        EventKind::SliceBuildSucceeded {
            slice_name: "s".into(),
        },
        EventKind::SliceBuildFailed {
            slice_name: "s".into(),
            reason: "cargo-check-failed".to_string(),
        },
    ]
}

fn probe_kinds_part2() -> Vec<EventKind> {
    vec![
        EventKind::SliceMergeStarted {
            slice_name: "s".into(),
        },
        EventKind::SliceMergeSucceeded {
            slice_name: "s".into(),
        },
        EventKind::SliceMergeFailed {
            slice_name: "s".into(),
            reason: "baseline-conflict".to_string(),
        },
        EventKind::TargetExecutionAgent {
            slice: "s".into(),
            target: "omnia".to_string(),
        },
        EventKind::SliceExtractCompleted {
            slice_name: "s".into(),
            source: "k".to_string(),
        },
        EventKind::SourceSurveyCompleted {
            source: "k".to_string(),
            adapter: "captures".to_string(),
        },
        EventKind::SourceExecutionAgent {
            source: "k".to_string(),
            adapter: "captures".to_string(),
            operation: SourceOperation::Extract,
        },
        EventKind::PlanReconcileCompleted {
            plan_name: "p".into(),
            slice_count: 1,
            slice_names: vec!["s".into()],
        },
        EventKind::SliceArchiveCreated {
            slice_name: "s".into(),
            touched_specs: vec!["identity".to_string()],
            outcome_summary: "identity: 1 modified".to_string(),
            merge_sha: Some("abc1234".to_string()),
            decisions: Vec::new(),
        },
        EventKind::WorkspaceSyncCompleted {
            projects: vec!["alpha".to_string()],
        },
        EventKind::WorkspacePushCompleted {
            plan_name: "p".into(),
            branch: "specify/p".to_string(),
            projects: vec!["alpha".to_string()],
        },
    ]
}

pub fn run_snake_case_probe(layout: Layout<'_>) {
    let mut kinds = probe_kinds_part1();
    kinds.extend(probe_kinds_part2());
    append_snake_probe_events(layout, &kinds);
    assert_no_snake_on_wire(layout);
}
