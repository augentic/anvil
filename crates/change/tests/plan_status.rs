//! Integration coverage for the read-only `plan status` projection,
//! exercised through the `plan status` operation (the public
//! boundary): each test stages `plan.yaml`, slice metadata, and
//! journal events on disk, invokes the operation, and asserts the
//! projected `StatusBody`.
//!
//! The base happy-path dispatch arms (pending-stops,
//! fresh-active-refine, lifecycle refine/build/merge, drained,
//! eligible-pending preview) are asserted end-to-end through the
//! crate's orchestrate suites. What stays here is the dispatch and
//! overlay classification that has no CLI status fixture: stuck
//! dependency graphs, dropped slices, failure-overlay precedence, the
//! torn merge-incomplete state, re-entry resume points, and workspace
//! slot routing.

mod support;

use change::plan::handlers::{Status as StatusOp, StatusInput};
use change::{Lifecycle, LoopStep, Plan, Status, StatusBody};
use jiff::Timestamp;
use mock::invoke::run;
use mock::session::Session;
use project::journal::{Event as JournalEvent, EventKind};
use slice::LifecycleStatus;
use support::{change, change_with_deps, plan_with_changes};

struct Event;

impl Event {
    const fn event(timestamp: Timestamp, kind: EventKind) -> JournalEvent {
        JournalEvent { timestamp, kind }
    }
}

const fn approved(mut plan: Plan) -> Plan {
    plan.lifecycle = Lifecycle::Approved;
    plan
}

/// Stage `plan.yaml` at the project root.
fn write_plan(project: &Session, plan: &Plan) {
    let yaml = serde_saphyr::to_string(plan).expect("serialize plan");
    std::fs::write(project.root().join("plan.yaml"), yaml).expect("write plan.yaml");
}

/// Project the status body for `plan` staged inside `project`.
async fn status(project: &Session, plan: &Plan) -> StatusBody {
    write_plan(project, plan);
    run::<StatusOp, _, _>(project.provider(), StatusInput {}).await.expect("status")
}

fn write_slice(root: &std::path::Path, name: &str, status: LifecycleStatus) {
    let slice_dir = root.join(".specify").join("slices").join(name);
    std::fs::create_dir_all(&slice_dir).expect("create slice dir");
    let status = serde_saphyr::to_string(&status).expect("serialize lifecycle").trim().to_string();
    std::fs::write(
        slice_dir.join("metadata.yaml"),
        format!("target: demo-target@1.0.0\nstatus: {status}\n"),
    )
    .expect("write metadata");
}

fn ts(seconds: i64) -> Timestamp {
    Timestamp::from_second(1_700_000_000 + seconds).expect("valid timestamp")
}

fn append(root: &std::path::Path, events: &[JournalEvent]) {
    let body = events
        .iter()
        .map(|event| serde_json::to_string(event).expect("serialize journal event"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(root.join(".specify/journal.jsonl"), format!("{body}\n"))
        .expect("write journal events");
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
        let plan =
            approved(plan_with_changes(vec![change_with_deps("b", Status::Pending, &["missing"])]));
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop stuck");
    }

    #[tokio::test]
    async fn dropped_slice_stops() {
        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", LifecycleStatus::Dropped);
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop slice-dropped");
    }

    #[tokio::test]
    async fn drained_finalize_line() {
        // The drained projection and the literal stop-conditions
        // drained string, asserted through the text rendering.
        let project = Session::scripted("demo", Vec::new());
        let plan = approved(plan_with_changes(vec![change("a", Status::Done)]));
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "drained");
        let mut out = Vec::new();
        project::handler::Render::render(&body, &mut out).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        assert!(
            text.contains("drained \u{2014} run /spec:finalize test"),
            "drained must render the literal finalize line, got:\n{text}"
        );
    }
}

mod failure_overlay {
    use super::*;

    #[tokio::test]
    async fn merge_failure_conflict() {
        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", LifecycleStatus::Built);
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
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
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
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop refine-failed");
    }

