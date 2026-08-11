//! The mock adapter's deterministic, model-free behaviour core:
//! behaviour keys off the routed adapter id (`docs` / `code` /
//! `fail-*` substrings, else `greeting`) and `mock-*` marker files.

pub use source::{extract, survey};
pub use targets::{
    BUILD_DIR, CONTINUATION_CLEAR_MARKER, CONTINUATION_MARKER, CONTINUATION_V1, CONTINUATION_V2,
    FAIL, FAIL_BUILD_MARKER, FAIL_MERGE, REVIEW_BLOCKED_MARKER, REVIEW_FIXABLE_MARKER,
    REVIEW_REPAIRED, VERIFICATION_REPAIRED, VERIFY_AFTER_REVIEW, VERIFY_BLOCKED_MARKER,
    VERIFY_FIXABLE_MARKER, build, build_artifact_path, guidance, merge, repair, review, verify,
};

mod source {
    use adapter::seam::{Authority, Backing, Claim, ClaimKind, Error, Evidence, Lead};

    /// Survey the source selected by `id` into its controlled lead set.
    ///
    /// # Errors
    ///
    /// `Internal` when the id selects the `fail-survey` profile.
    pub fn survey(id: &str) -> Result<Vec<Lead>, Error> {
        if id.contains("fail-survey") {
            return Err(Error::Internal(format!("mock survey failure for `{id}`")));
        }
        Ok(match profile(id) {
            Profile::Docs => vec![
                lead("login-flow", "Users sign in with an email address and password.", &["auth"]),
                lead("session-timeout", "Documented session expiry policy.", &["auth"]),
                lead(
                    "password-reset",
                    "A password reset flow is mentioned but never specified.",
                    &["auth"],
                ),
            ],
            Profile::Code => vec![
                lead(
                    "login-flow",
                    "signIn(email, password) handler in the auth module.",
                    &["auth"],
                ),
                lead("session-timeout", "Session TTL constant in the session store.", &["auth"]),
            ],
            Profile::Minimal => {
                vec![lead(
                    "greeting",
                    "The greeting endpoint returns a static string.",
                    &["greeting"],
                )]
            }
        })
    }

    /// Extract the controlled Evidence for one surveyed lead.
    ///
    /// # Errors
    ///
    /// - `Internal` when the id selects the `fail-extract` profile.
    /// - `InvalidRequest` when `lead` is not one this source surveys.
    pub fn extract(id: &str, lead: &Lead) -> Result<Evidence, Error> {
        if id.contains("fail-extract") {
            return Err(Error::Internal(format!("mock extract failure for `{id}`")));
        }
        let evidence = match (profile(id), lead.lead.as_str()) {
            (Profile::Docs, "login-flow") => Evidence {
                authority: Authority::Documentation,
                claims: vec![
                    requirement(
                        "login.flow",
                        "Documented login flow",
                        "Users sign in with an email address and password.",
                    ),
                    criterion(
                        "login.flow.lockout",
                        "Five failed attempts lock the account for fifteen minutes.",
                    ),
                ],
            },
            (Profile::Docs, "session-timeout") => Evidence {
                authority: Authority::Documentation,
                claims: vec![requirement(
                    "session.timeout",
                    "Documented session expiry",
                    "Sessions expire after 30 minutes of inactivity.",
                )],
            },
            // The deliberate evidence gap: the lead exists, but its only
            // claim is an anchorless mention with no behavioural detail,
            // so a faithful synthesis marks the requirement `[unknown]`.
            (Profile::Docs, "password-reset") => Evidence {
                authority: Authority::Documentation,
                claims: vec![Claim {
                    kind: ClaimKind::Section,
                    id: Some("password-reset.mention".to_string()),
                    path: None,
                    synopsis: Some("Password reset exists".to_string()),
                    backing: Some(Backing::Payload(
                        "A password reset flow is mentioned with no defined behaviour.".to_string(),
                    )),
                }],
            },
            (Profile::Code, "login-flow") => Evidence {
                authority: Authority::Behaviour,
                claims: vec![requirement(
                    "login.flow",
                    "Observed login handler",
                    "signIn validates credentials and issues a session token.",
                )],
            },
            // The authority disagreement: behaviour observes 15 minutes
            // where documentation states 30 — docs outrank behaviour, so
            // resolution is a `[divergence]` with the docs source winning.
            (Profile::Code, "session-timeout") => Evidence {
                authority: Authority::Behaviour,
                claims: vec![requirement(
                    "session.timeout",
                    "Observed session TTL",
                    "SESSION_TTL expires sessions after 15 minutes of inactivity.",
                )],
            },
            (Profile::Minimal, "greeting") => Evidence {
                authority: Authority::Documentation,
                claims: vec![requirement(
                    "greeting.behaviour",
                    "Greeting behaviour",
                    "GET /greeting returns the static string 'hello'.",
                )],
            },
            (_, unknown) => {
                return Err(Error::InvalidRequest(format!(
                    "mock source `{id}` surveys no lead `{unknown}`"
                )));
            }
        };
        Ok(evidence)
    }

