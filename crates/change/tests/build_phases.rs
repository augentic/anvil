//! RFC-90 Acceptance #10 (native rung) — the engine-owned build phase
//! machine over the mock catalog, driven through the public
//! `slice::orchestrate::build` path: phase sequences, repair routing
//! and origins, both budget exhaustions, artifact promotion and scope
//! violation, and failure-path invariants.

mod support;

use mock::behaviour::{
    REVIEW_BLOCKED_MARKER, REVIEW_FIXABLE_MARKER, REVIEW_REPAIRED, VERIFICATION_REPAIRED,
    VERIFY_AFTER_REVIEW_FAIL_MARKER, VERIFY_BLOCKED_MARKER, VERIFY_FIXABLE_MARKER,
};
use mock::session::Session;
use project::build_record::BuildRecord;
use project::config::Layout;
use project::journal::{EventKind, read_union};
use project::seam::PhaseSource;
use project::slice::SliceMetadata;
use slice::{BuildStatus, LifecycleStatus};

/// Author + refine the greeting fixture against `target_adapter`.
async fn ready(target_adapter: &str) -> Session {
    let session = Session::scripted(
        target_adapter,
        vec![mock::answers::greeting_grouping(), mock::answers::greeting_synthesis()],
    );
    support::greeting_ready(&session).await;
    session
}

fn slice_dir(session: &Session) -> std::path::PathBuf {
    session.root().join(".emery/slices/greeting")
}

fn operations(events: &[(u32, u32, String, String)]) -> Vec<&str> {
    events.iter().map(|(_, _, operation, _)| operation.as_str()).collect()
}

fn lifecycle(session: &Session) -> LifecycleStatus {
    let dir = slice_dir(session);
    let metadata = SliceMetadata::load(&dir).expect("metadata");
    LifecycleStatus::project(&dir, &metadata)
}

fn repair_written(session: &Session, attempt: u32, file: &str) -> String {
    let path = support::attempt_dir(session.root(), "greeting", attempt).join("phases").join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("`{}`: {err}", path.display()))
}

mod success {
    use super::*;

    /// The clean pass: `build → verify → review`, visible in the
    /// attempt tree, the ordered phase-completed facts, the terminal
    /// projection, the `BuildRecord`, and the promoted stage change.
    #[tokio::test]
    async fn phase_sequence_and_promotion() {
        let session = ready("mock").await;
        let root = session.root().to_path_buf();
        let dir = slice_dir(&session);
        let tasks_before = std::fs::read_to_string(dir.join("tasks.md")).expect("tasks.md");

        let outcome = support::build(&session, "greeting").await.expect("build");

        assert_eq!(outcome.status, BuildStatus::Success);
        assert_eq!(outcome.verification, Some(PhaseSource::Deterministic));

        assert_eq!(
            support::phase_files(&root, "greeting", 1),
            ["01-build.yaml", "02-verify.yaml", "03-review.yaml"]
        );
        let events = support::phase_events(&root);
        assert_eq!(
            events,
            [
                (1, 1, "build".to_string(), "deterministic".to_string()),
                (1, 2, "verify".to_string(), "deterministic".to_string()),
                (1, 3, "review".to_string(), "deterministic".to_string()),
            ]
        );

        let report = support::canonical_report(&root, "greeting");
        assert_eq!(report.status, BuildStatus::Success);
        assert_eq!(report.slice, "greeting");
        assert_eq!(report.target, "mock@0.0.0", "the resolved identity, not the bare name");
        assert!(BuildRecord::present(&dir), "terminal success writes the BuildRecord");
        assert_eq!(lifecycle(&session), LifecycleStatus::Built);

        let attempt = support::attempt_dir(&root, "greeting", 1);
        assert!(attempt.join("request.yaml").is_file(), "request copied into the attempt");
        assert!(attempt.join("report.yaml").is_file(), "terminal report beside phases/");
        assert!(!attempt.join("stage").exists(), "stage discarded on the terminal path");

        // The staged `tasks.md` grant landed on the authoritative
        // slice tree — exactly the granted change, nothing else.
        let tasks = std::fs::read_to_string(dir.join("tasks.md")).expect("tasks.md");
        assert_eq!(tasks, format!("{tasks_before}- [x] 99.1 built greeting\n"));
    }

