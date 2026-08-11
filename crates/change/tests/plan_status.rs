//! Integration coverage for the read-only `plan status` projection,
//! exercised through the `plan status` operation (the public
//! boundary): each test stages `plan.yaml`, slice artifacts, and
//! journal events on disk, invokes the operation, and asserts the
//! projected `StatusBody`.
//!
//! Progress is computed from artifacts and facts (RFC-86 D2 / D11) —
//! `plan.yaml` and `metadata.yaml` carry no stored status fields.
//! Ready / Authorized milestones follow D22 / D26 — never an
//! `approved` rung.
//!
//! The base happy-path dispatch arms (fresh-active-refine,
//! per-entry refine/build/merge, drained,
//! eligible-pending preview) are asserted end-to-end through the
//! crate's orchestrate suites. What stays here is the dispatch and
//! overlay classification that has no CLI status fixture: stuck
//! dependency graphs, dropped slices, failure-overlay precedence, the
//! torn merge-incomplete state, re-entry resume points, Ready /
//! Authorized, and workspace slot routing.

mod support;

use change::plan::handlers::{Status as StatusOp, StatusInput};
use change::{DebtCounts, LoopStep, NextActionKind, Plan, StatusBody};
use diagnostics::digest::sha256_hex;
use jiff::Timestamp;
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{
    ClosedPlanCoverage, DEFAULT_WRITER, Event as JournalEvent, EventKind, LeafSpecCoverage,
    append_for,
};
use support::{change, change_with_deps, plan_with_changes};

struct Event;

impl Event {
    const fn event(timestamp: Timestamp, kind: EventKind) -> JournalEvent {
        JournalEvent::new(timestamp, kind)
    }
}

/// Stage `plan.yaml` at the project root.
fn write_plan(project: &Session, plan: &Plan) {
    let yaml = serde_saphyr::to_string(plan).expect("serialize plan");
    std::fs::write(project.root().join("plan.yaml"), yaml).expect("write plan.yaml");
}

/// Digest of the staged `plan.yaml`, as an epoch would stamp it.
fn live_plan_digest(root: &std::path::Path) -> String {
    let bytes = std::fs::read(root.join("plan.yaml")).expect("read plan.yaml");
    format!("sha256:{}", sha256_hex(&bytes))
}

/// Project the status body for `plan` staged inside `project`.
async fn status(project: &Session, plan: &Plan) -> StatusBody {
    write_plan(project, plan);
    run::<StatusOp, _, _>(project.provider(), StatusInput {}).await.expect("status")
}

/// Stage a live slice directory with optional abandon / refine / build
/// artifact signals (not lifecycle status — projection ignores that).
fn write_slice(root: &std::path::Path, name: &str, kind: SliceArt) {
    let slice_dir = root.join(".emery").join("slices").join(name);
    std::fs::create_dir_all(&slice_dir).expect("create slice dir");
    let mut meta = String::from("target: demo-target@1.0.0\n");
    match kind {
        SliceArt::Dropped => {
            meta.push_str("dropped-at: \"2024-01-01T00:00:00Z\"\n");
        }
        SliceArt::Refined => {
            std::fs::write(slice_dir.join("model.yaml"), "requirements: []\n")
                .expect("write model.yaml");
        }
        SliceArt::Built => {
            std::fs::write(slice_dir.join("model.yaml"), "requirements: []\n")
                .expect("write model.yaml");
            // Minimal fact-substrate build record (RFC-86 D27). Report
            // fields satisfy the closed BuildReport shape.
            let builds = slice_dir.join("builds");
            std::fs::create_dir_all(&builds).expect("create builds dir");
            std::fs::write(
                builds.join("aa.yaml"),
                "base: sha256:aa\n\
                 result: sha256:bb\n\
                 touched: []\n\
                 wave: sha256:cc\n\
                 report:\n\
                   version: 1\n\
                   slice: a\n\
                   target: demo-target@1.0.0\n\
                   status: success\n\
                   findings: []\n",
            )
            .expect("write build record");
        }
    }
    std::fs::write(slice_dir.join("metadata.yaml"), meta).expect("write metadata");
}

