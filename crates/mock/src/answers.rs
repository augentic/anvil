//! The scripted judgment-answer corpus over the mock data sets.
//!
//! One copy of each answer stops the suites drifting apart on envelope
//! shape; the corpus is plain JSON and depends on no engine crate.

use serde_json::json;

/// The reconciliation grouping for the minimal profile: one lead, one
/// slice.
///
/// # Panics
///
/// Panics when the grouping value stops serialising.
#[must_use]
pub fn greeting_grouping() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": [{
            "name": "greeting",
            "sources": [{ "source": "main", "lead": "greeting" }],
            "rationale": "One mock lead, one slice."
        }],
        "gate": {
            "change": "## Intent\n\nCharacterise the greeting service.\n\n## Scope\n\nOne slice.",
            "discovery-summary": "Sources: 1. Leads: 1.",
            "discovery-source-inventory": "| key | adapter | binding |\n|---|---|---|\n| main | mock | \"The greeting service.\" |"
        }
    }))
    .expect("grouping serialises")
}

/// The synthesis answer for the minimal profile's `greeting` slice.
///
/// # Panics
///
/// Panics when the synthesis value stops serialising.
#[must_use]
pub fn greeting_synthesis() -> String {
    serde_json::to_string(&json!({
        "version": 2,
        "kind": "response",
        "slice": "greeting",
        "model": {
            "requirements": [{
                "title": "greeting returns the static string",
                "domain": "greeting",
                "claims": [{ "source": "main", "id": "greeting.behaviour", "kind": "requirement" }],
                "statement": "GET /greeting returns the static string 'hello'.",
                "scenarios": ["A request to /greeting receives 'hello'"]
            }],
            "tasks": [
                { "id": "TASK-001", "text": "Implement the greeting endpoint.", "satisfies": ["REQ-001"] }
            ]
        },
        "artifacts": {
            "proposal": "# greeting\n\n## Why\n\nThe mock source surfaced it.\n\n## Domains\n\n- greeting — the affected surface\n\n## Non-goals\n\n- Nothing else.\n",
            "design": "# Design\n\nHow the greeting slice lands.\n",
            "tasks": "# Tasks\n\n## Implementation\n\n- [ ] 1.1 Implement the endpoint (TASK-001)\n",
            "specs": [{ "domain": "greeting", "content": "## greeting\nAgent prose body.\n" }]
        }
    }))
    .expect("synthesis serialises")
}

/// The minimal-profile synthesis with an evidence gap: the sole
/// requirement carries no claims, so authority derivation mints
/// `status: unknown` — the gate-time deferral fixtures ride this.
///
/// # Panics
///
/// Panics when the synthesis value stops serialising.
#[must_use]
pub fn greeting_unknown_synthesis() -> String {
    serde_json::to_string(&json!({
        "version": 2,
        "kind": "response",
        "slice": "greeting",
        "model": {
            "requirements": [{
                "title": "greeting error handling",
                "domain": "greeting",
                "claims": [],
                "statement": "The greeting service handles errors; behaviour is not evidenced.",
                "scenarios": ["A failing request receives an error (behaviour unspecified)"]
            }],
            "tasks": [
                { "id": "TASK-001", "text": "Emery the error handling.", "satisfies": ["REQ-001"] }
            ]
        },
        "artifacts": {
            "proposal": "# greeting\n\n## Why\n\nThe mock source surfaced it.\n\n## Domains\n\n- greeting — the affected surface\n\n## Non-goals\n\n- Nothing else.\n",
            "design": "# Design\n\nHow the greeting slice lands.\n",
            "tasks": "# Tasks\n\n## Implementation\n\n- [ ] 1.1 Emery the error handling (TASK-001)\n",
            "specs": [{ "domain": "greeting", "content": "## greeting\nAgent prose body.\n" }]
        }
    }))
    .expect("synthesis serialises")
}