    /// The behaviour profile a routed adapter id selects.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Profile {
        /// Documentation half of the adversarial pair.
        Docs,
        /// Behaviour (code) half of the adversarial pair.
        Code,
        /// The single-lead `greeting` profile.
        Minimal,
    }

    fn profile(id: &str) -> Profile {
        if id.contains("docs") {
            Profile::Docs
        } else if id.contains("code") {
            Profile::Code
        } else {
            Profile::Minimal
        }
    }

    fn lead(id: &str, synopsis: &str, topics: &[&str]) -> Lead {
        Lead {
            lead: id.to_string(),
            synopsis: synopsis.to_string(),
            topics: topics.iter().map(ToString::to_string).collect(),
        }
    }

    fn requirement(id: &str, synopsis: &str, statement: &str) -> Claim {
        Claim {
            kind: ClaimKind::Requirement,
            id: Some(id.to_string()),
            path: None,
            synopsis: Some(synopsis.to_string()),
            backing: Some(Backing::Payload(statement.to_string())),
        }
    }

    fn criterion(id: &str, body: &str) -> Claim {
        Claim {
            kind: ClaimKind::Criterion,
            id: Some(id.to_string()),
            path: None,
            synopsis: None,
            backing: Some(Backing::Payload(body.to_string())),
        }
    }
}

mod targets {
    use std::path::{Path, PathBuf};

    use adapter::seam::{
        BuildOutput, DiagnosticSource, Error, FindingArtifact, FindingConfidence, FindingEvidence,
        FindingKind, Input, MergePhase, PhaseFinding, PhaseOutcome, PhaseReport, PhaseRoot,
        PhaseSource, PhaseWrite, Platform, RepairOrigin, Report, Severity, Status, Workspace,
    };

    /// Marker file (project-root-relative) that flips builds to a
    /// blocking-finding report while it exists.
    pub const FAIL_BUILD_MARKER: &str = "mock-fail-build";

    /// Directory (project-root-relative) mock builds write their
    /// observable output into.
    pub const BUILD_DIR: &str = "mock-build";

    /// Marker file: every `verify` returns one blocking finding —
    /// drives verification-budget exhaustion.
    pub const VERIFY_BLOCKED_MARKER: &str = "mock-verify-blocked";

    /// Marker file: `verify` blocks until a verification-origin
    /// `repair` has written [`VERIFICATION_REPAIRED`] — drives one
    /// verification-repair round.
    pub const VERIFY_FIXABLE_MARKER: &str = "mock-verify-fixable";

    /// Marker file: every `review` returns one blocking finding —
    /// drives review-budget exhaustion.
    pub const REVIEW_BLOCKED_MARKER: &str = "mock-review-blocked";

    /// Marker file: `review` blocks until a review-origin `repair` has
    /// written [`REVIEW_REPAIRED`].
    pub const REVIEW_FIXABLE_MARKER: &str = "mock-review-fixable";