    /// Every phase-completed digest is `sha256:` over the exact
    /// persisted phase-report bytes.
    #[tokio::test]
    async fn event_digests_match_phase_files() {
        let session = ready("mock").await;
        let root = session.root().to_path_buf();

        support::build(&session, "greeting").await.expect("build");

        let digests: Vec<(u32, String, String)> = read_union(Layout::new(&root))
            .expect("union")
            .into_iter()
            .filter_map(|event| match event.kind {
                EventKind::SliceBuildPhaseCompleted {
                    ordinal,
                    operation,
                    report_digest,
                    ..
                } => Some((ordinal, operation, report_digest)),
                _ => None,
            })
            .collect();
        assert_eq!(digests.len(), 3);
        for (ordinal, operation, digest) in digests {
            let path = support::attempt_dir(&root, "greeting", 1)
                .join("phases")
                .join(format!("{ordinal:02}-{operation}.yaml"));
            let bytes = std::fs::read(&path).expect("phase bytes");
            assert_eq!(digest, format!("sha256:{}", diagnostics::digest::sha256_hex(&bytes)));
        }
    }
}

mod repair_rounds {
    use super::*;

    /// One verification-repair round: blocking verify routes
    /// `repair(origin: verification)`, then verify re-runs clean and
    /// review completes the attempt.
    #[tokio::test]
    async fn verification_repair() {
        let session = ready("mock").await;
        let root = session.root().to_path_buf();
        support::marker(&root, VERIFY_FIXABLE_MARKER);

        let outcome = support::build(&session, "greeting").await.expect("build");

        assert_eq!(outcome.status, BuildStatus::Success);
        assert_eq!(
            support::phase_files(&root, "greeting", 1),
            [
                "01-build.yaml",
                "02-verify.yaml",
                "03-repair.yaml",
                "04-verify.yaml",
                "05-review.yaml",
            ]
        );
        // The repair report's audit `written` names the
        // verification-origin sentinel.
        let repair = repair_written(&session, 1, "03-repair.yaml");
        assert!(repair.contains(VERIFICATION_REPAIRED), "{repair}");
        assert!(BuildRecord::present(&slice_dir(&session)));
    }

    /// One review-remediation round: blocking review routes
    /// `repair(origin: review)`, then the machine re-enters
    /// verification before the clean review.
    #[tokio::test]
    async fn review_remediation() {
        let session = ready("mock").await;
        let root = session.root().to_path_buf();
        support::marker(&root, REVIEW_FIXABLE_MARKER);

        let outcome = support::build(&session, "greeting").await.expect("build");

        assert_eq!(outcome.status, BuildStatus::Success);
        assert_eq!(
            support::phase_files(&root, "greeting", 1),
            [
                "01-build.yaml",
                "02-verify.yaml",
                "03-review.yaml",
                "04-repair.yaml",
                "05-verify.yaml",
                "06-review.yaml",
            ]
        );
        let repair = repair_written(&session, 1, "04-repair.yaml");
        assert!(repair.contains(REVIEW_REPAIRED), "{repair}");
        assert!(BuildRecord::present(&slice_dir(&session)));
    }

    /// A failed verification after review repair consumes the shared
    /// verification budget: one review remediation, then three
    /// verification repairs, then the blocking terminal.
    #[tokio::test]
    async fn post_review_verification_failure() {
        let session = ready("mock").await;
        let root = session.root().to_path_buf();
        support::marker(&root, REVIEW_FIXABLE_MARKER);
        support::marker(&root, VERIFY_AFTER_REVIEW_FAIL_MARKER);

        let err = support::build(&session, "greeting").await.expect_err("budget exhausts");
        assert_eq!(err.variant_str(), "target-build-failed");

        let events = support::phase_events(&root);
        assert_eq!(
            operations(&events),
            [
                "build", "verify", "review", "repair", "verify", "repair", "verify", "repair",
                "verify", "repair", "verify",
            ],
            "one review remediation, then the shared verification budget: {events:?}"
        );
        // Both repair origins are visible in the persisted phase
        // reports: the remediation wrote the review sentinel, the
        // verification repairs the verification sentinel.
        let remediation = repair_written(&session, 1, "04-repair.yaml");
        assert!(remediation.contains(REVIEW_REPAIRED), "{remediation}");
        let verification = repair_written(&session, 1, "06-repair.yaml");
        assert!(verification.contains(VERIFICATION_REPAIRED), "{verification}");
        assert!(!BuildRecord::present(&slice_dir(&session)));
        assert_ne!(lifecycle(&session), LifecycleStatus::Built);
    }
}