/// The correct grouping over the adversarial lead catalog.
///
/// The overlapping `login-flow` leads merge into one slice, the
/// `session-timeout` disagreement is flagged as a likely divergence,
/// and the docs-only `password-reset` lead lands alone.
///
/// # Panics
///
/// Panics when the grouping value stops serialising.
#[must_use]
pub fn adversarial_grouping() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": [
            {
                "name": "login-flow",
                "sources": [
                    { "source": "docs", "lead": "login-flow" },
                    { "source": "code", "lead": "login-flow" }
                ],
                "rationale": "Both sources describe the same sign-in surface."
            },
            {
                "name": "session-policy",
                "sources": [
                    { "source": "docs", "lead": "session-timeout" },
                    { "source": "code", "lead": "session-timeout" }
                ],
                "divergence": "likely",
                "disagreements": [{
                    "field": "session-timeout-minutes",
                    "values": [
                        { "source": "docs", "value": "30 minutes" },
                        { "source": "code", "value": "15 minutes" }
                    ]
                }]
            },
            {
                "name": "password-reset",
                "sources": [{ "source": "docs", "lead": "password-reset" }]
            }
        ],
        "gate": {
            "change": "## Intent\n\nCharacterise the auth surface.\n\n## Scope\n\nThree slices.\n\n## Likely divergences\n\n- session-policy: docs say 30 minutes, code says 15.",
            "discovery-summary": "Sources: 2. Leads: 5.",
            "discovery-source-inventory": "| key | adapter | binding |\n|---|---|---|\n| docs | mock-docs | \"The docs source.\" |\n| code | mock-code | \"The code source.\" |"
        }
    }))
    .expect("grouping serialises")
}

/// Synthesis for the adversarial `login-flow` slice (agreed, both
/// sources).
///
/// # Panics
///
/// Panics when the synthesis value stops serialising.
#[must_use]
pub fn login_flow_synthesis() -> String {
    serde_json::to_string(&json!({
        "version": 2,
        "kind": "response",
        "slice": "login-flow",
        "model": {
            "requirements": [{
                "title": "users sign in with email and password",
                "domain": "auth",
                "claims": [
                    { "source": "docs", "id": "login.flow", "kind": "requirement" },
                    { "source": "code", "id": "login.flow", "kind": "requirement" }
                ],
                "statement": "Users authenticate with an email address and password.",
                "scenarios": ["A valid email and password yield an authenticated session"]
            }],
            "tasks": [
                { "id": "TASK-001", "text": "Implement email/password sign-in.", "satisfies": ["REQ-001"] }
            ]
        },
        "artifacts": {
            "proposal": "# login-flow\n\n## Why\n\nBoth sources describe sign-in.\n\n## Domains\n\n- auth — the affected surface\n\n## Non-goals\n\n- Nothing else.\n",
            "design": "# Design\n\nHow login-flow lands.\n",
            "tasks": "# Tasks\n\n## Implementation\n\n- [ ] 1.1 Implement sign-in (TASK-001)\n",
            "specs": [{ "domain": "auth", "content": "## auth\nAgent prose body.\n" }]
        }
    }))
    .expect("synthesis serialises")
}

/// Synthesis for the adversarial `password-reset` slice (evidence gap).
///
/// # Panics
///
/// Panics when the synthesis value stops serialising.
#[must_use]
pub fn password_reset_synthesis() -> String {
    serde_json::to_string(&json!({
        "version": 2,
        "kind": "response",
        "slice": "password-reset",
        "model": {
            "requirements": [{
                "title": "password reset behaviour",
                "domain": "password-reset",
                "claims": [],
                "statement": "A password reset flow exists; its behaviour is not evidenced.",
                "scenarios": ["A user requests a password reset (behaviour unspecified)"]
            }],
            "tasks": [
                { "id": "TASK-001", "text": "Emery the password reset flow.", "satisfies": ["REQ-001"] }
            ]
        },
        "artifacts": {
            "proposal": "# password-reset\n\n## Why\n\nDocs mention it without detail.\n\n## Domains\n\n- password-reset — the affected surface\n\n## Non-goals\n\n- Nothing else.\n",
            "design": "# Design\n\nHow password-reset lands.\n",
            "tasks": "# Tasks\n\n## Implementation\n\n- [ ] 1.1 Emery the flow (TASK-001)\n",
            "specs": [{ "domain": "password-reset", "content": "## password-reset\nAgent prose body.\n" }]
        }
    }))
    .expect("synthesis serialises")
}
