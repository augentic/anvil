//! Scripted judgment answers and source bindings shared by the
//! fixture-provider suites. Two fixture data sets exist: the minimal
//! single-lead `greeting` profile (full loop, seam failures, repair
//! loop) and the adversarial `docs` / `code` pair (reconciliation,
//! synthesis). Keeping one copy of each answer here stops the suites
//! drifting apart on envelope shape.

use change::plan::wire::SourceAssign;
use serde_json::json;

/// The single `main` binding onto the minimal fixture source.
pub fn greeting_binding() -> Vec<SourceAssign> {
    let main: SourceAssign = serde_json::from_value(
        json!({ "key": "main", "adapter": "fixture", "value": "The greeting service." }),
    )
    .expect("fixture binding parses");
    vec![main]
}

/// The single `main` binding onto the named fixture source adapter
/// (for the typed-failure profiles).
pub fn greeting_binding_for(adapter: &str) -> Vec<SourceAssign> {
    let main: SourceAssign = serde_json::from_value(
        json!({ "key": "main", "adapter": adapter, "value": "The greeting service." }),
    )
    .expect("fixture binding parses");
    vec![main]
}

/// The adversarial two-source pair: a docs source and a code source,
/// both served by the fixture core under different adapter names.
pub fn adversarial_bindings() -> Vec<SourceAssign> {
    ["docs", "code"]
        .map(|key| {
            serde_json::from_value(json!({
                "key": key,
                "adapter": format!("fixture-{key}"),
                "value": format!("The {key} source."),
            }))
            .expect("fixture binding parses")
        })
        .to_vec()
}

/// The reconciliation grouping for the minimal profile: one lead, one
/// slice.
pub fn greeting_grouping() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": [{
            "name": "greeting",
            "sources": [{ "source": "main", "lead": "greeting" }],
            "rationale": "One fixture lead, one slice."
        }],
        "gate": {
            "change": "## Intent\n\nCharacterise the greeting service.\n\n## Scope\n\nOne slice.",
            "discovery-summary": "Sources: 1. Leads: 1.",
            "discovery-source-inventory": "| key | adapter | binding |\n|---|---|---|\n| main | fixture | \"The greeting service.\" |"
        }
    }))
    .expect("grouping serialises")
}

/// The synthesis answer for the minimal profile's `greeting` slice.
pub fn greeting_synthesis() -> String {
    serde_json::to_string(&json!({
        "version": 1,
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
            "proposal": "# greeting\n\n## Why\n\nThe fixture source surfaced it.\n\n## Domains\n\n- greeting — the affected surface\n\n## Non-goals\n\n- Nothing else.\n",
            "design": "# Design\n\nHow the greeting slice lands.\n",
            "tasks": "# Tasks\n\n## Implementation\n\n- [ ] 1.1 Implement the endpoint (TASK-001)\n",
            "specs": [{ "domain": "greeting", "content": "## greeting\nAgent prose body.\n" }]
        }
    }))
    .expect("synthesis serialises")
}

/// The correct grouping over the adversarial lead catalog: the
/// overlapping `login-flow` leads merge into one slice, the
/// `session-timeout` disagreement is flagged as a likely divergence,
/// and the docs-only `password-reset` lead lands alone.
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
            "discovery-source-inventory": "| key | adapter | binding |\n|---|---|---|\n| docs | fixture-docs | \"The docs source.\" |\n| code | fixture-code | \"The code source.\" |"
        }
    }))
    .expect("grouping serialises")
}
