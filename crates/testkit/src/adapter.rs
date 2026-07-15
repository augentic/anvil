//! The Specify-owned harness adapter's native core.
//!
//! One deterministic, model-free library implementing both
//! `specify:adapter` axes for engine tests. The core speaks the
//! engine's own seam DTOs ([`project::seam`], [`artifacts::evidence`])
//! on both targets, so the native test provider passes values straight
//! through and only the WASM adapter guest maps to the WIT records.
//!
//! Behaviour keys off the routed adapter id: an id containing `docs`
//! or `code` selects that half of the adversarial pair, anything else
//! the minimal single-lead `greeting` profile, and `fail-*` substrings
//! select typed failures. Builds and merge gates also
//! honour the per-project [`FAIL_BUILD_MARKER`] /
//! [`FAIL_MERGE_PREFLIGHT_MARKER`] / [`FAIL_MERGE_POSTFLIGHT_MARKER`]
//! files so interruption tests can park and resume without rebinding.

pub use metadata::metadata_json;
pub use source::{extract, survey};
pub use targets::{
    BUILD_DIR, FAIL_BUILD_MARKER, FAIL_MERGE_POSTFLIGHT_MARKER, FAIL_MERGE_PREFLIGHT_MARKER, build,
    build_artifact_path, guidance, merge, target_platforms,
};

mod source {
    use artifacts::evidence::{AuthorityClass, Backing, Claim, ClaimKind};
    use project::seam::{Error, Evidence, Lead};