mod budgets {
    use super::*;

    /// Verification-budget exhaustion: exactly three repair
    /// dispatches — never a fourth — then the typed blocking
    /// terminal with the verify findings on the canonical report.
    #[tokio::test]
    async fn verification_exhaustion() {
        let session = ready("mock").await;
        let root = session.root().to_path_buf();
        support::marker(&root, VERIFY_BLOCKED_MARKER);

        let err = support::build(&session, "greeting").await.expect_err("budget exhausts");
        assert_eq!(err.variant_str(), "target-build-failed");

        let phases = support::phase_files(&root, "greeting", 1);
        assert_eq!(
            phases,
            [
                "01-build.yaml",
                "02-verify.yaml",
                "03-repair.yaml",
                "04-verify.yaml",
                "05-repair.yaml",
                "06-verify.yaml",
                "07-repair.yaml",
                "08-verify.yaml",
            ]
        );
        let repairs = phases.iter().filter(|name| name.ends_with("-repair.yaml")).count();
        assert_eq!(repairs, 3, "the fourth verification repair is never dispatched");

        // The failed canonical projection carries the latest blocking
        // verify findings; no BuildRecord, lifecycle un-Built.
        let report = support::canonical_report(&root, "greeting");
        assert_eq!(report.status, BuildStatus::Failure);
        assert!(
            report.findings.iter().any(|finding| finding.title.contains("verify-blocked")),
            "{:?}",
            report.findings
        );
        assert!(!BuildRecord::present(&slice_dir(&session)));
        assert_ne!(lifecycle(&session), LifecycleStatus::Built);
        assert!(!support::attempt_dir(&root, "greeting", 1).join("stage").exists());

        let failed =
            read_union(Layout::new(&root)).expect("union").into_iter().find_map(
                |event| match event.kind {
                    EventKind::SliceBuildFailed { reason, .. } => Some(reason),
                    _ => None,
                },
            );
        assert_eq!(failed.as_deref(), Some("target-build-failed"), "slice.build.failed journaled");
    }

    /// Review-budget exhaustion: exactly one remediation dispatch —
    /// never a second — then the typed blocking terminal.
    #[tokio::test]
    async fn review_exhaustion() {
        let session = ready("mock").await;
        let root = session.root().to_path_buf();
        support::marker(&root, REVIEW_BLOCKED_MARKER);

        let err = support::build(&session, "greeting").await.expect_err("budget exhausts");
        assert_eq!(err.variant_str(), "target-build-failed");

        let phases = support::phase_files(&root, "greeting", 1);
        assert_eq!(
            phases,
            [
                "01-build.yaml",
                "02-verify.yaml",
                "03-review.yaml",
                "04-repair.yaml",
                "05-verify.yaml",
                "06-review.yaml",
            ]
        );
        let repairs = phases.iter().filter(|name| name.ends_with("-repair.yaml")).count();
        assert_eq!(repairs, 1, "the second review remediation is never dispatched");

        let report = support::canonical_report(&root, "greeting");
        assert_eq!(report.status, BuildStatus::Failure);
        assert!(
            report.findings.iter().any(|finding| finding.title.contains("review-blocked")),
            "{:?}",
            report.findings
        );
        assert!(!BuildRecord::present(&slice_dir(&session)));
        assert_ne!(lifecycle(&session), LifecycleStatus::Built);
        assert!(!support::attempt_dir(&root, "greeting", 1).join("stage").exists());
    }
}

mod gate_profiles {
    use super::*;