    /// Marker file: `verify` blocks once a review-origin repair ran.
    ///
    /// After [`REVIEW_REPAIRED`] exists, `verify` returns a blocking
    /// finding — drives the post-review-repair verification failure
    /// consuming the shared verification budget. Compose with
    /// [`REVIEW_FIXABLE_MARKER`].
    pub const VERIFY_AFTER_REVIEW: &str = "mock-verify-after-review-fail";

    /// Marker file: `build` returns [`CONTINUATION_V1`]; a `review`
    /// receiving a non-empty continuation replaces it with
    /// [`CONTINUATION_V2`]; `repair` preserves (returns `None`).
    pub const CONTINUATION_MARKER: &str = "mock-continuation";

    /// Marker file: a `review` receiving a non-empty continuation
    /// clears it (returns `Some([])`) instead of replacing — compose
    /// with [`CONTINUATION_MARKER`].
    pub const CONTINUATION_CLEAR_MARKER: &str = "mock-continuation-clear";

    /// Workspace-relative sentinel a verification-origin repair writes.
    pub const VERIFICATION_REPAIRED: &str = "mock-build/verification-repaired";

    /// Workspace-relative round counter kept under
    /// [`VERIFY_BLOCKED_MARKER`]: each verify bumps it and tags its
    /// blocking finding (and each repair its audit finding) with the
    /// round, so round leaks survive dedupe in terminal-report tests.
    pub const VERIFY_ROUND_COUNTER: &str = "mock-build/verify-round";

    /// Workspace-relative sentinel a review-origin repair writes.
    pub const REVIEW_REPAIRED: &str = "mock-build/review-repaired";

    /// The continuation payload `build` returns under
    /// [`CONTINUATION_MARKER`].
    pub const CONTINUATION_V1: &[u8] = b"mock-continuation-v1";

    /// The replacement continuation `review` returns when it received
    /// a non-empty one under [`CONTINUATION_MARKER`].
    pub const CONTINUATION_V2: &[u8] = b"mock-continuation-v2";

    /// The deterministic guidance brief served to synthesis.
    ///
    /// # Errors
    ///
    /// `Internal` when the id selects the `fail-guidance` profile.
    pub fn guidance(id: &str) -> Result<String, Error> {
        if id.contains("fail-guidance") {
            return Err(Error::Internal(format!("mock guidance failure for `{id}`")));
        }
        Ok(format!(
            "Fixture guidance ({id}): keep specs behavioural, one domain per spec; builds write \
         one markdown artifact per slice under `{BUILD_DIR}/`."
        ))
    }

    /// Build one slice into the private workspace.
    ///
    /// Writes the observable artifact under [`BUILD_DIR`], reports it
    /// as a core-platform output, and — for the grant-holding default
    /// `mock` identity — appends a line to the artifact stage's
    /// `tasks.md` when a stage was lent.
    ///
    /// Control markers are test control-plane on the project tree
    /// (same posture as merge-gate markers) — they are not part of the
    /// recorded target-base pin, so they are read from `project_root`.
    ///
    /// # Errors
    ///
    /// - `Internal` when the id selects the `fail-build` profile.
    /// - `Io` when an artifact cannot be written.
    pub fn build(
        workspace: &Workspace, project_root: &Path, id: &str, slice: &str, inputs: &[Input],
    ) -> Result<PhaseReport, Error> {
        if id.contains("fail-build") {
            return Err(Error::Internal(format!("mock build failure for `{id}`")));
        }
        if id.contains("missing-output") {
            // A dishonest completion: the declared output is never
            // written, so the caller's outputs-exist gate must abort
            // the build.
            let mut report = PhaseReport::completed(PhaseSource::Deterministic);
            report.outputs = vec![core_output(format!("{BUILD_DIR}/{slice}-never-written.md"))];
            return Ok(report);
        }
        if project_root.join(FAIL_BUILD_MARKER).is_file() {
            return Ok(blocked("the fail-build marker forces a blocking build finding"));
        }
        let relative = format!("{BUILD_DIR}/{slice}.md");
        write_file(&workspace.root_path().join(&relative), &build_artifact(id, slice, inputs))?;
        let mut written = vec![workspace_write(&relative)];
        if let Some(stage) = &workspace.artifact_stage {
            if id.contains("stage-escape") {
                // The escape: a staged write outside the declared
                // `tasks.md` grant.
                write_file(&stage.root_path().join("undeclared.md"), "escaped the grant\n")?;
                written.push(stage_write("undeclared.md"));
            } else if adapter_name(id) == "mock" {
                // Only the grant-holding default identity writes the
                // stage; `- [x] X.Y` grammar keeps the promoted
                // tasks.md valid under the artifact rules.
                append_line(
                    &stage.root_path().join("tasks.md"),
                    &format!("- [x] 99.1 built {slice}"),
                )?;
                written.push(stage_write("tasks.md"));
            }
        }
        let next_continuation = if id.contains("oversized-continuation") {
            // One byte over the engine's 1 MiB continuation cap.
            Some(vec![0_u8; 1024 * 1024 + 1])
        } else if project_root.join(CONTINUATION_MARKER).is_file() {
            Some(CONTINUATION_V1.to_vec())
        } else {
            None
        };
        let mut report = PhaseReport::completed(PhaseSource::Deterministic);
        report.outputs = vec![core_output(relative)];
        report.written = written;
        report.next_continuation = next_continuation;
        Ok(report)
    }

