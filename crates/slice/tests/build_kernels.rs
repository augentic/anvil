//! Integration coverage for the RFC-90 build kernels:
//! `slice::build::{canonical, brief, gate, attempt, stage}`.

use diagnostics::{
    Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, FindingLocation, Severity, is_blocking,
};
use project::Platform;
use project::seam::wire::{
    BUILD_VERSION, BuildOutput, BuildReport, BuildStatus, PhaseOutcome, PhaseReport, PhaseRoot,
    PhaseSource, PhaseWrite, UiSurface,
};
use slice::build::canonical::{Stamp, canonicalize};
use slice::build::gate::PhaseOperation;
use slice::build::{attempt, brief, gate, stage};

fn finding(
    rule: &str, severity: Severity, source: DiagnosticSource, location: Option<FindingLocation>,
) -> Diagnostic {
    Diagnostic::finding(
        rule,
        format!("{rule} breached"),
        format!("{rule} detail"),
        severity,
        DiagnosticKind::Violation,
        source,
        Artifact::Code,
        location,
    )
}

fn located(path: &str, line: Option<u32>, column: Option<u32>) -> FindingLocation {
    FindingLocation {
        path: path.to_string(),
        line,
        column,
        end_line: None,
        end_column: None,
    }
}

const fn stamp() -> Stamp<'static> {
    Stamp {
        target_adapter: "omnia",
        slice: "checkout",
        change: Some("payments"),
    }
}

const fn report(source: PhaseSource, findings: Vec<Diagnostic>) -> PhaseReport {
    PhaseReport {
        outcome: PhaseOutcome::Completed,
        source,
        findings,
        outputs: Vec::new(),
        ui_surface: None,
        written: Vec::new(),
        next_continuation: None,
    }
}

fn code(err: &error::Error) -> String {
    err.variant_str().into_owned()
}

mod canonicalizer {
    use super::*;