#[derive(Clone, Copy)]
enum SliceArt {
    Dropped,
    Refined,
    Built,
}

fn ts(seconds: i64) -> Timestamp {
    Timestamp::from_second(1_700_000_000 + seconds).expect("valid timestamp")
}

fn append(root: &std::path::Path, events: &[JournalEvent]) {
    append_for(Layout::new(root), DEFAULT_WRITER, events).expect("write journal events");
}

fn advanced(seconds: i64, plan: &str, slice: &str) -> JournalEvent {
    Event::event(
        ts(seconds),
        EventKind::PlanEntryAdvanced {
            plan_name: plan.into(),
            slice_name: slice.into(),
        },
    )
}

fn archived(seconds: i64, slice: &str) -> JournalEvent {
    Event::event(
        ts(seconds),
        EventKind::SliceArchiveCreated {
            slice_name: slice.into(),
            touched_specs: Vec::new(),
            outcome_summary: "merged".into(),
            merge_sha: None,
            decisions: Vec::new(),
        },
    )
}

fn build_failed(seconds: i64, slice: &str, reason: &str) -> JournalEvent {
    Event::event(
        ts(seconds),
        EventKind::SliceBuildFailed {
            slice_name: slice.into(),
            reason: reason.to_string(),
        },
    )
}

mod next_action {
    use super::*;

    #[tokio::test]
    async fn unmet_deps_stuck() {
        let project = Session::scripted("demo", Vec::new());
        let plan = plan_with_changes(vec![change_with_deps("b", &["missing"])]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop stuck");
    }

    #[tokio::test]
    async fn dropped_slice_stops() {
        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", SliceArt::Dropped);
        append(project.root(), &[advanced(0, "test", "a")]);
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop slice-dropped");
    }

    #[tokio::test]
    async fn fresh_plan() {
        // A fresh plan (nothing advanced, nothing done) resumes with
        // `/emery:execute`; the `resume:` line is the only
        // start-execution hint — no approval footer.
        let project = Session::scripted("demo", Vec::new());
        let body = status(&project, &plan_with_changes(vec![change("a")])).await;
        assert_eq!(body.resume.as_deref(), Some("/emery:execute"));
        let mut out = Vec::new();
        project::handler::Render::render(&body, &mut out).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        assert!(!text.contains("gate 1"), "no approval footer, got:\n{text}");
        assert!(!text.contains("pending review"), "no approval footer, got:\n{text}");
    }

    #[tokio::test]
    async fn drained_finalize() {
        // The drained projection and the literal stop-conditions
        // drained string, asserted through the text rendering.
        let project = Session::scripted("demo", Vec::new());
        append(project.root(), &[archived(0, "a")]);
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "drained");
        let mut out = Vec::new();
        project::handler::Render::render(&body, &mut out).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        assert!(
            text.contains("drained \u{2014} run /emery:finalize test"),
            "drained must render the literal finalize line, got:\n{text}"
        );
    }
}

mod failure_overlay {
    use super::*;