    /// One verification pass over the lent workspace: a clean
    /// deterministic report unless a marker profile (or an
    /// invalid-report identity) says otherwise.
    ///
    /// # Errors
    ///
    /// Reserved by the trait surface; the mock's verify never fails.
    pub fn verify(
        workspace: &Workspace, project_root: &Path, id: &str,
    ) -> Result<PhaseReport, Error> {
        if id.contains("tool-source") {
            // Gate-invalid: `tool` is reserved on the wire but
            // rejected by the RFC-90 engine gate.
            return Ok(PhaseReport::completed(PhaseSource::Tool));
        }
        if id.contains("verify-outputs") {
            // Gate-invalid: only `build` declares outputs.
            let mut report = PhaseReport::completed(PhaseSource::Deterministic);
            report.outputs = vec![core_output(format!("{BUILD_DIR}/verify-declared.md"))];
            return Ok(report);
        }
        if id.contains("na-blocking") {
            // Gate-invalid: a non-applicable report with a blocking
            // finding.
            let mut report = blocked("a non-applicable verify carrying a blocking finding");
            report.outcome = PhaseOutcome::NotApplicable;
            return Ok(report);
        }
        if id.contains("verify-continuation") {
            // Gate-invalid: verify must not mutate the continuation.
            let mut report = PhaseReport::completed(PhaseSource::Deterministic);
            report.next_continuation = Some(b"mock-verify-continuation".to_vec());
            return Ok(report);
        }
        if project_root.join(VERIFY_BLOCKED_MARKER).is_file() {
            let round = bump_round(workspace)?;
            return Ok(blocked(&format!(
                "the verify-blocked marker fails every verification (round {round})"
            )));
        }
        if project_root.join(VERIFY_FIXABLE_MARKER).is_file()
            && !workspace.root_path().join(VERIFICATION_REPAIRED).is_file()
        {
            return Ok(blocked("the verify-fixable marker blocks until a verification repair ran"));
        }
        if project_root.join(VERIFY_AFTER_REVIEW).is_file()
            && workspace.root_path().join(REVIEW_REPAIRED).is_file()
        {
            return Ok(blocked("the review-origin repair regressed verification"));
        }
        Ok(PhaseReport::completed(PhaseSource::Deterministic))
    }

    /// One findings-directed repair pass: write the origin's repaired
    /// sentinel into the workspace so a fixable verify/review pass
    /// observes it, preserving the continuation (`None`).
    ///
    /// Under the verify-blocked round counter the report carries one
    /// round-tagged non-blocking audit finding.
    ///
    /// # Errors
    ///
    /// `Io` when the sentinel cannot be written.
    pub fn repair(workspace: &Workspace, origin: RepairOrigin) -> Result<PhaseReport, Error> {
        let relative = match origin {
            RepairOrigin::Verification => VERIFICATION_REPAIRED,
            RepairOrigin::Review => REVIEW_REPAIRED,
        };
        write_file(&workspace.root_path().join(relative), "repaired\n")?;
        let mut report = PhaseReport::completed(PhaseSource::Deterministic);
        report.written = vec![workspace_write(relative)];
        if let Some(round) = read_round(workspace) {
            report.findings = vec![repair_finding(&format!("repair pass after round {round}"))];
        }
        Ok(report)
    }