    /// Identity is stamped onto every finding and the fingerprint is
    /// always recomputed — a mismatched supplied value is replaced.
    #[test]
    fn stamps_and_recomputes() {
        let mut tampered =
            finding("rule-a", Severity::Important, DiagnosticSource::ModelAssisted, None);
        tampered.fingerprint = "sha256:not-a-real-fingerprint".to_string();

        let out = canonicalize(vec![tampered], &stamp());

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].target_adapter.as_deref(), Some("omnia"));
        assert_eq!(out[0].slice.as_deref(), Some("checkout"));
        assert_eq!(out[0].change.as_deref(), Some("payments"));
        assert_eq!(out[0].fingerprint, diagnostics::fingerprint(&out[0]));
    }

    /// One representative per fingerprint; the strongest severity
    /// survives (`Critical` strongest).
    #[test]
    fn dedupe_keeps_strongest() {
        let weak = finding("rule-a", Severity::Important, DiagnosticSource::ModelAssisted, None);
        let strong = finding("rule-a", Severity::Critical, DiagnosticSource::ModelAssisted, None);

        let out = canonicalize(vec![weak, strong], &stamp());

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].severity, Severity::Critical);
    }

    /// A blocking finding is never displaced by a non-blocking twin
    /// in the same fingerprint group — a `critical`/`review` must not
    /// beat an `important`/`violation` and flip the report passing.
    #[test]
    fn dedupe_prefers_blocking() {
        let blocking =
            finding("rule-a", Severity::Important, DiagnosticSource::ModelAssisted, None);
        let mut review =
            finding("rule-a", Severity::Critical, DiagnosticSource::ModelAssisted, None);
        review.kind = DiagnosticKind::Review;

        let forward = canonicalize(vec![blocking.clone(), review.clone()], &stamp());
        let backward = canonicalize(vec![review, blocking], &stamp());

        for out in [forward, backward] {
            assert_eq!(out.len(), 1);
            assert!(is_blocking(&out[0]), "{:?}", out[0]);
            assert_eq!(out[0].severity, Severity::Important);
            assert_eq!(out[0].kind, DiagnosticKind::Violation);
        }
    }

    /// A same-severity tie breaks on the lexicographically least
    /// id-less JSON of the stamped finding, independent of input order.
    #[test]
    fn tie_break_is_order_independent() {
        let first = finding("rule-a", Severity::Important, DiagnosticSource::ModelAssisted, None);
        let mut second = first.clone();
        second.title = "zzz later title".to_string();

        let forward = canonicalize(vec![first.clone(), second.clone()], &stamp());
        let backward = canonicalize(vec![second, first], &stamp());

        assert_eq!(forward, backward);
        assert_eq!(forward.len(), 1);
        assert_eq!(forward[0].title, "rule-a breached");
    }

    /// Representatives sort by `(severity, location presence, path,
    /// line, column, fingerprint)`: located before unlocated, missing
    /// line/column after concrete values.
    #[test]
    fn canonical_ordering() {
        let critical_unlocated =
            finding("crit", Severity::Critical, DiagnosticSource::ModelAssisted, None);
        let a_line_one = finding(
            "imp-a1",
            Severity::Important,
            DiagnosticSource::ModelAssisted,
            Some(located("a.rs", Some(1), Some(1))),
        );
        let a_no_line = finding(
            "imp-a-none",
            Severity::Important,
            DiagnosticSource::ModelAssisted,
            Some(located("a.rs", None, None)),
        );
        let b_line_one = finding(
            "imp-b",
            Severity::Important,
            DiagnosticSource::ModelAssisted,
            Some(located("b.rs", Some(1), Some(1))),
        );
        let unlocated =
            finding("imp-none", Severity::Important, DiagnosticSource::ModelAssisted, None);

        let out = canonicalize(
            vec![unlocated, b_line_one, a_no_line, a_line_one, critical_unlocated],
            &stamp(),
        );

        let rules: Vec<&str> = out.iter().filter_map(|f| f.rule_id.as_deref()).collect();
        assert_eq!(rules, vec!["crit", "imp-a1", "imp-a-none", "imp-b", "imp-none"]);
        let ids: Vec<&str> = out.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, vec!["DIAG-0001", "DIAG-0002", "DIAG-0003", "DIAG-0004", "DIAG-0005"]);
    }

    /// Byte-stability: every permutation of the same multiset yields
    /// the identical output sequence.
    #[test]
    fn deterministic_across_permutations() {
        let findings = vec![
            finding("r1", Severity::Critical, DiagnosticSource::ModelAssisted, None),
            finding(
                "r2",
                Severity::Important,
                DiagnosticSource::ModelAssisted,
                Some(located("z.rs", Some(9), None)),
            ),
            finding("r2", Severity::Suggestion, DiagnosticSource::ModelAssisted, None),
            finding("r3", Severity::Optional, DiagnosticSource::ModelAssisted, None),
        ];
        let mut reversed = findings.clone();
        reversed.reverse();

        let forward = canonicalize(findings, &stamp());
        let backward = canonicalize(reversed, &stamp());

        assert_eq!(forward, backward);
        assert_eq!(
            serde_json::to_string(&forward).expect("serialize"),
            serde_json::to_string(&backward).expect("serialize"),
        );
    }
}

mod repair_brief {
    use super::*;

    /// Only blocking findings survive, in their canonical order, with
    /// ids and fingerprints untouched.
    #[test]
    fn blocking_only_in_order() {
        let mut review_request =
            finding("review", Severity::Critical, DiagnosticSource::ModelAssisted, None);
        review_request.kind = DiagnosticKind::Review;
        let canonical = canonicalize(
            vec![
                finding("block-1", Severity::Critical, DiagnosticSource::ModelAssisted, None),
                finding("soft", Severity::Suggestion, DiagnosticSource::ModelAssisted, None),
                review_request,
                finding("block-2", Severity::Important, DiagnosticSource::ModelAssisted, None),
            ],
            &stamp(),
        );

        let briefed = brief::repair_brief(&canonical);

        assert!(briefed.iter().all(is_blocking));
        let rules: Vec<&str> = briefed.iter().filter_map(|f| f.rule_id.as_deref()).collect();
        assert_eq!(rules, vec!["block-1", "block-2"]);
        // Ids stay canonical — never renumbered by the brief.
        for briefed_finding in &briefed {
            assert!(canonical.contains(briefed_finding));
        }
    }