    /// Survey the source selected by `id` into its controlled lead set.
    ///
    /// # Errors
    ///
    /// `Internal` when the id selects the `fail-survey` profile.
    pub fn survey(id: &str) -> Result<Vec<Lead>, Error> {
        if id.contains("fail-survey") {
            return Err(Error::Internal(format!("fixture survey failure for `{id}`")));
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
            return Err(Error::Internal(format!("fixture extract failure for `{id}`")));
        }
        let evidence = match (profile(id), lead.lead.as_str()) {
            (Profile::Docs, "login-flow") => Evidence {
                authority: AuthorityClass::Documentation,
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
                authority: AuthorityClass::Documentation,
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
                authority: AuthorityClass::Documentation,
                claims: vec![{
                    let mut claim = Claim::new(ClaimKind::Section);
                    claim.id = Some("password-reset.mention".to_string());
                    claim.synopsis = Some("Password reset exists".to_string());
                    claim.set_backing(Some(Backing::Payload(
                        "A password reset flow is mentioned with no defined behaviour.".to_string(),
                    )));
                    claim
                }],
            },
            (Profile::Code, "login-flow") => Evidence {
                authority: AuthorityClass::Behaviour,
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
                authority: AuthorityClass::Behaviour,
                claims: vec![requirement(
                    "session.timeout",
                    "Observed session TTL",
                    "SESSION_TTL expires sessions after 15 minutes of inactivity.",
                )],
            },
            (Profile::Minimal, "greeting") => Evidence {
                authority: AuthorityClass::Documentation,
                claims: vec![requirement(
                    "greeting.behaviour",
                    "Greeting behaviour",
                    "GET /greeting returns the static string 'hello'.",
                )],
            },
            (_, unknown) => {
                return Err(Error::InvalidRequest(format!(
                    "fixture source `{id}` surveys no lead `{unknown}`"
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
        let mut claim = Claim::new(ClaimKind::Requirement);
        claim.id = Some(id.to_string());
        claim.synopsis = Some(synopsis.to_string());
        claim.set_backing(Some(Backing::Payload(statement.to_string())));
        claim
    }

    fn criterion(id: &str, body: &str) -> Claim {
        let mut claim = Claim::new(ClaimKind::Criterion);
        claim.id = Some(id.to_string());
        claim.set_backing(Some(Backing::Payload(body.to_string())));
        claim
    }
}

mod targets {
    use std::path::{Path, PathBuf};

    use project::adapter::PlatformsCapability;
    use project::platform::Platform;
    use project::seam::wire::{BUILD_VERSION, BuildOutput, BuildReport, BuildStatus};
    use project::seam::{Error, Input, MergePhase};

    /// The platform capability a target identity declares — deterministic
    /// per id, so one artifact stands in for several capability shapes
    /// (real adapters compile in one answer):
    ///
    /// - a name containing `limited` requires platforms from
    ///   `{core, ios}`;
    /// - a name containing `platforms` requires platforms from
    ///   `{core, ios, android}`;
    /// - anything else is platform-agnostic (`None`).
    #[must_use]
    pub fn target_platforms(id: &str) -> Option<PlatformsCapability> {
        if id.contains("limited") {
            Some(PlatformsCapability {
                required: true,
                allowed: vec![Platform::Core, Platform::Ios],
                default: vec![Platform::Core, Platform::Ios],
            })
        } else if id.contains("platforms") {
            Some(PlatformsCapability {
                required: true,
                allowed: vec![Platform::Core, Platform::Ios, Platform::Android],
                default: vec![Platform::Core, Platform::Ios, Platform::Android],
            })
        } else {
            None
        }
    }

    /// Marker file (project-root-relative) that flips builds to a failed
    /// report while it exists.
    pub const FAIL_BUILD_MARKER: &str = "fixture-fail-build";

    /// Directory (project-root-relative) fixture builds write their
    /// observable output into.
    pub const BUILD_DIR: &str = "fixture-build";

    /// The deterministic guidance brief served to synthesis.
    ///
    /// # Errors
    ///
    /// `Internal` when the id selects the `fail-guidance` profile.
    pub fn guidance(id: &str) -> Result<String, Error> {
        if id.contains("fail-guidance") {
            return Err(Error::Internal(format!("fixture guidance failure for `{id}`")));
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
    pub fn build(
        root: &Path, id: &str, slice: &str, inputs: &[Input],
    ) -> Result<BuildReport, Error> {
        if id.contains("fail-build") {
            return Err(Error::Internal(format!("fixture build failure for `{id}`")));
        }
        if id.contains("missing-output") {
            // A dishonest success: the declared output is never written, so
            // the caller's outputs-exist gate must abort the build.
            return Ok(report(
                id,
                slice,
                BuildStatus::Success,
                vec![core_output(format!("{BUILD_DIR}/{slice}-never-written.md"))],
            ));
        }
        if root.join(FAIL_BUILD_MARKER).is_file() {
            return Ok(report(id, slice, BuildStatus::Failure, Vec::new()));
        }
        let relative = format!("{BUILD_DIR}/{slice}.md");
        let path = root.join(&relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| Error::Io(err.to_string()))?;
        }
        std::fs::write(&path, build_artifact(id, slice, inputs))
            .map_err(|err| Error::Io(err.to_string()))?;
        Ok(report(id, slice, BuildStatus::Success, vec![core_output(relative)]))
    }

    /// Marker file (project-root-relative) that flips the preflight merge
    /// gate to a failed report while it exists.
    pub const FAIL_MERGE_PREFLIGHT_MARKER: &str = "fixture-fail-merge-preflight";

    /// Marker file (project-root-relative) that flips the postflight merge
    /// gate to a failed report while it exists.
    pub const FAIL_MERGE_POSTFLIGHT_MARKER: &str = "fixture-fail-merge-postflight";

    /// One phased merge gate: a success report with no outputs, unless the
    /// id selects a failure profile or the matching per-phase marker file
    /// exists at the project root.
    ///
    /// # Errors
    ///
    /// `Internal` when the id selects the `fail-merge` profile.
    pub fn merge(
        root: &Path, id: &str, slice: &str, phase: MergePhase,
    ) -> Result<BuildReport, Error> {
        if id.contains("fail-merge") {
            return Err(Error::Internal(format!("fixture merge failure for `{id}`")));
        }
        let marker = match phase {
            MergePhase::Preflight => FAIL_MERGE_PREFLIGHT_MARKER,
            MergePhase::Postflight => FAIL_MERGE_POSTFLIGHT_MARKER,
        };
        let status =
            if root.join(marker).is_file() { BuildStatus::Failure } else { BuildStatus::Success };
        Ok(report(id, slice, status, Vec::new()))
    }

    /// A fully stamped [`BuildReport`] envelope — the same stamping the
    /// engine's guest shim applies when widening a WIT report.
    fn report(
        id: &str, slice: &str, status: BuildStatus, outputs: Vec<BuildOutput>,
    ) -> BuildReport {
        BuildReport {
            version: BUILD_VERSION,
            slice: slice.to_string(),
            target: id.strip_prefix("target:").unwrap_or(id).to_string(),
            status,
            findings: Vec::new(),
            outputs,
            ui_surface: None,
        }
    }

    /// The fixture only ever builds for the core platform.
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

mod metadata {
    use serde_json::json;

    use super::targets::target_platforms;

    /// The deterministic resolve-time metadata JSON a routed adapter id
    /// answers.
    ///
    /// This is the one id-keyed convention behind both the direct
    /// fixture resolution and the stub metadata runner fed to the
    /// shipped `resolver::Component`. The special identities:
    ///
    /// - `target:demo-target` — a `specify` floor newer than any real
    ///   binary (the `adapter-cli-too-old` gate);
    /// - `target:bad-floor` — an unparseable floor
    ///   (`adapter-floor-malformed`);
    /// - `target:vectis` — declared build inputs plus the full
    ///   three-platform capability;
    /// - ids matching a [`target_platforms`] profile (`limited`,
    ///   `platforms`) — that profile's capability;
    /// - anything else — `{}` (no floor, no inputs, no capability).
    #[must_use]
    pub fn metadata_json(adapter_id: &str) -> String {
        match adapter_id {
            "target:demo-target" => r#"{"specify-floor":"999.0.0"}"#.to_string(),
            "target:bad-floor" => r#"{"specify-floor":"v1"}"#.to_string(),
            "target:vectis" => json!({
                "inputs": [
                    { "path": "tokens.yaml", "required": true },
                    { "path": "assets.yaml", "required": false },
                ],
                "platforms": {
                    "required": true,
                    "allowed": ["core", "ios", "android"],
                    "default": ["core", "ios", "android"],
                },
            })
            .to_string(),
            id => target_platforms(id).map_or_else(
                || "{}".to_string(),
                |capability| {
                    json!({
                        "platforms": {
                            "required": capability.required,
                            "allowed": platform_names(&capability.allowed),
                            "default": platform_names(&capability.default),
                        },
                    })
                    .to_string()
                },
            ),
        }
    }

    fn platform_names(platforms: &[project::platform::Platform]) -> Vec<String> {
        platforms.iter().map(ToString::to_string).collect()
    }
}
