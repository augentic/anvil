//! The scripted judgment-answer corpus over the mock data sets.
//!
//! One copy of each answer stops the suites drifting apart on envelope
//! shape; the corpus is plain JSON and depends on no engine crate.

use serde_json::json;

fn quiet_assessment() -> serde_json::Value {
    json!({
        "behavioural-breadth": 1,
        "coupling": 1,
        "uncertainty": 1,
        "context-volume": 1,
        "verification-surface": 1
    })
}

fn loud_assessment() -> serde_json::Value {
    json!({
        "behavioural-breadth": 10,
        "coupling": 10,
        "uncertainty": 10,
        "context-volume": 10,
        "verification-surface": 10
    })
}

/// Partition + change-prose answers for the degenerate greeting author.
#[must_use]
pub fn greeting_author() -> Vec<String> {
    vec![greeting_leaf(), greeting_change()]
}

/// Degenerate `leaf` partition: one intent lead, target `app`.
///
/// # Panics
///
/// Panics when the value stops serialising.
#[must_use]
pub fn greeting_leaf() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "leaf",
        "target": "app",
        "slice": "greeting",
        "ownership": ["."],
        "acceptance": "The greeting endpoint returns a static string.",
        "sources": [{ "source": "intent", "lead": "intent" }],
        "assessment": quiet_assessment(),
        "rationale": "One intent lead, one slice."
    }))
    .expect("leaf serialises")
}

/// `change.md` review prose for the degenerate greeting author.
///
/// # Panics
///
/// Panics when the value stops serialising.
#[must_use]
pub fn greeting_change() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": [{
            "name": "greeting",
            "target": "app",
            "sources": [{ "source": "intent", "lead": "intent" }],
            "rationale": "One intent lead, one slice."
        }],
        "gate": {
            "change": "## Intent\n\nCharacterise the greeting service.\n\n## Scope\n\nOne slice."
        }
    }))
    .expect("change serialises")
}

/// The reconciliation grouping for the minimal profile: one lead, one
/// slice. Same envelope as [`greeting_change`].
///
/// # Panics
///
/// Panics when the grouping value stops serialising.
#[must_use]
pub fn greeting_grouping() -> String {
    greeting_change()
}

/// High-score leaf that triggers boundary review (degenerate intent).
///
/// # Panics
///
/// Panics when the value stops serialising.
#[must_use]
pub fn greeting_leaf_loud() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "leaf",
        "target": "app",
        "slice": "greeting",
        "ownership": ["."],
        "acceptance": "The greeting endpoint returns a static string.",
        "sources": [{ "source": "intent", "lead": "intent" }],
        "assessment": loud_assessment()
    }))
    .expect("loud leaf serialises")
}

/// Boundary review that blocks authoring as unready.
///
/// # Panics
///
/// Panics when the value stops serialising.
#[must_use]
pub fn greeting_unready() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "verdict": "unready",
        "rationale": "the greeting slice exceeds the target envelope"
    }))
    .expect("unready serialises")
}

/// Boundary review that requeues via focused survey of `intent`.
///
/// # Panics
///
/// Panics when the value stops serialising.
#[must_use]
pub fn greeting_focus() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "verdict": "focus",
        "focus": [{ "source": "intent", "lead": "intent" }]
    }))
    .expect("focus serialises")
}

/// Incomplete leaf used as the first (invalid) partition answer.
///
/// # Panics
///
/// Panics when the value stops serialising.
#[must_use]
pub fn greeting_leaf_invalid() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "leaf",
        "target": "app",
        "slice": "greeting",
        "assessment": quiet_assessment()
    }))
    .expect("invalid leaf serialises")
}

/// Overlapping two-child split with no order (blocks after repair).
///
/// # Panics
///
/// Panics when the value stops serialising.
#[must_use]
pub fn greeting_overlap() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "split",
        "assessment": quiet_assessment(),
        "children": [
            {
                "id": "left",
                "sources": [{ "source": "intent", "lead": "intent" }],
                "target": "app",
                "ownership": ["."]
            },
            {
                "id": "right",
                "sources": [{ "source": "intent", "lead": "intent" }],
                "target": "app",
                "ownership": ["."]
            }
        ]
    }))
    .expect("overlap serialises")
}

/// The synthesis answer for the minimal profile's `greeting` slice.
///
/// # Panics
///
/// Panics when the synthesis value stops serialising.
#[must_use]
pub fn greeting_synthesis() -> String {
    serde_json::to_string(&json!({
        "version": 3,
        "kind": "proceed",
        "slice": "greeting",
        "assessment": quiet_assessment(),
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

/// Boundary-escalation synthesis for the degenerate authored `greeting`
/// leaf (`intent` / `intent`). Assessment exceeds the compiled
/// slice-split threshold.
///
/// # Panics
///
/// Panics when the synthesis value stops serialising.
#[must_use]
pub fn greeting_escalation() -> String {
    serde_json::to_string(&json!({
        "version": 3,
        "kind": "boundary-escalation",
        "slice": "greeting",
        "assessment": loud_assessment(),
        "affected": [{ "source": "intent", "lead": "intent" }],
        "rationale": "Evidence supports separately acceptable child boundaries for the greeting surface."
    }))
    .expect("escalation serialises")
}

/// The minimal-profile synthesis with an evidence gap: the sole
/// requirement carries no claims, so authority derivation mints
/// `status: unknown` — the gate-time deferral fixtures ride this.
///
/// # Panics
///
/// Panics when the synthesis value stops serialising.
#[must_use]
pub fn greeting_unknown_synth() -> String {
    serde_json::to_string(&json!({
        "version": 3,
        "kind": "proceed",
        "slice": "greeting",
        "assessment": quiet_assessment(),
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
            "change": "## Intent\n\nCharacterise the auth surface.\n\n## Scope\n\nThree slices.\n\n## Likely divergences\n\n- session-policy: docs say 30 minutes, code says 15."
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
        "version": 3,
        "kind": "proceed",
        "slice": "login-flow",
        "assessment": quiet_assessment(),
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
        "version": 3,
        "kind": "proceed",
        "slice": "password-reset",
        "assessment": quiet_assessment(),
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
