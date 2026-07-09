use jiff::Timestamp;
use tempfile::TempDir;

use super::super::{change, change_with_deps, plan_with_changes};
use super::*;
use crate::journal::{Event, append_batch};
use crate::slice::SliceMetadata;

fn approved(mut plan: Plan) -> Plan {
    plan.lifecycle = Lifecycle::Approved;
    plan
}

fn write_slice(root: &std::path::Path, name: &str, status: LifecycleStatus) {
    let slice_dir = root.join(".specify").join("slices").join(name);
    std::fs::create_dir_all(&slice_dir).expect("create slice dir");
    let metadata = SliceMetadata {
        target: "demo-target@1.0.0".to_string(),
        status,
        created_at: None,
        defined_at: None,
        completed_at: None,
        merged_at: None,
        dropped_at: None,
        drop_reason: None,
        touched_specs: vec![],
        outcome: None,
    };
    metadata.save(&slice_dir).expect("write metadata");
}

fn ts(seconds: i64) -> Timestamp {
    Timestamp::from_second(1_700_000_000 + seconds).expect("valid timestamp")
}

fn append(root: &std::path::Path, events: &[Event]) {
    append_batch(Layout::new(root), events).expect("append journal events");
}

fn advanced(seconds: i64, plan: &str, slice: &str) -> Event {
    Event::new(
        ts(seconds),
        EventKind::PlanEntryAdvanced {
            plan_name: plan.into(),
            slice_name: slice.into(),
        },
    )
}

fn build_failed(seconds: i64, slice: &str, reason: &str) -> Event {
    Event::new(
        ts(seconds),
        EventKind::SliceBuildFailed {
            slice_name: slice.into(),
            reason: reason.to_string(),
        },
    )
}

mod next_action {
    use super::*;

    // The base happy-path dispatch arms (pending-stops, fresh-active-refine,
    // lifecycle refine/build/merge, drained, eligible-pending preview) are
    // asserted end-to-end through the parked composed-deployment rig in
    // `core/tests/`. What stays here is the dispatch that has no
    // CLI status fixture: a stuck dependency graph and a dropped slice.

    #[test]
    fn stuck_when_deps_unmet() {
        let dir = TempDir::new().expect("tempdir");
        let plan =
            approved(plan_with_changes(vec![change_with_deps("b", Status::Pending, &["missing"])]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.next_action, "stop stuck");
    }

    #[test]
    fn dropped_slice_stops() {
        let dir = TempDir::new().expect("tempdir");
        write_slice(dir.path(), "a", LifecycleStatus::Dropped);
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.next_action, "stop slice-dropped");
    }
}

mod failure_overlay {
    use super::*;

    // The build-failure overlay is asserted end-to-end through the parked
    // composed-deployment rig in `core/tests/`. The remaining
    // overlay classifications below have no CLI status fixture:
    // merge-conflict / refine-failed mapping, newest-marker precedence,
    // stale / pre-claim shadowing, and the torn merge-incomplete state.