    #[tokio::test]
    async fn later_success_clears_failure() {
        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", LifecycleStatus::Refined);
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
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "build a", "newest marker is a success — dispatch resumes");
    }

    #[tokio::test]
    async fn non_awaited_failure_ignored() {
        // The slice was hand-advanced past the failed phase; the stale
        // failure must not pin the projection.
        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", LifecycleStatus::Built);
        append(project.root(), &[advanced(0, "test", "a"), build_failed(10, "a", "stale")]);
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "merge a");
    }

    #[tokio::test]
    async fn reclaim_shadows_old_failure() {
        // A fresh `plan.entry.advanced` (re-claim after undo, or a new
        // plan reusing the slice name) is newer than the failure, so
        // dispatch falls back to the lifecycle.
        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", LifecycleStatus::Refined);
        append(project.root(), &[build_failed(0, "a", "old plan"), advanced(10, "test", "a")]);
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "build a");
    }

    #[tokio::test]
    async fn unstamped_merge_stops() {
        // Torn state: the merge landed (slice dir archived) but the
        // entry is still in-progress.
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
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
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
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
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
            &[Event::event(
                ts(0),
                EventKind::SliceMergeSucceeded {
                    slice_name: "b".into(),
                },
            )],
        );
        let plan = approved(plan_with_changes(vec![
            change("a", Status::Done),
            change("b", Status::Pending),
        ]));
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
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = status(&project, &plan).await;
        assert_eq!(body.current_step, Some(LoopStep::Merge));
        assert_eq!(body.last_completed, Some(LoopStep::Merge));
        assert_eq!(body.resume.as_deref(), Some("specify plan transition a done"));
    }

    #[tokio::test]
    async fn drained_finalize() {
        let project = Session::scripted("demo", Vec::new());
        let plan = approved(plan_with_changes(vec![change("a", Status::Done)]));
        let body = status(&project, &plan).await;
        assert_eq!(body.current_step, None);
        assert_eq!(body.last_completed, None);
        assert_eq!(body.resume.as_deref(), Some("/spec:finalize test"));
    }

    #[tokio::test]
    async fn gate_one_approved_stamp() {
        let project = Session::scripted("demo", Vec::new());
        let plan = plan_with_changes(vec![change("a", Status::Pending)]);
        let body = status(&project, &plan).await;
        assert_eq!(body.current_step, None);
        assert_eq!(body.resume.as_deref(), Some("specify plan approve"));
    }

    #[tokio::test]
    async fn repair_stops_no_resume() {
        // `stuck` and `slice-dropped` need operator repair — no single
        // command makes progress, so `resume` stays empty.
        let project = Session::scripted("demo", Vec::new());
        let plan =
            approved(plan_with_changes(vec![change_with_deps("b", Status::Pending, &["missing"])]));
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop stuck");
        assert_eq!(body.resume, None);

        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", LifecycleStatus::Dropped);
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop slice-dropped");
        assert_eq!(body.current_step, None);
        assert_eq!(body.last_completed, None);
        assert_eq!(body.resume, None);
    }
}

mod workspace_routing {
    use super::*;

    #[tokio::test]
    async fn entry_uses_slot_state() {
        let project = Session::scripted("demo", Vec::new());
        let slot = project.root().join("workspace").join("storefront");
        std::fs::create_dir_all(&slot).expect("create slot");
        write_slice(&slot, "a", LifecycleStatus::Refined);
        append(&slot, &[advanced(0, "test", "a"), build_failed(10, "a", "slot failure")]);

        let mut entry = change("a", Status::InProgress);
        entry.project = Some("storefront".to_string());
        let plan = approved(plan_with_changes(vec![entry]));
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "stop build-failed");
        assert_eq!(body.project.as_deref(), Some("storefront"));
    }

    // A bound slot that is not materialised falls back to the
    // project root's state.
    #[tokio::test]
    async fn missing_slot_falls_back() {
        let project = Session::scripted("demo", Vec::new());
        write_slice(project.root(), "a", LifecycleStatus::Built);
        let mut entry = change("a", Status::InProgress);
        entry.project = Some("storefront".to_string());
        let plan = approved(plan_with_changes(vec![entry]));
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "merge a");
    }
}