    /// The brief truncates to the first [`brief::REPAIR_BRIEF_LIMIT`]
    /// blocking findings; the rest remain gate authority elsewhere.
    #[test]
    fn truncates_to_limit() {
        assert_eq!(brief::REPAIR_BRIEF_LIMIT, 16);
        let findings: Vec<Diagnostic> = (0..20)
            .map(|index| {
                finding(
                    &format!("rule-{index:02}"),
                    Severity::Important,
                    DiagnosticSource::ModelAssisted,
                    None,
                )
            })
            .collect();
        let canonical = canonicalize(findings, &stamp());
        assert_eq!(canonical.len(), 20);

        let briefed = brief::repair_brief(&canonical);

        assert_eq!(briefed.len(), 16);
        assert_eq!(briefed.as_slice(), &canonical[..16]);
    }
}

mod phase_gate {
    use super::*;

    #[test]
    fn accepts_valid_build() {
        let mut valid = report(
            PhaseSource::ModelAssisted,
            vec![finding("r", Severity::Suggestion, DiagnosticSource::ModelAssisted, None)],
        );
        valid.outputs = vec![BuildOutput {
            platform: Platform::Core,
            path: "crates/demo".to_string(),
        }];
        valid.ui_surface = Some(UiSurface { screens: 0 });
        valid.written = vec![PhaseWrite {
            root: PhaseRoot::Workspace,
            path: "crates/demo/src/lib.rs".to_string(),
        }];
        valid.next_continuation = Some(vec![1, 2, 3]);

        gate::accept(PhaseOperation::Build, &valid).unwrap();
    }

    /// A clean deterministic pass is a valid report even with no
    /// findings.
    #[test]
    fn accepts_clean_deterministic_verify() {
        let clean = report(PhaseSource::Deterministic, Vec::new());
        gate::accept(PhaseOperation::Verify, &clean).unwrap();
    }

    #[test]
    fn rejects_source_tool() {
        let err = gate::accept(PhaseOperation::Verify, &report(PhaseSource::Tool, Vec::new()))
            .expect_err("tool source rejected");
        assert_eq!(code(&err), "target-phase-source-tool");
    }

    mod coherence {
        use super::*;

        fn expect_incoherent(operation: PhaseOperation, phase_report: &PhaseReport) {
            let err = gate::accept(operation, phase_report).expect_err("incoherent rejected");
            assert_eq!(code(&err), "target-phase-source-incoherent");
        }

        #[test]
        fn deterministic_with_model_finding() {
            expect_incoherent(
                PhaseOperation::Verify,
                &report(
                    PhaseSource::Deterministic,
                    vec![finding("r", Severity::Important, DiagnosticSource::ModelAssisted, None)],
                ),
            );
        }

        #[test]
        fn model_with_deterministic_finding() {
            expect_incoherent(
                PhaseOperation::Verify,
                &report(
                    PhaseSource::ModelAssisted,
                    vec![finding("r", Severity::Important, DiagnosticSource::Deterministic, None)],
                ),
            );
        }

        /// A report mixing deterministic and model-assisted findings
        /// must claim `hybrid`.
        #[test]
        fn mixed_findings_require_hybrid() {
            let mixed = vec![
                finding("det", Severity::Important, DiagnosticSource::Deterministic, None),
                finding("model", Severity::Important, DiagnosticSource::ModelAssisted, None),
            ];
            expect_incoherent(
                PhaseOperation::Verify,
                &report(PhaseSource::ModelAssisted, mixed.clone()),
            );
            gate::accept(PhaseOperation::Verify, &report(PhaseSource::Hybrid, mixed)).unwrap();
        }