    #[tokio::test]
    async fn merge_failure_conflict() {
        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", SliceArt::Built);
        append(
            project.root(),
            &[
                advanced(0, "test", "a"),
                Event::event(
                    ts(10),
                    EventKind::SliceMergeFailed {
                        slice_name: "a".into(),
                        reason: "baseline conflict".to_string(),
                    },
                ),
            ],
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop merge-conflict");
    }

    #[tokio::test]
    async fn refine_failure_stops() {
        let project = Session::scripted("demo", Vec::new());
        append(
            project.root(),
            &[
                advanced(0, "test", "a"),
                Event::event(
                    ts(10),
                    EventKind::SliceSynthesizeFailed {
                        slice_name: "a".into(),
                        reason: "schema rejection".to_string(),
                    },
                ),
            ],
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop refine-failed");
    }

    #[tokio::test]
    async fn later_success_clears_failure() {
        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", SliceArt::Refined);
        append(
            project.root(),
            &[
                advanced(0, "test", "a"),
                build_failed(10, "a", "first attempt"),
                Event::event(
                    ts(20),
                    EventKind::SliceBuildSucceeded {
                        slice_name: "a".into(),
                    },
                ),
            ],
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(
            body.next_action, "merge a",
            "build.succeeded projects built — dispatch advances to merge"
        );
    }

    #[tokio::test]
    async fn non_awaited_failure_ignored() {
        // The slice already carries a built artifact; a stale build
        // failure must not pin the projection off merge.
        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", SliceArt::Built);
        append(project.root(), &[advanced(0, "test", "a"), build_failed(10, "a", "stale")]);
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "merge a");
    }

    #[tokio::test]
    async fn reclaim_shadows_old_failure() {
        // A fresh `plan.entry.advanced` (re-claim after undo, or a new
        // plan reusing the slice name) is newer than the failure, so
        // dispatch falls back to the artifact phase.
        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", SliceArt::Refined);
        append(project.root(), &[build_failed(0, "a", "old plan"), advanced(10, "test", "a")]);
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "build a");
    }

    #[tokio::test]
    async fn unstamped_merge_stops() {
        // Torn state: the merge landed (merge.succeeded) but the
        // archive / done stamp has not.
        let project = Session::scripted("demo", Vec::new());
        append(
            project.root(),
            &[
                advanced(0, "test", "a"),
                Event::event(
                    ts(10),
                    EventKind::SliceMergeSucceeded {
                        slice_name: "a".into(),
                    },
                ),
            ],
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop merge-incomplete");
    }

    #[tokio::test]
    async fn durable_merge_dominates() {
        // The merge landed (durable archive evidence in the claim
        // window); a later failed retry against the archived slice is
        // noise — the torn state still projects merge-incomplete, not
        // merge-conflict.
        let project = Session::scripted("demo", Vec::new());
        append(
            project.root(),
            &[
                advanced(0, "test", "a"),
                Event::event(
                    ts(10),
                    EventKind::SliceMergeSucceeded {
                        slice_name: "a".into(),
                    },
                ),
                Event::event(
                    ts(20),
                    EventKind::SliceMergeFailed {
                        slice_name: "a".into(),
                        reason: "retry against archived slice".to_string(),
                    },
                ),
            ],
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop merge-incomplete");
    }

    #[tokio::test]
    async fn pre_claim_skips_overlay() {
        // Stale same-name events (e.g. from an archived plan) must not
        // classify an entry that has not been claimed yet.
        let project = Session::scripted("demo", Vec::new());
        append(
            project.root(),
            &[
                archived(0, "a"),
                Event::event(
                    ts(0),
                    EventKind::SliceMergeSucceeded {
                        slice_name: "b".into(),
                    },
                ),
            ],
        );
        let plan = plan_with_changes(vec![change("a"), change("b")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "refine b");
    }
}

mod postflight_debt {
    use super::*;

    #[tokio::test]
    async fn sticky_stop_until_ack() {
        // After a non-rollback postflight failure the entry is `done`
        // — nothing in-progress — so status must stick on
        // `merge-postflight-failed` rather than projecting drained.
        let project = Session::scripted("demo", Vec::new());
        append(
            project.root(),
            &[Event::event(
                ts(10),
                EventKind::TargetMergeWavePostflightFailed {
                    target: "demo".into(),
                    digest:
                        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .into(),
                    slice_name: "a".into(),
                    reason: "target-merge-postflight-failed".to_string(),
                },
            )],
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop merge-postflight-failed");
        assert_eq!(body.slice.as_deref(), Some("a"));
        assert_eq!(body.current_step, None);
        assert_eq!(body.last_completed, Some(LoopStep::Merge));
        assert_eq!(body.resume.as_deref(), Some("emery plan execute"));
        assert!(
            body.stop.as_ref().is_some_and(
                |s| !s.hint.contains("in-progress") && !s.hint.contains("baseline conflict")
            ),
            "hint must not describe a retryable merge conflict: {:?}",
            body.stop.as_ref().map(|s| s.hint)
        );
    }

    #[tokio::test]
    async fn ack_clears_sticky_stop() {
        let project = Session::scripted("demo", Vec::new());
        append(
            project.root(),
            &[
                Event::event(
                    ts(10),
                    EventKind::TargetMergeWavePostflightFailed {
                        target: "demo".into(),
                        digest: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .into(),
                        slice_name: "a".into(),
                        reason: "target-merge-postflight-failed".to_string(),
                    },
                ),
                Event::event(
                    ts(20),
                    EventKind::PlanMergePostflightAcknowledged {
                        slice_name: "a".into(),
                    },
                ),
            ],
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "drained");
        assert_eq!(body.resume.as_deref(), Some("/emery:finalize test"));
    }

    #[tokio::test]
    async fn sticky_blocks_next_pending() {
        // Unacked postflight debt must not silently advance to the next
        // pending entry's refine.
        let project = Session::scripted("demo", Vec::new());
        append(
            project.root(),
            &[Event::event(
                ts(10),
                EventKind::TargetMergeWavePostflightFailed {
                    target: "demo".into(),
                    digest:
                        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .into(),
                    slice_name: "a".into(),
                    reason: "target-merge-postflight-failed".to_string(),
                },
            )],
        );
        let plan = plan_with_changes(vec![change("a"), change("b")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop merge-postflight-failed");
        assert_eq!(body.slice.as_deref(), Some("a"));
    }

    #[tokio::test]
    async fn ack_then_next_pending() {
        let project = Session::scripted("demo", Vec::new());
        append(
            project.root(),
            &[
                Event::event(
                    ts(10),
                    EventKind::TargetMergeWavePostflightFailed {
                        target: "demo".into(),
                        digest: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .into(),
                        slice_name: "a".into(),
                        reason: "target-merge-postflight-failed".to_string(),
                    },
                ),
                Event::event(
                    ts(20),
                    EventKind::PlanMergePostflightAcknowledged {
                        slice_name: "a".into(),
                    },
                ),
            ],
        );
        let plan = plan_with_changes(vec![change("a"), change("b")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "refine b");
    }
}

mod re_entry {
    use super::*;

    #[tokio::test]
    async fn merge_incomplete_done_stamp() {
        let project = Session::scripted("demo", Vec::new());
        append(
            project.root(),
            &[
                advanced(0, "test", "a"),
                Event::event(
                    ts(10),
                    EventKind::SliceMergeSucceeded {
                        slice_name: "a".into(),
                    },
                ),
            ],
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.current_step, Some(LoopStep::Merge));
        assert_eq!(body.last_completed, Some(LoopStep::Merge));
        assert_eq!(body.resume.as_deref(), Some("emery plan execute"));
    }

    #[tokio::test]
    async fn drained_finalize() {
        let project = Session::scripted("demo", Vec::new());
        append(project.root(), &[archived(0, "a")]);
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.current_step, None);
        assert_eq!(body.last_completed, None);
        assert_eq!(body.resume.as_deref(), Some("/emery:finalize test"));
    }

    #[tokio::test]
    async fn fresh_plan_resumes_execute() {
        // A fresh plan projects the real next action but resumes with
        // `/emery:execute` rather than a phase breakout — the loop, not
        // a single phase, is the natural entry point.
        let project = Session::scripted("demo", Vec::new());
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "refine a");
        assert_eq!(body.resume.as_deref(), Some("/emery:execute"));
    }

    #[tokio::test]
    async fn repair_stops_no_resume() {
        // `stuck` and `slice-dropped` need operator repair — no single
        // command makes progress, so `resume` stays empty.
        let project = Session::scripted("demo", Vec::new());
        let plan = plan_with_changes(vec![change_with_deps("b", &["missing"])]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop stuck");
        assert_eq!(body.resume, None);

        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", SliceArt::Dropped);
        append(project.root(), &[advanced(0, "test", "a")]);
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop slice-dropped");
        assert_eq!(body.current_step, None);
        assert_eq!(body.last_completed, None);
        assert_eq!(body.resume, None);
    }
}

mod milestones {
    use std::collections::BTreeMap;

    use super::*;

    fn write_model(root: &std::path::Path, name: &str, model: &str) {
        let slice_dir = root.join(".emery").join("slices").join(name);
        std::fs::create_dir_all(&slice_dir).expect("slice dir");
        std::fs::write(slice_dir.join("metadata.yaml"), "target: demo-target@1.0.0\n")
            .expect("metadata");
        std::fs::write(slice_dir.join("model.yaml"), model).expect("model");
    }

    #[tokio::test]
    async fn refined_clean_is_ready_not_authorized() {
        // Clean gaps + refined → Ready. No plan.execute.started yet →
        // not Authorized. Resume stays at execute (D22 / D26).
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-001
    title: login works
    statement: ''
    status: agreed
    sources: [intent]
",
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert!(body.ready, "clean refined plan must be Ready");
        assert!(!body.authorized, "no epoch yet → not Authorized");
        assert_eq!(body.next_action, "build a");
        assert_eq!(body.resume.as_deref(), Some("/emery:execute"));
        assert!(!serde_json::to_string(&body).expect("json").contains("approved"));
    }

    #[tokio::test]
    async fn open_unknowns_not_ready_build_proceeds() {
        // Refined + open unknowns → not Ready, but nothing blocks:
        // the projection keeps the build dispatch and resumes at the
        // execute loop, which defers open gaps at the gate.
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-003
    title: reset path not evidenced
    statement: ''
    status: unknown
    sources: [intent]
",
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert!(!body.ready);
        assert!(!body.authorized);
        assert_eq!(body.action, NextActionKind::Build);
        assert_eq!(body.next_action, "build a");
        assert_eq!(body.resume.as_deref(), Some("/emery:execute"));
        let mut out = Vec::new();
        project::handler::Render::render(&body, &mut out).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains("ready: false"));
        assert!(text.contains("authorized: false"));
        assert!(!text.contains("approved"), "never project approved: {text}");
    }

    #[tokio::test]
    async fn open_conflict_not_ready_build_proceeds() {
        // D6: `[conflict]` blocks Ready under the same semantics as
        // `[unknown]`, and dispatches build the same way — the gate
        // defers open conflicts too.
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-002
    title: auth disagree
    statement: ''
    status: conflict
    sources: [intent]
",
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert!(!body.ready);
        assert_eq!(body.next_action, "build a");
        assert_eq!(body.resume.as_deref(), Some("/emery:execute"));
    }

    #[tokio::test]
    async fn digest_less_legacy_rows_build_proceeds() {
        // A `spec.md`-fallback inventory (refined slice, model without
        // requirements) carries no requirement digests. It still
        // blocks Ready and dispatches build like any open row.
        let project = Session::scripted("demo", Vec::new());
        write_model(project.root(), "a", "requirements: []\n");
        let specs = project.root().join(".emery/slices/a/specs/auth");
        std::fs::create_dir_all(&specs).expect("specs dir");
        std::fs::write(
            specs.join("spec.md"),
            "### Requirement: reset path not evidenced [unknown]\n\
             ID: REQ-001\n\
             Sources: []\n\
             Status: unknown\n",
        )
        .expect("spec.md");
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert!(!body.ready);
        assert_eq!(body.next_action, "build a");
        assert_eq!(body.resume.as_deref(), Some("/emery:execute"));
    }

    /// Canonical digest of a title-only requirement body — the shape
    /// this suite's fixture models carry.
    fn title_digest(title: &str) -> String {
        project::slice::RequirementBody {
            title,
            statement: "",
            scenarios: &[],
            notes: None,
        }
        .digest()
    }

    fn gap_deferred(seconds: i64, slice: &str, req: &str, title: &str) -> JournalEvent {
        Event::event(
            ts(seconds),
            EventKind::GapDeferred {
                slice: slice.into(),
                req: req.into(),
                requirement_digest: title_digest(title),
                reason: "carried to next change".into(),
            },
        )
    }

    #[tokio::test]
    async fn deferred_everything_resumes_execute() {
        // A fully-dispositioned plan projects the build dispatch and
        // resumes at execute. Ready stays clean-only: the carried
        // debt keeps it false (D22).
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-003
    title: reset path not evidenced
    statement: ''
    status: unknown
    sources: [intent]
  - id: REQ-002
    title: auth disagree
    statement: ''
    status: conflict
    sources: [intent]
",
        );
        append(
            project.root(),
            &[
                gap_deferred(0, "a", "REQ-003", "reset path not evidenced"),
                gap_deferred(1, "a", "REQ-002", "auth disagree"),
            ],
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert!(!body.ready, "deferrals never contribute to Ready (D22)");
        assert_eq!(body.action, NextActionKind::Build);
        assert_eq!(body.next_action, "build a");
        assert_eq!(body.resume.as_deref(), Some("/emery:execute"));
        assert_eq!(
            body.debt,
            DebtCounts {
                unknown: 1,
                conflict: 1,
            }
        );
        let mut out = Vec::new();
        project::handler::Render::render(&body, &mut out).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        assert!(
            text.contains("debt: 2 deferred gaps (1 unknown, 1 conflict)"),
            "debt line with conflicts broken out, got:\n{text}"
        );
    }

    #[tokio::test]
    async fn deferred_beside_open_builds_and_counts_debt() {
        // A deferred row beside an open one: the open row blocks
        // Ready but not the build dispatch; only the deferred row
        // counts as debt.
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-003
    title: reset path not evidenced
    statement: ''
    status: unknown
    sources: [intent]
  - id: REQ-005
    title: reset copy not evidenced
    statement: ''
    status: unknown
    sources: [intent]
",
        );
        append(project.root(), &[gap_deferred(0, "a", "REQ-003", "reset path not evidenced")]);
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert!(!body.ready);
        assert_eq!(body.next_action, "build a");
        assert_eq!(body.resume.as_deref(), Some("/emery:execute"));
        assert_eq!(
            body.debt,
            DebtCounts {
                unknown: 1,
                conflict: 0,
            }
        );
    }

    #[tokio::test]
    async fn divergence_alone_does_not_block_ready() {
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-004
    title: authority chose
    statement: ''
    status: divergence
    sources: [intent]
",
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert!(body.ready, "divergence is listed but does not block Ready");
        assert_eq!(body.next_action, "build a");
    }

    #[tokio::test]
    async fn dropped_excluded_from_ready() {
        // Drop the gappy slice; the remaining refined sibling makes
        // the change Ready (D24).
        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", SliceArt::Dropped);
        write_model(
            project.root(),
            "b",
            r"requirements:
  - id: REQ-001
    title: ok
    statement: ''
    status: agreed
    sources: [intent]
",
        );
        let plan = plan_with_changes(vec![change("a"), change("b")]);
        let body = status(&project, &plan).await;
        assert!(body.ready);
        assert!(body.gaps.rows.is_empty());
    }

    /// Stamp a `plan.execute.started` epoch covering the staged
    /// `plan.yaml` with the given per-leaf coverage.
    fn stamp_epoch(root: &std::path::Path, specs: BTreeMap<String, LeafSpecCoverage>) {
        append(
            root,
            &[Event::event(
                ts(0),
                EventKind::PlanExecuteStarted {
                    coverage: ClosedPlanCoverage::ClosedPlan {
                        plan_digest: live_plan_digest(root),
                        specs,
                    },
                    discovery_digest: None,
                },
            )],
        );
    }

    #[tokio::test]
    async fn epoch_fact_projects_authorized_without_ready() {
        // A covering plan.execute.started → Authorized even while
        // unknowns keep Ready false (D22).
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-003
    title: reset path not evidenced
    statement: ''
    status: unknown
    sources: [intent]
",
        );
        let plan = plan_with_changes(vec![change("a")]);
        write_plan(&project, &plan);
        let mut specs = BTreeMap::new();
        specs.insert("a".into(), LeafSpecCoverage::RefineUnderEpoch);
        stamp_epoch(project.root(), specs);
        let body = status(&project, &plan).await;
        assert!(!body.ready, "an epoch must not backfill Ready");
        assert!(body.authorized);
        let json = serde_json::to_string(&body).expect("json");
        assert!(!json.contains("\"approved\""), "{json}");
    }

    #[tokio::test]
    async fn drifted_plan_digest_clears_authorized() {
        // An epoch whose plan digest no longer matches the live
        // `plan.yaml` does not authorize — same freshness rule as the
        // execute gap gate.
        let project = Session::scripted("demo", Vec::new());
        let plan = plan_with_changes(vec![change("a")]);
        write_plan(&project, &plan);
        let mut specs = BTreeMap::new();
        specs.insert("a".into(), LeafSpecCoverage::RefineUnderEpoch);
        append(
            project.root(),
            &[Event::event(
                ts(0),
                EventKind::PlanExecuteStarted {
                    coverage: ClosedPlanCoverage::ClosedPlan {
                        plan_digest:
                            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                                .into(),
                        specs,
                    },
                    discovery_digest: None,
                },
            )],
        );
        let body = status(&project, &plan).await;
        assert!(!body.authorized, "drifted plan digest must not authorize");
    }

    #[tokio::test]
    async fn covered_spec_drift_clears_authorized() {
        // Mutating a covered spec tree after the epoch stamps clears
        // Authorized while the old epoch remains in the union; build
        // refuses `plan-epoch-stale` on the same rule.
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-001
    title: login works
    statement: ''
    status: agreed
    sources: [intent]
",
        );
        let specs_dir = project.root().join(".emery/slices/a/specs");
        std::fs::create_dir_all(&specs_dir).expect("specs dir");
        std::fs::write(specs_dir.join("spec.md"), "# a\n").expect("spec.md");
        let plan = plan_with_changes(vec![change("a")]);
        write_plan(&project, &plan);
        let mut specs = BTreeMap::new();
        specs.insert(
            "a".into(),
            LeafSpecCoverage::Existing {
                digest: project::plan::dir_cid(&specs_dir).expect("specs cid").to_string(),
            },
        );
        stamp_epoch(project.root(), specs);

        let fresh = status(&project, &plan).await;
        assert!(fresh.authorized, "covering epoch authorizes");

        std::fs::write(specs_dir.join("spec.md"), "# a (drifted)\n").expect("mutate spec.md");
        let body = status(&project, &plan).await;
        assert!(!body.authorized, "covered-spec drift clears Authorized");

        let err = change::orchestrate::enforce_before_build(
            Layout::new(project.root()),
            &plan,
            "a",
            Timestamp::from_second(1_700_000_100).expect("timestamp"),
        )
        .expect_err("stale epoch refuses build");
        assert_eq!(err.variant_str(), "plan-epoch-stale");
    }

    #[tokio::test]
    async fn merged_leaf_absence_is_not_drift() {
        // Merge archives the slice tree; a done leaf's absent specs
        // are completion under the epoch, not drift.
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-001
    title: login works
    statement: ''
    status: agreed
    sources: [intent]
",
        );
        let slice_dir = project.root().join(".emery/slices/a");
        let specs_dir = slice_dir.join("specs");
        std::fs::create_dir_all(&specs_dir).expect("specs dir");
        std::fs::write(specs_dir.join("spec.md"), "# a\n").expect("spec.md");
        let plan = plan_with_changes(vec![change("a")]);
        write_plan(&project, &plan);
        let mut specs = BTreeMap::new();
        specs.insert(
            "a".into(),
            LeafSpecCoverage::Existing {
                digest: project::plan::dir_cid(&specs_dir).expect("specs cid").to_string(),
            },
        );
        stamp_epoch(project.root(), specs);
        append(project.root(), &[archived(10, "a")]);
        std::fs::remove_dir_all(&slice_dir).expect("archive removes slice tree");

        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "drained");
        assert!(body.authorized, "archived covered leaf keeps the epoch fresh");
    }

    #[tokio::test]
    async fn fresh_unrefined_resume_execute() {
        // D26: post-author resume stays /emery:execute; next-action
        // may still name the refine phase.
        let project = Session::scripted("demo", Vec::new());
        let body = status(&project, &plan_with_changes(vec![change("a")])).await;
        assert!(!body.ready);
        assert!(!body.authorized);
        assert_eq!(body.next_action, "refine a");
        assert_eq!(body.resume.as_deref(), Some("/emery:execute"));
    }
}
