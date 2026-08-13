//! RFC-90 Acceptance #10 (native rung) — attempt-scoped continuation
//! semantics (persist / preserve / replace / clear / size rejection)
//! and D6 attempt isolation: abandonment of interrupted attempts,
//! canonical-report preservation, and a failed attempt never
//! replacing a successful `BuildRecord`.

mod support;

use mock::behaviour::{
    CONTINUATION_CLEAR_MARKER, CONTINUATION_MARKER, CONTINUATION_V1, CONTINUATION_V2,
    VERIFY_BLOCKED_MARKER,
};
use mock::session::Session;
use project::build_record::BuildRecord;
use slice::BuildStatus;

/// Author + refine the greeting fixture against `target_adapter`.
async fn ready(target_adapter: &str) -> Session {
    let session = Session::scripted(
        target_adapter,
        vec![mock::answers::greeting_grouping(), mock::answers::greeting_synthesis()],
    );
    support::greeting_ready(&session).await;
    session
}

fn continuation(session: &Session, attempt: u32) -> Option<Vec<u8>> {
    let path = support::attempt_dir(session.root(), "greeting", attempt).join("continuation.bin");
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => panic!("`{}`: {err}", path.display()),
    }
}

mod continuation_rules {
    use super::*;

    /// `build` returns V1 → persisted at `continuation.bin`; `verify`
    /// never mutates; `review` receives the non-empty payload and its
    /// replacement (V2) is what survives the attempt.
    #[tokio::test]
    async fn persisted_and_replaced() {
        let session = ready("mock").await;
        support::marker(session.root(), CONTINUATION_MARKER);

        support::build(&session, "greeting").await.expect("build");

        // Only a non-empty received continuation makes the mock's
        // review return V2 — so V2 on disk proves build's V1 was
        // persisted and echoed forward.
        assert_eq!(continuation(&session, 1), Some(CONTINUATION_V2.to_vec()));
    }

    /// A repair returning `None` preserves the stored payload: after
    /// exhausting verification (review never runs), V1 survives the
    /// three repair dispatches untouched.
    #[tokio::test]
    async fn preserved_across_repairs() {
        let session = ready("mock").await;
        support::marker(session.root(), CONTINUATION_MARKER);
        support::marker(session.root(), VERIFY_BLOCKED_MARKER);

        let err = support::build(&session, "greeting").await.expect_err("budget exhausts");
        assert_eq!(err.variant_str(), "target-build-failed");

        assert_eq!(continuation(&session, 1), Some(CONTINUATION_V1.to_vec()));
    }

    /// `Some([])` clears the stored continuation.
    #[tokio::test]
    async fn cleared_by_empty() {
        let session = ready("mock").await;
        support::marker(session.root(), CONTINUATION_MARKER);
        support::marker(session.root(), CONTINUATION_CLEAR_MARKER);

        support::build(&session, "greeting").await.expect("build");

        assert_eq!(continuation(&session, 1), None, "review's `Some([])` cleared the payload");
    }

    /// A continuation over the 1 MiB cap is rejected before
    /// persistence and terminates the attempt.
    #[tokio::test]
    async fn oversized_rejected() {
        let session = ready("mock-oversized-continuation").await;
        let root = session.root().to_path_buf();

        let err = support::build(&session, "greeting").await.expect_err("cap enforced");
        assert_eq!(err.variant_str(), "target-phase-continuation-oversized");

        assert_eq!(continuation(&session, 1), None, "never persisted");
        let report = support::canonical_report(&root, "greeting");
        assert_eq!(report.status, BuildStatus::Failure);
        assert!(!support::attempt_dir(&root, "greeting", 1).join("stage").exists());
        assert!(!BuildRecord::present(&root.join(".emery/change/slices/greeting")));
    }
}

mod attempts {
    use super::*;