        /// A hybrid claim holds even when `findings` is empty — the
        /// source states how the pass was produced, not that it
        /// found anything (RFC-90 D2).
        #[test]
        fn hybrid_accepts_empty_findings() {
            gate::accept(PhaseOperation::Verify, &report(PhaseSource::Hybrid, Vec::new())).unwrap();
        }

        #[test]
        fn finding_source_tool_rejected() {
            expect_incoherent(
                PhaseOperation::Verify,
                &report(
                    PhaseSource::Hybrid,
                    vec![finding("r", Severity::Important, DiagnosticSource::Tool, None)],
                ),
            );
        }

        #[test]
        fn finding_source_human_rejected() {
            expect_incoherent(
                PhaseOperation::Verify,
                &report(
                    PhaseSource::Hybrid,
                    vec![finding("r", Severity::Important, DiagnosticSource::Human, None)],
                ),
            );
        }
    }

    #[test]
    fn rejects_non_build_output_declaration() {
        let mut with_outputs = report(PhaseSource::ModelAssisted, Vec::new());
        with_outputs.outputs = vec![BuildOutput {
            platform: Platform::Core,
            path: "crates/demo".to_string(),
        }];
        let err = gate::accept(PhaseOperation::Verify, &with_outputs)
            .expect_err("verify outputs rejected");
        assert_eq!(code(&err), "target-phase-output-declaration");

        let mut with_surface = report(PhaseSource::ModelAssisted, Vec::new());
        with_surface.ui_surface = Some(UiSurface { screens: 2 });
        let err = gate::accept(PhaseOperation::Repair, &with_surface)
            .expect_err("repair ui-surface rejected");
        assert_eq!(code(&err), "target-phase-output-declaration");
    }

    #[test]
    fn rejects_dirty_not_applicable() {
        let mut blocking = report(
            PhaseSource::ModelAssisted,
            vec![finding("r", Severity::Critical, DiagnosticSource::ModelAssisted, None)],
        );
        blocking.outcome = PhaseOutcome::NotApplicable;
        let err = gate::accept(PhaseOperation::Review, &blocking)
            .expect_err("blocking not-applicable rejected");
        assert_eq!(code(&err), "target-phase-not-applicable-dirty");

        let mut written = report(PhaseSource::ModelAssisted, Vec::new());
        written.outcome = PhaseOutcome::NotApplicable;
        written.written = vec![PhaseWrite {
            root: PhaseRoot::Artifacts,
            path: "tasks.md".to_string(),
        }];
        let err = gate::accept(PhaseOperation::Review, &written)
            .expect_err("written not-applicable rejected");
        assert_eq!(code(&err), "target-phase-not-applicable-dirty");

        let mut clean = report(PhaseSource::ModelAssisted, Vec::new());
        clean.outcome = PhaseOutcome::NotApplicable;
        gate::accept(PhaseOperation::Review, &clean).unwrap();
    }

    #[test]
    fn rejects_write_escapes() {
        for path in ["", "/abs/path", "a\\b", "C:evil", "a/../b"] {
            let mut escaping = report(PhaseSource::ModelAssisted, Vec::new());
            escaping.written = vec![PhaseWrite {
                root: PhaseRoot::Workspace,
                path: path.to_string(),
            }];
            let err = gate::accept(PhaseOperation::Build, &escaping)
                .expect_err("escaping write rejected");
            assert_eq!(code(&err), "target-phase-write-escape", "path: {path:?}");
        }
    }

    #[test]
    fn rejects_location_escapes() {
        let escaping = report(
            PhaseSource::ModelAssisted,
            vec![finding(
                "r",
                Severity::Important,
                DiagnosticSource::ModelAssisted,
                Some(located("../outside.rs", Some(1), None)),
            )],
        );
        let err = gate::accept(PhaseOperation::Verify, &escaping)
            .expect_err("escaping location rejected");
        assert_eq!(code(&err), "target-phase-location-escape");
    }

