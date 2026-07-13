//! Deterministic source-adapter behavior and its WIT-mirroring types.

use crate::Error;

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
