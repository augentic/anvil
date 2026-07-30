//! The scripted judgment-answer corpus over the mock data sets.
//!
//! Two mock data sets exist: the minimal single-lead `greeting`
//! profile (full loop, seam failures, repair loop) and the adversarial
//! `docs` / `code` pair (reconciliation, synthesis). Keeping one copy
//! of each answer here stops the suites drifting apart on envelope
//! shape. Source-binding builders live with the `change` suites — the
//! corpus itself is plain JSON and depends on no engine crate.

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
