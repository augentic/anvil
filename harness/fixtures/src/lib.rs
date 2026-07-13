//! The Specify-owned fixture adapter: one deterministic, model-free
//! native core implementing both `specify:adapter` axes for engine
//! tests. Its types mirror the WIT `specify:adapter/types` records so
//! both consumers (the native test provider and the wasm32
//! `fixture_adapter` example) stay thin mapping layers.
//!
//! Behaviour keys off the routed adapter id — the profile catalog
//! lives in the package `README.md`. Builds and merge gates also
//! honour the per-project [`FAIL_BUILD_MARKER`] /
//! [`FAIL_MERGE_PREFLIGHT_MARKER`] / [`FAIL_MERGE_POSTFLIGHT_MARKER`]
//! files so interruption tests can park and resume without rebinding.

use std::path::{Path, PathBuf};

/// Typed adapter failure, mirroring the WIT `types.error` variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The request itself is malformed; retrying unchanged is pointless.
    InvalidRequest(String),
    /// A filesystem operation failed on the adapter side.
    Io(String),
    /// An internal adapter step failed.
    Internal(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(detail) => write!(f, "invalid request: {detail}"),
            Self::Io(detail) => write!(f, "io: {detail}"),
            Self::Internal(detail) => write!(f, "internal: {detail}"),
        }
    }
}

impl std::error::Error for Error {}

/// One lead surfaced by a survey (the WIT `lead` record).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lead {
    /// Stable kebab-case lead id, unique within this source.
    pub lead: String,
    /// Headline used for cross-source reconciliation.
    pub synopsis: String,
    /// Per-lead topic slugs.
    pub topics: Vec<String>,
}

/// Document-level Evidence authority (the WIT `authority` enum).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Authority {
    /// Operator intent — the highest class.
    Intent,
    /// Written documentation.
    Documentation,
    /// Observed behaviour — the lowest class.
    Behaviour,
}

/// The claim kinds the fixture emits (a subset of the closed WIT
/// `claim-kind` taxonomy).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimKind {
    /// A behavioural requirement (id required).
    Requirement,
    /// An acceptance criterion (id required).
    Criterion,
    /// A prose section.
    Section,
}

/// The claim's backing (the WIT `backing` variant).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Backing {
    /// Inline payload body.
    Payload(String),
    /// Filesystem pointer.
    Path(String),
}

/// One extracted Evidence claim (the WIT `claim` record).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    /// Claim kind.
    pub kind: ClaimKind,
    /// Stable dotted-kebab claim id.
    pub id: Option<String>,
    /// `<path>#L<n>` anchor.
    pub path: Option<String>,
    /// One-line synopsis.
    pub synopsis: Option<String>,
    /// Claim backing.
    pub backing: Option<Backing>,
}

/// The per-lead result of an extract (the WIT `evidence` record).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    /// Document-level authority class.
    pub authority: Authority,
    /// Extracted claims.
    pub claims: Vec<Claim>,
}

/// One slice-artifact input to a build (the WIT `input` variant).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    /// The slice's `proposal.md` body.
    Proposal(String),
    /// The slice's `design.md` body.
    Design(String),
    /// The slice's `tasks.md` body.
    Tasks(String),
    /// One behavioural spec body.
    Spec(String),
    /// Any additional artifact body.
    Other(String),
}

/// Build status (the WIT `status` enum).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// The build succeeded.
    Success,
    /// The build failed.
    Failure,
}

/// One build output — the path half of the WIT `build-output` record.
/// The fixture only ever builds for the core platform, so the mapping
/// layers stamp `platform: core` when widening.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    /// Project-root-relative output path.
    pub path: String,
}

/// A build or merge report (the WIT `report` record, minus findings
/// and UI surface — the fixture never emits either).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Terminal status.
    pub status: Status,
    /// Core-platform outputs the build wrote.
    pub outputs: Vec<Output>,
}

/// The platforms the fixture's capability shapes mention (the subset
/// of the WIT `platform` enum the fixture ever declares).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// The mandatory core platform.
    Core,
    /// iOS shell support.
    Ios,
    /// Android shell support.
    Android,
}

/// A target's declared platform capability (the WIT
/// `platforms-capability` record).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformsCapability {
    /// Whether the project must declare a platform set at init.
    pub required: bool,
    /// The platforms the target can build for.
    pub allowed: Vec<Platform>,
    /// The set assumed when the operator declares none.
    pub default: Vec<Platform>,
}

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
            lead("login-flow", "signIn(email, password) handler in the auth module.", &["auth"]),
            lead("session-timeout", "Session TTL constant in the session store.", &["auth"]),
        ],
        Profile::Minimal => {
            vec![lead("greeting", "The greeting endpoint returns a static string.", &["greeting"])]
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
                "fixture source `{id}` surveys no lead `{unknown}`"
            )));
        }
    };
    Ok(evidence)
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
        return Err(Error::Internal(format!("fixture build failure for `{id}`")));
    }
    if id.contains("missing-output") {
        // A dishonest success: the declared output is never written, so
        // the caller's outputs-exist gate must abort the build.
        return Ok(Report {
            status: Status::Success,
            outputs: vec![Output {
                path: format!("{BUILD_DIR}/{slice}-never-written.md"),
            }],
        });
    }
    if root.join(FAIL_BUILD_MARKER).is_file() {
        return Ok(Report {
            status: Status::Failure,
            outputs: Vec::new(),
        });
    }
    let relative = format!("{BUILD_DIR}/{slice}.md");
    let path = root.join(&relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| Error::Io(err.to_string()))?;
    }
    std::fs::write(&path, build_artifact(id, slice, inputs))
        .map_err(|err| Error::Io(err.to_string()))?;
    Ok(Report {
        status: Status::Success,
        outputs: vec![Output { path: relative }],
    })
}

/// Which side of the engine's deterministic core merge a merge gate
/// runs on (the WIT `merge-phase` enum).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergePhase {
    /// Before the deterministic commit.
    Preflight,
    /// After the commit and archive.
    Postflight,
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
pub fn merge(root: &Path, id: &str, _slice: &str, phase: MergePhase) -> Result<Report, Error> {
    if id.contains("fail-merge") {
        return Err(Error::Internal(format!("fixture merge failure for `{id}`")));
    }
    let marker = match phase {
        MergePhase::Preflight => FAIL_MERGE_PREFLIGHT_MARKER,
        MergePhase::Postflight => FAIL_MERGE_POSTFLIGHT_MARKER,
    };
    let status = if root.join(marker).is_file() { Status::Failure } else { Status::Success };
    Ok(Report {
        status,
        outputs: Vec::new(),
    })
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
