//! Mock source component with behavior selected by routed adapter ID.
#![cfg(target_arch = "wasm32")]

use std::future::Future;

use emery_adapter::registry::Doc;
use emery_adapter::seam::{
    Authority, Backing, Claim, ClaimKind, Context, Error, Evidence, SourceContent, SourceInput,
    SourceMetadata,
};
use emery_adapter::{Model, SourceAdapter};

// This development-only identity must never match a release pin.
#[derive(Clone, Copy, Debug)]
struct Mock;

emery_adapter::source!(Mock);

const DOCS: &[Doc] = &[Doc {
    path: "reference.md",
    body: "# Adapter Reference\n\nThe mock source adapter serves deterministic extract data on \
           the source interface.\n",
}];

impl SourceAdapter for Mock {
    const IDENTITY: &str = "source@0.1.0";

    fn metadata() -> SourceMetadata {
        SourceMetadata { emery_floor: None }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    fn extract<P: Model>(
        _model: &P, ctx: &Context<'_>, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send {
        std::future::ready(extract(ctx.adapter_id, input))
    }
}

fn extract(id: &str, input: &SourceInput) -> Result<Evidence, Error> {
    if id.contains("fail-extract") {
        return Err(Error::Internal(format!("mock extract failure for `{id}`")));
    }
    // Deliberately violates A8 to exercise the engine's fail-closed extras gate.
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
    Ok(evidence_for(profile(id), input))
}

fn evidence_for(profile: Profile, input: &SourceInput) -> Evidence {
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
                requirement("session.timeout", "Documented session expiry", &session_policy(input)),
            ],
        },
        // Deliberate conflict: documentation's 30 minutes outranks behavior's 15.
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
        // Intent resolves the adversarial timeout pair by outranking both sources.
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

// The workspace override lets the journey change one claim between runs.
fn session_policy(input: &SourceInput) -> String {
    const DEFAULT: &str = "Sessions expire after 30 minutes of inactivity.";
    const OVERRIDE: &str = "session-policy.md";
    let SourceContent::Workspace(workspace) = &input.content else {
        return DEFAULT.to_string();
    };
    // Wasm falls back to `.` because the journey binds the project as its preopen.
    std::fs::read_to_string(std::path::Path::new(&workspace.root).join(OVERRIDE))
        .or_else(|_| std::fs::read_to_string(OVERRIDE))
        .map_or_else(|_| DEFAULT.to_string(), |text| text.trim().to_string())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Profile {
    Docs,
    Code,
    Intent,
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

// Mirror first-party per-kind extras so the seam's A8 conservation is covered.
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