    /// Drive one gate-invalid verify profile to its typed terminal:
    /// engine-authored finding on the failed canonical report, stage
    /// discarded, no `BuildRecord`, lifecycle un-Built.
    async fn rejects(adapter: &str, code: &str) {
        let session = ready(adapter).await;
        let root = session.root().to_path_buf();
        let dir = slice_dir(&session);

        let err = support::build(&session, "greeting").await.expect_err("gate rejects");
        assert_eq!(err.variant_str(), code);

        let report = support::canonical_report(&root, "greeting");
        assert_eq!(report.status, BuildStatus::Failure);
        assert!(
            report.findings.iter().any(|finding| finding.rule_id.as_deref() == Some(code)),
            "engine-authored terminal finding present: {:?}",
            report.findings
        );
        assert!(!support::attempt_dir(&root, "greeting", 1).join("stage").exists());
        assert!(!BuildRecord::present(&dir));
        assert_ne!(lifecycle(&session), LifecycleStatus::Built);
    }

    #[tokio::test]
    async fn tool_source() {
        rejects("mock-tool-source", "target-phase-source-tool").await;
    }

    #[tokio::test]
    async fn verify_outputs() {
        rejects("mock-verify-outputs", "target-phase-output-declaration").await;
    }

    #[tokio::test]
    async fn na_blocking() {
        rejects("mock-na-blocking", "target-phase-not-applicable-dirty").await;
    }

    #[tokio::test]
    async fn verify_continuation() {
        rejects("mock-verify-continuation", "target-phase-verify-continuation").await;
    }
}

mod round_isolation {
    use super::*;

    /// RFC-90 AC7: the terminal report carries only the latest verify
    /// round's findings — superseded rounds and every repair report
    /// stay attempt-local evidence. The mock round-tags each verify /
    /// repair finding under the blocked profile so dedupe cannot hide
    /// a leak.
    #[tokio::test]
    async fn terminal_carries_final_round_only() {
        let session = ready("mock").await;
        let root = session.root().to_path_buf();
        support::marker(&root, VERIFY_BLOCKED_MARKER);

        let err = support::build(&session, "greeting").await.expect_err("budget exhausts");
        assert_eq!(err.variant_str(), "target-build-failed");

        // The repair reports carried their own round-tagged findings
        // — persisted attempt evidence.
        let repair = repair_written(&session, 1, "03-repair.yaml");
        assert!(repair.contains("repair pass after round 1"), "{repair}");

        // Four verify rounds ran (initial + one per repair); the
        // canonical terminal report carries the final round alone and
        // no repair findings.
        let report = support::canonical_report(&root, "greeting");
        let titles: Vec<&str> =
            report.findings.iter().map(|finding| finding.title.as_str()).collect();
        assert!(titles.iter().any(|title| title.contains("(round 4)")), "{titles:?}");
        assert!(
            !titles.iter().any(|title| {
                ["(round 1)", "(round 2)", "(round 3)"].iter().any(|round| title.contains(round))
            }),
            "superseded verify rounds leaked: {titles:?}"
        );
        assert!(
            !titles.iter().any(|title| title.contains("repair pass")),
            "repair findings leaked: {titles:?}"
        );
    }
}

mod artifact_scope {
    use super::*;

    /// A staged write outside the declared grants terminates the
    /// attempt: typed scope violation, authoritative slice tree
    /// untouched, stage discarded, failed canonical projection.
    #[tokio::test]
    async fn stage_escape_rejected() {
        let session = ready("mock-stage-escape").await;
        let root = session.root().to_path_buf();
        let dir = slice_dir(&session);
        let tasks_before = std::fs::read_to_string(dir.join("tasks.md")).expect("tasks.md");

        let err = support::build(&session, "greeting").await.expect_err("scope violation");
        assert_eq!(err.variant_str(), "target-build-artifact-scope-violation");

        assert!(!dir.join("undeclared.md").exists(), "escaped write never promoted");
        assert_eq!(
            std::fs::read_to_string(dir.join("tasks.md")).expect("tasks.md"),
            tasks_before,
            "authoritative slice tree unchanged"
        );
        assert!(!support::attempt_dir(&root, "greeting", 1).join("stage").exists());

        let report = support::canonical_report(&root, "greeting");
        assert_eq!(report.status, BuildStatus::Failure);
        assert!(
            report.findings.iter().any(|finding| {
                finding.rule_id.as_deref() == Some("target-build-artifact-scope-violation")
            }),
            "engine-authored terminal finding present: {:?}",
            report.findings
        );
        assert!(!BuildRecord::present(&dir));
        assert_ne!(lifecycle(&session), LifecycleStatus::Built);
    }
}