    #[test]
    fn rejects_verify_continuation() {
        let mut mutating = report(PhaseSource::ModelAssisted, Vec::new());
        mutating.next_continuation = Some(vec![1]);
        let err = gate::accept(PhaseOperation::Verify, &mutating)
            .expect_err("verify continuation rejected");
        assert_eq!(code(&err), "target-phase-verify-continuation");
    }

    #[test]
    fn rejects_oversized_continuation() {
        let mut oversized = report(PhaseSource::ModelAssisted, Vec::new());
        oversized.next_continuation = Some(vec![0; gate::CONTINUATION_LIMIT + 1]);
        let err = gate::accept(PhaseOperation::Build, &oversized)
            .expect_err("oversized continuation rejected");
        assert_eq!(code(&err), "target-phase-continuation-oversized");

        let mut at_limit = report(PhaseSource::ModelAssisted, Vec::new());
        at_limit.next_continuation = Some(vec![0; gate::CONTINUATION_LIMIT]);
        gate::accept(PhaseOperation::Build, &at_limit).unwrap();
    }

    #[test]
    fn engine_finding_blocks() {
        let engine = gate::engine_finding(
            "target-phase-source-tool",
            "phase gate rejected the report",
            "the verify report claimed source tool",
        );
        assert!(is_blocking(&engine));
        assert_eq!(engine.source, DiagnosticSource::Deterministic);
        assert_eq!(engine.severity, Severity::Important);
        assert_eq!(engine.artifact, Artifact::Code);
        assert_eq!(engine.fingerprint, diagnostics::fingerprint(&engine));
    }
}

mod attempt_store {
    use super::*;

    fn terminal_report() -> BuildReport {
        BuildReport {
            version: BUILD_VERSION,
            slice: "checkout".to_string(),
            target: "omnia".to_string(),
            status: BuildStatus::Failure,
            findings: Vec::new(),
            outputs: Vec::new(),
            ui_surface: None,
        }
    }

    /// Allocation is monotonic and never resumes: an unterminated
    /// attempt keeps its ordinal, and the next allocation moves on.
    #[test]
    fn allocation_monotonic() {
        let slice_dir = tempfile::tempdir().expect("tempdir");
        let first = attempt::allocate(slice_dir.path()).expect("first attempt");
        assert_eq!(first.id, 1);
        assert!(first.dir.ends_with("build/attempts/0001"));
        assert!(!first.dir.join("report.yaml").is_file());

        let second = attempt::allocate(slice_dir.path()).expect("second attempt");
        assert_eq!(second.id, 2);
        assert!(second.dir.ends_with("build/attempts/0002"));
    }

    #[test]
    fn copy_request_preserves_bytes() {
        let slice_dir = tempfile::tempdir().expect("tempdir");
        let build_dir = slice_dir.path().join("build");
        std::fs::create_dir_all(&build_dir).expect("build dir");
        std::fs::write(build_dir.join("request.yaml"), b"version: 1\n").expect("request");

        let allocated = attempt::allocate(slice_dir.path()).expect("attempt");
        attempt::copy_request(&allocated, slice_dir.path()).expect("copy");

        let copied = std::fs::read(allocated.dir.join("request.yaml")).expect("copied");
        assert_eq!(copied, b"version: 1\n");
    }

    /// The phase record digest is `sha256:` over the exact written
    /// bytes and stable across identical reports.
    #[test]
    fn phase_write_digest_stable() {
        let slice_dir = tempfile::tempdir().expect("tempdir");
        let allocated = attempt::allocate(slice_dir.path()).expect("attempt");
        let phase_report = report(PhaseSource::ModelAssisted, Vec::new());

        let first = attempt::write_phase(&allocated, 1, PhaseOperation::Verify, &phase_report)
            .expect("first write");
        let second = attempt::write_phase(&allocated, 2, PhaseOperation::Verify, &phase_report)
            .expect("second write");

        assert!(first.path.ends_with("phases/01-verify.yaml"));
        assert!(second.path.ends_with("phases/02-verify.yaml"));
        assert_eq!(first.digest, second.digest);

        let bytes = std::fs::read(&first.path).expect("written bytes");
        assert_eq!(first.digest, format!("sha256:{}", diagnostics::digest::sha256_hex(&bytes)));
    }

