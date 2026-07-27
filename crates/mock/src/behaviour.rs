//! The mock adapter's deterministic, model-free behaviour core.
//!
//! One id-keyed library implementing both `emery:adapter` axes over
//! the SDK seam DTOs ([`adapter::seam`]) — the canonical internal
//! representation. The trait implementors in [`crate::ops`] and the
//! WASM adapter guest both route through it; only the workflow
//! providers (the native host's conversion layer) widen the values onto engine DTOs.
//!
//! Behaviour keys off the routed adapter id: an id containing `docs`
//! or `code` selects that half of the adversarial pair, anything else
//! the minimal single-lead `greeting` profile, and `fail-*` substrings
//! select typed failures. Builds and merge gates also honour the
//! per-project [`FAIL_BUILD_MARKER`] / [`FAIL_MERGE_PREFLIGHT_MARKER`]
//! / [`FAIL_MERGE_POSTFLIGHT_MARKER`] files so interruption tests can
//! park and resume without rebinding.

pub use source::{extract, survey};
pub use targets::{
    BUILD_DIR, FAIL_BUILD_MARKER, FAIL_MERGE_POSTFLIGHT_MARKER, FAIL_MERGE_PREFLIGHT_MARKER, build,
    build_artifact_path, guidance, merge,
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
            // where documentation states 30 — documentation outranks
            // behaviour, so resolution is a `[divergence]` with the docs
            // source winning.
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

    use adapter::seam::{BuildOutput, Error, Input, MergePhase, Platform, Report, Status};

    /// Marker file (project-root-relative) that flips builds to a failed
    /// report while it exists.
    pub const FAIL_BUILD_MARKER: &str = "mock-fail-build";

    /// Directory (project-root-relative) mock builds write their
    /// observable output into.
    pub const BUILD_DIR: &str = "mock-build";

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

    /// Build one slice: write the observable artifact under
    /// [`BUILD_DIR`] and report it as a core-platform output.
    ///
    /// # Errors
    ///
    /// - `Internal` when the id selects the `fail-build` profile.
    /// - `Io` when the artifact cannot be written.
    pub fn build(root: &Path, id: &str, slice: &str, inputs: &[Input]) -> Result<Report, Error> {
        if id.contains("fail-build") {
            return Err(Error::Internal(format!("mock build failure for `{id}`")));
        }
        if id.contains("missing-output") {
            // A dishonest success: the declared output is never written, so
            // the caller's outputs-exist gate must abort the build.
            return Ok(report(
                Status::Success,
                vec![core_output(format!("{BUILD_DIR}/{slice}-never-written.md"))],
            ));
        }
        if root.join(FAIL_BUILD_MARKER).is_file() {
            return Ok(report(Status::Failure, Vec::new()));
        }
        let relative = format!("{BUILD_DIR}/{slice}.md");
        let path = root.join(&relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| Error::Io(err.to_string()))?;
        }
        std::fs::write(&path, build_artifact(id, slice, inputs))
            .map_err(|err| Error::Io(err.to_string()))?;
        Ok(report(Status::Success, vec![core_output(relative)]))
    }

    /// Marker file (project-root-relative) that flips the preflight merge
    /// gate to a failed report while it exists.
    pub const FAIL_MERGE_PREFLIGHT_MARKER: &str = "mock-fail-merge-preflight";

    /// Marker file (project-root-relative) that flips the postflight merge
    /// gate to a failed report while it exists.
    pub const FAIL_MERGE_POSTFLIGHT_MARKER: &str = "mock-fail-merge-postflight";

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
            MergePhase::Preflight => FAIL_MERGE_PREFLIGHT_MARKER,
            MergePhase::Postflight => FAIL_MERGE_POSTFLIGHT_MARKER,
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