    #[test]
    fn merge_failure_maps_to_conflict() {
        let dir = TempDir::new().expect("tempdir");
        write_slice(dir.path(), "a", LifecycleStatus::Built);
        append(
            dir.path(),
            &[
                advanced(0, "test", "a"),
                Event::new(
                    ts(10),
                    EventKind::SliceMergeFailed {
                        slice_name: "a".into(),
                        reason: "baseline conflict".to_string(),
                    },
                ),
            ],
        );
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.next_action, "stop merge-conflict");
    }

    #[test]
    fn refine_failure_stops() {
        let dir = TempDir::new().expect("tempdir");
        append(
            dir.path(),
            &[
                advanced(0, "test", "a"),
                Event::new(
                    ts(10),
                    EventKind::SliceSynthesizeFailed {
                        slice_name: "a".into(),
                        reason: "schema rejection".to_string(),
                    },
                ),
            ],
        );
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.next_action, "stop refine-failed");
    }

    #[test]
    fn later_success_clears_failure() {
        let dir = TempDir::new().expect("tempdir");
        write_slice(dir.path(), "a", LifecycleStatus::Refined);
        append(
            dir.path(),
            &[
                advanced(0, "test", "a"),
                build_failed(10, "a", "first attempt"),
                Event::new(
                    ts(20),
                    EventKind::SliceBuildSucceeded {
                        slice_name: "a".into(),
                    },
                ),
            ],
        );
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.next_action, "build a", "newest marker is a success — dispatch resumes");
    }

    #[test]
    fn non_awaited_failure_ignored() {
        // The slice was hand-advanced past the failed phase; the stale
        // failure must not pin the projection.
        let dir = TempDir::new().expect("tempdir");
        write_slice(dir.path(), "a", LifecycleStatus::Built);
        append(dir.path(), &[advanced(0, "test", "a"), build_failed(10, "a", "stale")]);
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.next_action, "merge a");
    }

    #[test]
    fn reclaim_shadows_old_failure() {
        // A fresh `plan.entry.advanced` (re-claim after undo, or a new
        // plan reusing the slice name) is newer than the failure, so
        // dispatch falls back to the lifecycle.
        let dir = TempDir::new().expect("tempdir");
        write_slice(dir.path(), "a", LifecycleStatus::Refined);
        append(dir.path(), &[build_failed(0, "a", "old plan"), advanced(10, "test", "a")]);
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.next_action, "build a");
    }

    #[test]
    fn merge_succeeded_without_stamp_stops() {
        // Torn state: the merge landed (slice dir archived) but the
        // entry is still in-progress.
        let dir = TempDir::new().expect("tempdir");
        append(
            dir.path(),
            &[
                advanced(0, "test", "a"),
                Event::new(
                    ts(10),
                    EventKind::SliceMergeSucceeded {
                        slice_name: "a".into(),
                    },
                ),
            ],
        );
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.next_action, "stop merge-incomplete");
    }

    #[test]
    fn pre_claim_candidate_skips_overlay() {
        // Stale same-name events (e.g. from an archived plan) must not
        // classify an entry that has not been claimed yet.
        let dir = TempDir::new().expect("tempdir");
        append(
            dir.path(),
            &[Event::new(
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
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.next_action, "refine b");
    }
}

mod re_entry {
    use super::*;

    // The dispatch + resume projection for the fresh-refine and
    // refined/build-failed scenarios is exercised end-to-end through the
    // parked composed-deployment rig in `core/tests/`. The re-entry
    // overlays with no CLI fixture remain: the torn merge-incomplete
    // resume, the drained finalize resume, the Gate-1 approved-stamp
    // resume, and the repair-shaped stops that carry no resume.

    #[test]
    fn merge_incomplete_resumes_at_done_stamp() {
        let dir = TempDir::new().expect("tempdir");
        append(
            dir.path(),
            &[
                advanced(0, "test", "a"),
                Event::new(
                    ts(10),
                    EventKind::SliceMergeSucceeded {
                        slice_name: "a".into(),
                    },
                ),
            ],
        );
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.current_step, Some(LoopStep::Merge));
        assert_eq!(body.last_completed, Some(LoopStep::Merge));
        assert_eq!(body.resume.as_deref(), Some("specify plan transition a done"));
    }

    #[test]
    fn drained_resumes_at_finalize() {
        let dir = TempDir::new().expect("tempdir");
        let plan = approved(plan_with_changes(vec![change("a", Status::Done)]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.current_step, None);
        assert_eq!(body.last_completed, None);
        assert_eq!(body.resume.as_deref(), Some("/spec:finalize test"));
    }

    #[test]
    fn gate_one_resumes_at_approved_stamp() {
        let dir = TempDir::new().expect("tempdir");
        let plan = plan_with_changes(vec![change("a", Status::Pending)]);
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.current_step, None);
        assert_eq!(body.resume.as_deref(), Some("specify plan transition test approved"));
    }

    #[test]
    fn repair_shaped_stops_have_no_resume() {
        // `stuck` and `slice-dropped` need operator repair — no single
        // command makes progress, so `resume` stays empty.
        let dir = TempDir::new().expect("tempdir");
        let plan =
            approved(plan_with_changes(vec![change_with_deps("b", Status::Pending, &["missing"])]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.next_action, "stop stuck");
        assert_eq!(body.resume, None);

        let dir = TempDir::new().expect("tempdir");
        write_slice(dir.path(), "a", LifecycleStatus::Dropped);
        let plan = approved(plan_with_changes(vec![change("a", Status::InProgress)]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.next_action, "stop slice-dropped");
        assert_eq!(body.current_step, None);
        assert_eq!(body.last_completed, None);
        assert_eq!(body.resume, None);
    }
}

mod workspace_routing {
    use super::*;

    #[test]
    fn slot_bound_entry_reads_slot_state() {
        let dir = TempDir::new().expect("tempdir");
        let slot = dir.path().join("workspace").join("storefront");
        std::fs::create_dir_all(&slot).expect("create slot");
        write_slice(&slot, "a", LifecycleStatus::Refined);
        append(&slot, &[advanced(0, "test", "a"), build_failed(10, "a", "slot failure")]);

        let mut entry = change("a", Status::InProgress);
        entry.project = Some("storefront".to_string());
        let plan = approved(plan_with_changes(vec![entry]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.next_action, "stop build-failed");
        assert_eq!(body.project.as_deref(), Some("storefront"));
    }

    #[test]
    fn missing_slot_falls_back_to_project_root() {
        let dir = TempDir::new().expect("tempdir");
        write_slice(dir.path(), "a", LifecycleStatus::Built);
        let mut entry = change("a", Status::InProgress);
        entry.project = Some("storefront".to_string());
        let plan = approved(plan_with_changes(vec![entry]));
        let body = plan_status_body(&plan, Layout::new(dir.path())).expect("status");
        assert_eq!(body.next_action, "merge a");
    }
}

#[test]
fn drained_line_renders_literal() {
    assert_eq!(drained_line("platform-v2"), "drained — run /spec:finalize platform-v2");
}