    /// The persisted phase YAML never carries the continuation — it is
    /// `#[serde(skip)]` on the wire type.
    #[test]
    fn phase_write_omits_continuation() {
        let slice_dir = tempfile::tempdir().expect("tempdir");
        let allocated = attempt::allocate(slice_dir.path()).expect("attempt");
        let mut phase_report = report(PhaseSource::ModelAssisted, Vec::new());
        phase_report.next_continuation = Some(b"opaque".to_vec());

        let record = attempt::write_phase(&allocated, 1, PhaseOperation::Build, &phase_report)
            .expect("write");

        let yaml = std::fs::read_to_string(&record.path).expect("yaml");
        assert!(!yaml.contains("continuation"), "{yaml}");
    }

    #[test]
    fn continuation_roundtrip() {
        let slice_dir = tempfile::tempdir().expect("tempdir");
        let allocated = attempt::allocate(slice_dir.path()).expect("attempt");

        assert_eq!(attempt::load_continuation(&allocated).expect("empty load"), None);
        attempt::store_continuation(&allocated, b"session-state").expect("store");
        assert_eq!(
            attempt::load_continuation(&allocated).expect("load"),
            Some(b"session-state".to_vec())
        );
        attempt::clear_continuation(&allocated).expect("clear");
        assert_eq!(attempt::load_continuation(&allocated).expect("cleared load"), None);
        attempt::clear_continuation(&allocated).expect("clear absent is fine");
    }

    /// An attempt without a terminal `report.yaml` is abandoned
    /// evidence (RFC-90 D6); `write_terminal` supplies it.
    #[test]
    fn terminal_report_terminates() {
        let slice_dir = tempfile::tempdir().expect("tempdir");
        let allocated = attempt::allocate(slice_dir.path()).expect("attempt");
        assert!(!allocated.dir.join("report.yaml").is_file());

        attempt::write_terminal(&allocated, &terminal_report()).expect("terminal");

        assert!(allocated.dir.join("report.yaml").is_file());
    }
}

mod artifact_stage {
    use project::adapter::{WritableArtifactDeclaration, WritableArtifactKind};
    use slice::build::stage::{ChangeKind, StageChange};

    use super::*;

    fn file_grant(path: &str) -> WritableArtifactDeclaration {
        WritableArtifactDeclaration {
            path: path.to_string(),
            kind: WritableArtifactKind::File,
        }
    }

    fn tree_grant(path: &str) -> WritableArtifactDeclaration {
        WritableArtifactDeclaration {
            path: path.to_string(),
            kind: WritableArtifactKind::Tree,
        }
    }

    /// A slice tree with artifacts plus the engine-owned `build/`,
    /// `builds/`, `merge/`, and `decisions/` subtrees; returns the
    /// `(slice_dir, attempt_dir)` temp handles and the seeded stage.
    fn seeded() -> (tempfile::TempDir, tempfile::TempDir, stage::Stage) {
        let slice_dir = tempfile::tempdir().expect("slice dir");
        std::fs::write(slice_dir.path().join("proposal.md"), b"# Proposal\n").expect("proposal");
        std::fs::write(slice_dir.path().join("tasks.md"), b"- [ ] task\n").expect("tasks");
        let specs = slice_dir.path().join("specs/core");
        std::fs::create_dir_all(&specs).expect("specs");
        std::fs::write(specs.join("spec.md"), b"# Spec\n").expect("spec");
        let build_dir = slice_dir.path().join("build");
        std::fs::create_dir_all(&build_dir).expect("build dir");
        std::fs::write(build_dir.join("request.yaml"), b"version: 1\n").expect("request");
        for (dir, file) in
            [("builds", "record.yaml"), ("merge", "preflight.yaml"), ("decisions", "one.md")]
        {
            let engine_dir = slice_dir.path().join(dir);
            std::fs::create_dir_all(&engine_dir).expect("engine dir");
            std::fs::write(engine_dir.join(file), b"engine-owned\n").expect("engine file");
        }

        let attempt_dir = tempfile::tempdir().expect("attempt dir");
        let staged = stage::seed(attempt_dir.path(), slice_dir.path()).expect("seed");
        (slice_dir, attempt_dir, staged)
    }