    /// An interrupted attempt (no terminal report) is abandoned, never
    /// resumed: re-entry allocates a fresh ordinal, its stale
    /// continuation is never loaded, its contents stay untouched, and
    /// the canonical report tracks only terminal attempts.
    #[tokio::test]
    async fn interrupted_attempt() {
        let session = ready("mock").await;
        let root = session.root().to_path_buf();
        support::build(&session, "greeting").await.expect("first build");
        let canonical_path = root.join(".emery/change/slices/greeting/build/report.yaml");
        let canonical_before = std::fs::read(&canonical_path).expect("canonical after success");

        // Simulate an interrupted attempt: an allocated directory with
        // a stale continuation and no report.yaml.
        let interrupted = support::attempt_dir(&root, "greeting", 2);
        std::fs::create_dir_all(&interrupted).expect("interrupted attempt");
        std::fs::write(interrupted.join("continuation.bin"), b"stale").expect("stale payload");
        assert_eq!(
            std::fs::read(&canonical_path).expect("canonical"),
            canonical_before,
            "an interrupted attempt leaves the previous canonical report unchanged"
        );

        support::build(&session, "greeting").await.expect("re-entry build");

        // Fresh ordinal, never a resume of `0002`.
        let fresh = support::attempt_dir(&root, "greeting", 3);
        assert_eq!(
            support::phase_files(&root, "greeting", 3),
            ["01-build.yaml", "02-verify.yaml", "03-review.yaml"]
        );
        assert_eq!(continuation(&session, 3), None, "the stale continuation is never loaded");

        // The abandoned attempt is untouched evidence.
        let mut leftovers: Vec<String> = std::fs::read_dir(&interrupted)
            .expect("interrupted dir")
            .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        leftovers.sort();
        assert_eq!(leftovers, ["continuation.bin"]);
        assert_eq!(std::fs::read(interrupted.join("continuation.bin")).expect("stale"), b"stale");

        // Canonical projection tracks the latest terminal attempt.
        let canonical = support::canonical_report(&root, "greeting");
        assert_eq!(canonical.status, BuildStatus::Success);
        let terminal = std::fs::read_to_string(fresh.join("report.yaml")).expect("terminal");
        let terminal: slice::BuildReport =
            serde_saphyr::from_str(&terminal).expect("terminal parses");
        assert_eq!(canonical, terminal);
    }

    /// A failed attempt writes the failed canonical projection but
    /// never writes or replaces the earlier successful `BuildRecord`.
    #[tokio::test]
    async fn failed_attempt_keeps() {
        let session = ready("mock").await;
        let root = session.root().to_path_buf();
        let dir = root.join(".emery/change/slices/greeting");
        support::build(&session, "greeting").await.expect("first build");

        assert!(BuildRecord::present(&dir));
        let records_before = record_files(&dir);
        assert_eq!(records_before.len(), 1, "one content-addressed record: {records_before:?}");

        support::marker(&root, VERIFY_BLOCKED_MARKER);
        let err = support::build(&session, "greeting").await.expect_err("forced failure");
        assert_eq!(err.variant_str(), "target-build-failed");

        assert!(BuildRecord::present(&dir), "the successful record remains");
        assert_eq!(record_files(&dir), records_before, "no record written or replaced");

        // D6: the canonical projection is the latest terminal attempt
        // (failed), while both attempt records persist as evidence.
        let canonical = support::canonical_report(&root, "greeting");
        assert_eq!(canonical.status, BuildStatus::Failure);
        for attempt in [1, 2] {
            assert!(
                support::attempt_dir(&root, "greeting", attempt).join("report.yaml").is_file(),
                "attempt {attempt} terminal report retained"
            );
            assert!(
                !support::attempt_dir(&root, "greeting", attempt).join("stage").exists(),
                "attempt {attempt} stage discarded"
            );
        }
    }

    /// Sorted `builds/<digest>.yaml` file names.
    fn record_files(slice_dir: &std::path::Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(slice_dir.join("builds"))
            .expect("builds dir")
            .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}