    /// Bump and return the verify-blocked round counter kept in the
    /// workspace at [`VERIFY_ROUND_COUNTER`].
    fn bump_round(workspace: &Workspace) -> Result<u32, Error> {
        let round = read_round(workspace).unwrap_or(0) + 1;
        write_file(&workspace.root_path().join(VERIFY_ROUND_COUNTER), &round.to_string())?;
        Ok(round)
    }

    /// The current verify-blocked round, `None` outside the profile.
    fn read_round(workspace: &Workspace) -> Option<u32> {
        std::fs::read_to_string(workspace.root_path().join(VERIFY_ROUND_COUNTER))
            .ok()
            .and_then(|body| body.trim().parse().ok())
    }

    /// One standards-review pass: a clean deterministic report unless
    /// a marker profile says otherwise.
    ///
    /// Under [`CONTINUATION_MARKER`] a non-empty received continuation
    /// is replaced with [`CONTINUATION_V2`], or cleared under
    /// [`CONTINUATION_CLEAR_MARKER`].
    ///
    /// # Errors
    ///
    /// Reserved by the trait surface; the mock's review never fails.
    pub fn review(
        workspace: &Workspace, project_root: &Path, continuation: Option<&[u8]>,
    ) -> Result<PhaseReport, Error> {
        if project_root.join(REVIEW_BLOCKED_MARKER).is_file() {
            return Ok(blocked("the review-blocked marker fails every review"));
        }
        if project_root.join(REVIEW_FIXABLE_MARKER).is_file()
            && !workspace.root_path().join(REVIEW_REPAIRED).is_file()
        {
            return Ok(blocked("the review-fixable marker blocks until a review repair ran"));
        }
        let mut report = PhaseReport::completed(PhaseSource::Deterministic);
        if project_root.join(CONTINUATION_MARKER).is_file()
            && continuation.is_some_and(|payload| !payload.is_empty())
        {
            report.next_continuation = if project_root.join(CONTINUATION_CLEAR_MARKER).is_file() {
                Some(Vec::new())
            } else {
                Some(CONTINUATION_V2.to_vec())
            };
        }
        Ok(report)
    }

    /// Marker file (project-root-relative) that flips the preflight merge
    /// gate to a failed report while it exists.
    pub const FAIL: &str = "mock-fail-merge-preflight";

    /// Marker file (project-root-relative) that flips the postflight merge
    /// gate to a failed report while it exists.
    pub const FAIL_MERGE: &str = "mock-fail-merge-postflight";

    /// One phased merge gate: a success report with no outputs, unless the
    /// id selects a failure profile or the matching per-phase marker file
    /// exists at the project root.
    ///
    /// # Errors
    ///
    /// `Internal` when the id selects the `fail-merge` profile.
    pub fn merge(root: &Path, id: &str, _slice: &str, phase: MergePhase) -> Result<Report, Error> {
        if id.contains("fail-merge") {
            return Err(Error::Internal(format!("mock merge failure for `{id}`")));
        }
        let marker = match phase {
            MergePhase::Preflight => FAIL,
            MergePhase::Postflight => FAIL_MERGE,
        };
        let status = if root.join(marker).is_file() { Status::Failure } else { Status::Success };
        Ok(report(status, Vec::new()))
    }

    // The seam report carries no envelope keys (`version`, `slice`,
    // `target`) — the workflow provider stamps them when widening.
    const fn report(status: Status, outputs: Vec<BuildOutput>) -> Report {
        Report {
            status,
            findings: Vec::new(),
            outputs,
            ui_surface: None,
        }
    }