    /// Seeding mirrors the slice tree minus the engine-owned
    /// subtrees, and an untouched stage diffs empty.
    #[test]
    fn seed_excludes_engine_owned() {
        let (_slice_dir, _attempt_dir, staged) = seeded();

        assert!(staged.root().join("tasks.md").is_file());
        assert!(staged.root().join("specs/core/spec.md").is_file());
        for engine_owned in ["build", "builds", "merge", "decisions"] {
            assert!(!staged.root().join(engine_owned).exists(), "{engine_owned} mirrored");
        }
        assert_eq!(staged.diff().expect("diff"), Vec::new());
    }

    /// A symlinked file inside the stage — even under a granted path
    /// — is rejected before any diff or promotion can follow it.
    #[test]
    fn symlink_in_stage_rejected() {
        let (_slice_dir, _attempt_dir, staged) = seeded();
        std::fs::remove_file(staged.root().join("tasks.md")).expect("clear granted path");
        std::os::unix::fs::symlink("/etc/passwd", staged.root().join("tasks.md")).expect("symlink");

        let err = staged.diff().expect_err("symlink rejected");
        assert_eq!(code(&err), "target-build-artifact-symlink");
    }

    /// The diff classifies adds, modifications, and deletions, sorted
    /// by path.
    #[test]
    fn diff_classifies_changes() {
        let (_slice_dir, _attempt_dir, staged) = seeded();
        std::fs::write(staged.root().join("tasks.md"), b"- [x] task\n").expect("modify");
        let contracts = staged.root().join("contracts");
        std::fs::create_dir_all(&contracts).expect("contracts dir");
        std::fs::write(contracts.join("api.yaml"), b"openapi: 3.1.0\n").expect("add");
        std::fs::remove_file(staged.root().join("proposal.md")).expect("delete");

        let changes = staged.diff().expect("diff");

        assert_eq!(
            changes,
            vec![
                StageChange {
                    path: "contracts/api.yaml".to_string(),
                    kind: ChangeKind::Added,
                },
                StageChange {
                    path: "proposal.md".to_string(),
                    kind: ChangeKind::Deleted,
                },
                StageChange {
                    path: "tasks.md".to_string(),
                    kind: ChangeKind::Modified,
                },
            ]
        );
    }

    mod grants {
        use super::*;

        fn change(path: &str) -> StageChange {
            StageChange {
                path: path.to_string(),
                kind: ChangeKind::Modified,
            }
        }

        #[test]
        fn file_grant_covers_exact_path_only() {
            let grants = [file_grant("tasks.md")];
            stage::enforce_grants(&[change("tasks.md")], &grants).unwrap();

            let err = stage::enforce_grants(&[change("tasks.md.bak")], &grants)
                .expect_err("uncovered path rejected");
            assert_eq!(code(&err), "target-build-artifact-scope-violation");
        }

        #[test]
        fn tree_grant_covers_descendants() {
            let grants = [tree_grant("contracts")];
            stage::enforce_grants(&[change("contracts")], &grants).unwrap();
            stage::enforce_grants(&[change("contracts/api.yaml")], &grants).unwrap();
            stage::enforce_grants(&[change("contracts/nested/deep.yaml")], &grants).unwrap();

            let err = stage::enforce_grants(&[change("contracts-other/api.yaml")], &grants)
                .expect_err("sibling prefix rejected");
            assert_eq!(code(&err), "target-build-artifact-scope-violation");
        }

        /// The violation names the offending path even when the phase
        /// omitted it from `written`.
        #[test]
        fn violation_names_path() {
            let err = stage::enforce_grants(&[change("spec.md")], &[file_grant("tasks.md")])
                .expect_err("violation");
            assert!(err.to_string().contains("spec.md"), "{err}");
        }

