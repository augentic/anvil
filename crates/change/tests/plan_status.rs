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
use change::{LoopStep, NextActionKind, Plan, StatusBody};
use jiff::Timestamp;
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{
    ClosedPlanCoverage, DEFAULT_WRITER, Event as JournalEvent, EventKind, append_for,
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

/// Project the status body for `plan` staged inside `project`.
async fn status(project: &Session, plan: &Plan) -> StatusBody {
    write_plan(project, plan);
    run::<StatusOp, _, _>(project.provider(), StatusInput {}).await.expect("status")
}

/// Stage a live slice directory with optional abandon / refine / build
/// artifact signals (not lifecycle status — projection ignores that).
/// RFC-91 D2: "refined" is a FRESH refinement manifest, so the refined
/// and built rungs stage one the status freshness recompute accepts.
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
            write_manifest(root, name);
        }
        SliceArt::Built => {
            std::fs::write(slice_dir.join("model.yaml"), "requirements: []\n")
                .expect("write model.yaml");
            write_manifest(root, name);
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

/// Stage a `refinement.yaml` the status projection's freshness
/// recompute accepts: planning projections computed from the same
/// minimal entry the suite's plans carry (no sources, no deps — plan
/// siblings never enter a leaf's digests), the live empty baseline,
/// and an empty bundle.
fn write_manifest(root: &std::path::Path, name: &str) {
    let plan = plan_with_changes(vec![change(name)]);
    let layout = Layout::new(root);
    let target = project::config::ProjectConfig::load(root).ok().and_then(|c| c.adapter);
    let planning =
        project::plan::Projections::compute(&plan, &plan.entries[0], &[], target.as_deref())
            .expect("projections");
    let manifest = slice::refinement::Manifest {
        version: slice::refinement::VERSION,
        slice: name.to_string(),
        inputs: slice::refinement::Inputs {
            planning: slice::refinement::Planning {
                entry: planning.entry,
                leads: planning.leads,
                decomposition: planning.decomposition,
            },
            profile: slice::refinement::empty_digest(),
            observations: slice::refinement::empty_digest(),
            target_guidance: slice::refinement::empty_digest(),
            baseline_specs: project::plan::dir_cid(&layout.specs_dir()).expect("dir cid"),
            sources: std::collections::BTreeMap::new(),
            dependencies: vec![],
        },
        bundle: vec![],
    };
    manifest.write(&layout.slice_dir(name)).expect("write refinement.yaml");
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
        // A fresh plan (nothing advanced, nothing refined) resumes
        // with `/emery:refine` (RFC-91 D8); the `resume:` line is the
        // only hint — no approval footer.
        let project = Session::scripted("demo", Vec::new());
        let body = status(&project, &plan_with_changes(vec![change("a")])).await;
        assert_eq!(body.resume.as_deref(), Some("/emery:refine"));
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
    async fn later_success_clears() {
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
    async fn non_awaited_failure() {
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
    async fn reclaim_shadows_old() {
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
                EventKind::MergeWavePostflightFailed {
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
                    EventKind::MergeWavePostflightFailed {
                        target: "demo".into(),
                        digest: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .into(),
                        slice_name: "a".into(),
                        reason: "target-merge-postflight-failed".to_string(),
                    },
                ),
                Event::event(
                    ts(20),
                    EventKind::PostflightAcknowledged {
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
    async fn sticky_blocks_next() {
        // Unacked postflight debt must not silently advance to the next
        // pending entry's refine.
        let project = Session::scripted("demo", Vec::new());
        append(
            project.root(),
            &[Event::event(
                ts(10),
                EventKind::MergeWavePostflightFailed {
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
                    EventKind::MergeWavePostflightFailed {
                        target: "demo".into(),
                        digest: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .into(),
                        slice_name: "a".into(),
                        reason: "target-merge-postflight-failed".to_string(),
                    },
                ),
                Event::event(
                    ts(20),
                    EventKind::PostflightAcknowledged {
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
    async fn merge_incomplete_done() {
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
    async fn fresh_plan_resumes() {
        // A fresh, unrefined plan projects `refine <slice>` and
        // resumes with `/emery:refine` — the refinement drain, not the
        // execute loop, is the natural entry point (RFC-91 D8).
        let project = Session::scripted("demo", Vec::new());
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert_eq!(body.next_action, "refine a");
        assert_eq!(body.resume.as_deref(), Some("/emery:refine"));
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

    /// Stage a refined slice: model + fresh refinement manifest
    /// (RFC-91 D2 — Ready requires every in-scope leaf FRESH).
    fn write_model(root: &std::path::Path, name: &str, model: &str) {
        let slice_dir = root.join(".emery").join("slices").join(name);
        std::fs::create_dir_all(&slice_dir).expect("slice dir");
        std::fs::write(slice_dir.join("metadata.yaml"), "target: demo-target@1.0.0\n")
            .expect("metadata");
        std::fs::write(slice_dir.join("model.yaml"), model).expect("model");
        write_manifest(root, name);
    }

    #[tokio::test]
    async fn refined_clean_ready() {
        // Clean gaps + refined → Ready. No plan.execute.started yet →
        // not Authorized. Resume stays at execute (D22 / D26).
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-001
    title: login works
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
    async fn open_unknowns_ready() {
        // Refined + open unknowns → not Ready; next-action is
        // review-gaps; resume points at per-req --waive (D22).
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-003
    title: reset path not evidenced
    status: unknown
    sources: [intent]
",
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert!(!body.ready);
        assert!(!body.authorized);
        assert_eq!(body.action, NextActionKind::ReviewGaps);
        assert_eq!(body.next_action, "review-gaps");
        let resume = body.resume.as_deref().expect("resume");
        assert!(
            resume.contains("emery plan execute")
                && resume.contains("--waive a/REQ-003")
                && resume.contains("--reason"),
            "resume must suggest waive path, got: {resume}"
        );
        let mut out = Vec::new();
        project::handler::Render::render(&body, &mut out).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains("ready: false"));
        assert!(text.contains("authorized: false"));
        assert!(!text.contains("approved"), "never project approved: {text}");
    }

    #[tokio::test]
    async fn conflict_resume_re_refine() {
        // Conflicts resume at `emery plan refine` — fix inputs and
        // re-refine (RFC-91 D8), never waive through execute.
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-002
    title: auth disagree
    status: conflict
    sources: [intent]
",
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert!(!body.ready);
        assert_eq!(body.next_action, "review-gaps");
        assert_eq!(body.resume.as_deref(), Some("emery plan refine"));
    }

    #[tokio::test]
    async fn divergence_alone_block() {
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-004
    title: authority chose
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
    async fn dropped_excluded_ready() {
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
    status: agreed
    sources: [intent]
",
        );
        let plan = plan_with_changes(vec![change("a"), change("b")]);
        let body = status(&project, &plan).await;
        assert!(body.ready);
        assert!(body.gaps.rows.is_empty());
    }

    #[tokio::test]
    async fn epoch_fact_projects() {
        // Hand-stamped plan.execute.started → Authorized even while
        // unknowns keep Ready false (D22). Execute writer is S18.
        let project = Session::scripted("demo", Vec::new());
        write_model(
            project.root(),
            "a",
            r"requirements:
  - id: REQ-003
    title: reset path not evidenced
    status: unknown
    sources: [intent]
",
        );
        let mut refinements = BTreeMap::new();
        refinements.insert(
            "a".into(),
            project::snapshot::SnapshotId::parse(
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            )
            .expect("digest"),
        );
        append(
            project.root(),
            &[Event::event(
                ts(0),
                EventKind::PlanExecuteStarted {
                    coverage: ClosedPlanCoverage::ClosedPlan {
                        plan_digest: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                        refinements,
                        unknown_waivers: Vec::new(),
                    },
                    discovery_digest: None,
                },
            )],
        );
        let plan = plan_with_changes(vec![change("a")]);
        let body = status(&project, &plan).await;
        assert!(!body.ready, "waivers/epoch must not backfill Ready");
        assert!(body.authorized);
        let json = serde_json::to_string(&body).expect("json");
        assert!(!json.contains("\"approved\""), "{json}");
    }

    #[tokio::test]
    async fn fresh_unrefined_resume() {
        // RFC-91 D8: post-author resume is /emery:refine — refinement
        // is a first-class stage between authoring and execution.
        let project = Session::scripted("demo", Vec::new());
        let body = status(&project, &plan_with_changes(vec![change("a")])).await;
        assert!(!body.ready);
        assert!(!body.authorized);
        assert_eq!(body.next_action, "refine a");
        assert_eq!(body.resume.as_deref(), Some("/emery:refine"));
    }
}
