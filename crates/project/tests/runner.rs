//! Non-golden integration tests for `validate_slice` — synthetic
//! scenarios that don't make sense to pin as static JSON.

use std::fs;
use std::path::PathBuf;

use artifacts::validate::validate_slice;
use schema::diagnostics::DiagnosticKind;
use tempfile::TempDir;

mod common;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Stage an empty project dir.
fn stage_project() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().unwrap();
    let project_dir = tempdir.path().to_path_buf();
    (tempdir, project_dir)
}

mod validate {
    use super::*;

    #[test]
    fn missing_artifact_fails_synth() {
        let (_guard, project_dir) = stage_project();
        let slice_dir = project_dir.join(".specify/slices/synth-missing");
        fs::create_dir_all(&slice_dir).unwrap();
        // Deliberately leave out every canonical artifact.

        let findings = validate_slice(&slice_dir).expect("validate_slice ok");

        // Every literal canonical artifact should have synthesised exactly
        // one `<brief>.artifact-exists` violation. `specs` is glob-expanded;
        // an empty slice has no `specs/**/*.md` matches and is silently
        // skipped — the operator-facing failure there comes from the
        // cross-validation rules instead.
        for brief in &["proposal", "design", "tasks"] {
            let rule_id = format!("{brief}.artifact-exists");
            let matches: Vec<_> = findings
                .iter()
                .filter(|d| d.rule_id.as_deref() == Some(rule_id.as_str()))
                .collect();
            assert_eq!(
                matches.len(),
                1,
                "{brief} should have exactly one artifact-exists violation"
            );
            let first = matches[0];
            assert_eq!(
                first.kind,
                DiagnosticKind::Violation,
                "expected violation for `{brief}`: {first:?}"
            );
            assert!(
                first.impact.contains("not found"),
                "unexpected impact for `{brief}`: {}",
                first.impact
            );
        }

        // `contracts` and `specs` are globs — empty expansion is silently
        // skipped per workflow §"Refinement" (slices need not populate every
        // overlay; the cross-validation rules surface the operator-facing
        // failure for the missing slice spec separately).
        assert!(
            !findings.iter().any(|d| d.rule_id.as_deref() == Some("contracts.artifact-exists"))
        );
        assert!(!findings.iter().any(|d| d.rule_id.as_deref() == Some("specs.artifact-exists")));

        // A literal-artifact slice with no populated overlays must surface
        // at least one blocking violation.
        assert!(findings.iter().any(|d| d.kind == DiagnosticKind::Violation));
    }

    #[test]
    fn good_fixture_only_reviews() {
        // Reuses the good fixture to exercise the Semantic-rules-never-called
        // invariant in situ: if any Semantic rule's `check` were invoked the
        // runner would panic (by construction) and this test would fail.
        let fixture = crate_root().join("tests/fixtures/change-good");
        let (_guard, project_dir) = stage_project();
        let slice_dir = project_dir.join(".specify/slices/change-good");
        common::copy_dir(&fixture, &slice_dir);

        let findings = validate_slice(&slice_dir).expect("validate_slice ok");

        // The good fixture has no structural breaches: every diagnostic
        // must be a non-blocking `review` (the deferred semantic rules).
        assert!(
            findings.iter().all(|d| d.kind == DiagnosticKind::Review),
            "good fixture must surface only review-kind diagnostics: {findings:?}"
        );

        // Confirm every Semantic rule surfaced as a deferred review.
        let deferred_count = findings.iter().filter(|d| d.kind == DiagnosticKind::Review).count();
        assert!(deferred_count >= 2, "expected at least two deferred rules");
    }
}