        #[test]
        fn malformed_grant_rejected() {
            let err = stage::enforce_grants(&[], &[file_grant("../escape.md")])
                .expect_err("malformed grant rejected");
            assert_eq!(code(&err), "target-build-artifact-grant-invalid");
        }
    }

    /// Promotion applies the complete change set — add, modify,
    /// delete — and leaves no temporaries behind.
    #[test]
    fn promote_commits_all() {
        let (slice_dir, _attempt_dir, staged) = seeded();
        std::fs::write(staged.root().join("tasks.md"), b"- [x] task\n").expect("modify");
        let contracts = staged.root().join("contracts");
        std::fs::create_dir_all(&contracts).expect("contracts dir");
        std::fs::write(contracts.join("api.yaml"), b"openapi: 3.1.0\n").expect("add");
        std::fs::remove_file(staged.root().join("proposal.md")).expect("delete");
        let changes = staged.diff().expect("diff");

        staged.promote(&changes, slice_dir.path()).expect("promote");

        assert_eq!(
            std::fs::read(slice_dir.path().join("tasks.md")).expect("tasks"),
            b"- [x] task\n"
        );
        assert_eq!(
            std::fs::read(slice_dir.path().join("contracts/api.yaml")).expect("contract"),
            b"openapi: 3.1.0\n"
        );
        assert!(!slice_dir.path().join("proposal.md").exists());
        assert!(!slice_dir.path().join("tasks.md.promote.tmp").exists());
        assert!(!slice_dir.path().join("contracts/api.yaml.promote.tmp").exists());
    }

    /// A bogus change path fails validation before any mutation.
    #[test]
    fn promote_validates_before_mutating() {
        let (slice_dir, _attempt_dir, staged) = seeded();
        let bogus = [StageChange {
            path: "missing.txt".to_string(),
            kind: ChangeKind::Added,
        }];

        let err = staged.promote(&bogus, slice_dir.path()).expect_err("bogus path rejected");

        assert_eq!(code(&err), "target-build-artifact-promotion-failed");
        assert!(!slice_dir.path().join("missing.txt").exists());
        assert_eq!(
            std::fs::read(slice_dir.path().join("tasks.md")).expect("tasks"),
            b"- [ ] task\n"
        );
    }

    /// A mid-commit failure rolls the already-committed subset back:
    /// the modified file is restored to its original bytes and no
    /// temporaries survive.
    #[test]
    fn promote_rolls_back_on_commit_failure() {
        let (slice_dir, _attempt_dir, staged) = seeded();
        std::fs::write(staged.root().join("tasks.md"), b"- [x] task\n").expect("modify");
        std::fs::remove_file(staged.root().join("proposal.md")).expect("stage delete");
        // Replace the deletion's destination with a directory so the
        // commit-time `remove_file` fails after the modify committed.
        std::fs::remove_file(slice_dir.path().join("proposal.md")).expect("clear dest");
        std::fs::create_dir(slice_dir.path().join("proposal.md")).expect("block dest");
        let changes = [
            StageChange {
                path: "tasks.md".to_string(),
                kind: ChangeKind::Modified,
            },
            StageChange {
                path: "proposal.md".to_string(),
                kind: ChangeKind::Deleted,
            },
        ];

        let err =
            staged.promote(&changes, slice_dir.path()).expect_err("blocked delete fails commit");

        assert_eq!(code(&err), "target-build-artifact-promotion-failed");
        assert_eq!(
            std::fs::read(slice_dir.path().join("tasks.md")).expect("tasks restored"),
            b"- [ ] task\n"
        );
        assert!(!slice_dir.path().join("tasks.md.promote.tmp").exists());
    }

    #[test]
    fn discard_removes_stage() {
        let (_slice_dir, attempt_dir, staged) = seeded();
        assert!(staged.root().is_dir());

        stage::discard(attempt_dir.path());
        assert!(!staged.root().exists());

        // Discarding an absent stage is a silent no-op.
        stage::discard(attempt_dir.path());
    }
}