    /// The mock only ever builds for the core platform.
    const fn core_output(path: String) -> BuildOutput {
        BuildOutput {
            platform: Platform::Core,
            path,
        }
    }

    /// A completed report blocked by one model-assisted violation —
    /// the mock's one blocking-profile shape (report source covers
    /// the finding source).
    fn blocked(detail: &str) -> PhaseReport {
        let mut report = PhaseReport::completed(PhaseSource::ModelAssisted);
        report.findings = vec![blocking_finding(detail)];
        report
    }

    /// A well-formed blocking phase finding: an `important`
    /// model-assisted violation on `code` with snippet evidence. The
    /// engine renumbers ids and recomputes the fingerprint.
    fn blocking_finding(detail: &str) -> PhaseFinding {
        PhaseFinding {
            id: "MOCK-0001".to_string(),
            rule_id: None,
            related_rule_ids: Vec::new(),
            title: detail.to_string(),
            severity: Severity::Important,
            source: DiagnosticSource::ModelAssisted,
            kind: FindingKind::Violation,
            artifact: FindingArtifact::Code,
            location: None,
            evidence: FindingEvidence::Snippet {
                value: detail.to_string(),
            },
            impact: detail.to_string(),
            remediation: "remove the control marker or run the matching repair".to_string(),
            confidence: Some(FindingConfidence::Medium),
            fingerprint: String::new(),
        }
    }

    /// A non-blocking deterministic audit finding for repair reports
    /// (suggestion severity keeps it off every blocking gate).
    fn repair_finding(detail: &str) -> PhaseFinding {
        let mut finding = blocking_finding(detail);
        finding.severity = Severity::Suggestion;
        finding.source = DiagnosticSource::Deterministic;
        finding.confidence = None;
        finding
    }

    /// The version- and axis-stripped adapter name of a routed id.
    fn adapter_name(id: &str) -> &str {
        let name = id.rsplit(':').next().unwrap_or(id);
        name.split_once('@').map_or(name, |(stem, _)| stem)
    }

    fn workspace_write(path: &str) -> PhaseWrite {
        PhaseWrite {
            root: PhaseRoot::Workspace,
            path: path.to_string(),
        }
    }

    fn stage_write(path: &str) -> PhaseWrite {
        PhaseWrite {
            root: PhaseRoot::Artifacts,
            path: path.to_string(),
        }
    }

    fn write_file(path: &Path, body: &str) -> Result<(), Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| Error::Io(err.to_string()))?;
        }
        std::fs::write(path, body).map_err(|err| Error::Io(err.to_string()))
    }

    /// Append one line to `path`, creating the file when absent.
    fn append_line(path: &Path, line: &str) -> Result<(), Error> {
        use std::io::Write as _;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| Error::Io(err.to_string()))?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|err| Error::Io(err.to_string()))?;
        writeln!(file, "{line}").map_err(|err| Error::Io(err.to_string()))
    }

    /// The written build artifact body: slice identity plus per-variant
    /// input counts, so tests can assert the build saw its inputs.
    fn build_artifact(id: &str, slice: &str, inputs: &[Input]) -> String {
        let mut proposal = 0_usize;
        let mut design = 0_usize;
        let mut tasks = 0_usize;
        let mut specs = 0_usize;
        let mut other = 0_usize;
        for input in inputs {
            match input {
                Input::Proposal(_) => proposal += 1,
                Input::Design(_) => design += 1,
                Input::Tasks(_) => tasks += 1,
                Input::Spec(_) => specs += 1,
                Input::Other(_) => other += 1,
            }
        }
        format!(
            "# Fixture build — {slice}\n\nBuilt by `{id}`.\n\nInputs: proposal {proposal}, design \
         {design}, tasks {tasks}, specs {specs}, other {other}.\n"
        )
    }

    /// The absolute path of the build artifact [`build`] writes for
    /// `slice` — for test assertions.
    #[must_use]
    pub fn build_artifact_path(root: &Path, slice: &str) -> PathBuf {
        root.join(BUILD_DIR).join(format!("{slice}.md"))
    }
}
