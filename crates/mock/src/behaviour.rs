//! The mock adapter's deterministic, model-free behaviour core:
//! behaviour keys off the routed adapter id (`docs` / `code` /
//! `intent` / `fail-*` substrings, else the `greeting` profile).

pub use source::extract;

mod source {
    use adapter::seam::{Authority, Backing, Claim, ClaimKind, Error, Evidence, SourceInput};

    /// Extract the controlled Evidence for the source selected by `id`.
    ///
    /// # Errors
    ///
    /// `Internal` when the id selects the `fail-extract` profile.
    pub fn extract(id: &str, _input: &SourceInput) -> Result<Evidence, Error> {
        if id.contains("fail-extract") {
            return Err(Error::Internal(format!("mock extract failure for `{id}`")));
        }
        // The A8 violation profile: a requirement claim without its
        // required `statement` extra, for the engine's fail-closed gate.
        if id.contains("missing-extras") {
            return Ok(Evidence {
                authority: Authority::Documentation,
                claims: vec![Claim {
                    kind: ClaimKind::Requirement,
                    id: Some("greeting.behaviour".to_string()),
                    path: None,
                    synopsis: Some("A requirement without its statement".to_string()),
                    backing: None,
                    extras: serde_json::Map::new(),
                }],
            });
        }
        Ok(evidence_for(profile(id)))
    }

    fn evidence_for(profile: Profile) -> Evidence {
        match profile {
            Profile::Docs => Evidence {
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
                    requirement(
                        "session.timeout",
                        "Documented session expiry",
                        "Sessions expire after 30 minutes of inactivity.",
                    ),
                ],
            },
            // The authority disagreement: behaviour observes 15 minutes
            // where documentation states 30 — docs outrank behaviour.
            Profile::Code => Evidence {
                authority: Authority::Behaviour,
                claims: vec![
                    requirement(
                        "login.flow",
                        "Observed login handler",
                        "signIn validates credentials and issues a session token.",
                    ),
                    requirement(
                        "session.timeout",
                        "Observed session TTL",
                        "SESSION_TTL expires sessions after 15 minutes of inactivity.",
                    ),
                ],
            },
            // The operator directive: intent outranks both halves of
            // the adversarial pair, resolving the session-timeout
            // disagreement by authority precedence.
            Profile::Intent => Evidence {
                authority: Authority::Intent,
                claims: vec![requirement(
                    "session.timeout",
                    "Operator directive on session expiry",
                    "Sessions must expire after 30 minutes of inactivity.",
                )],
            },
            Profile::Minimal => Evidence {
                authority: Authority::Documentation,
                claims: vec![requirement(
                    "greeting.behaviour",
                    "Greeting behaviour",
                    "GET /greeting returns the static string 'hello'.",
                )],
            },
        }
    }

    /// The behaviour profile a routed adapter id selects.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Profile {
        /// Documentation half of the adversarial pair.
        Docs,
        /// Behaviour (code) half of the adversarial pair.
        Code,
        /// The inline operator-intent source.
        Intent,
        /// The single-claim `greeting` profile.
        Minimal,
    }

    fn profile(id: &str) -> Profile {
        if id.contains("docs") {
            Profile::Docs
        } else if id.contains("code") {
            Profile::Code
        } else if id.contains("intent") {
            Profile::Intent
        } else {
            Profile::Minimal
        }
    }

    /// Open per-kind body extras, mirroring the fields first-party
    /// extract prompts emit (A8 — the seam must conserve them).
    fn extras(key: &str, value: &str) -> serde_json::Map<String, serde_json::Value> {
        let mut extras = serde_json::Map::new();
        extras.insert(key.to_string(), serde_json::Value::String(value.to_string()));
        extras
    }

    fn requirement(id: &str, synopsis: &str, statement: &str) -> Claim {
        Claim {
            kind: ClaimKind::Requirement,
            id: Some(id.to_string()),
            path: None,
            synopsis: Some(synopsis.to_string()),
            backing: Some(Backing::Payload(statement.to_string())),
            extras: extras("statement", statement),
        }
    }

    fn criterion(id: &str, body: &str) -> Claim {
        Claim {
            kind: ClaimKind::Criterion,
            id: Some(id.to_string()),
            path: None,
            synopsis: None,
            backing: Some(Backing::Payload(body.to_string())),
            extras: extras("criterion", body),
        }
    }
}
