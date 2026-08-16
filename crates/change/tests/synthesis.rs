//! Synthesis over the adversarial mock evidence: an authority
//! disagreement resolves to a `[divergence]` with the documentation
//! source winning, an evidence gap projects an `[unknown]`
//! requirement, and the provenance projection stays complete — all
//! through the public refine / model / provenance operations.

mod support;

use std::fs;

use mock::invoke::run;
use mock::session::Session;
use serde_json::json;

/// Synthesis for `session-policy`: the two `session.timeout` claims
/// disagree, so the answer carries the `disagreed` verdict and the
/// kernel resolves documentation over behaviour.
fn session_synthesis_answer() -> String {
    serde_json::to_string(&json!({
        "version": 4,
        "kind": "proceed",
        "slice": "session-policy",
        "assessment": {
            "behavioural-breadth": 1,
            "coupling": 1,
            "uncertainty": 1,
            "context-volume": 1,
            "verification-surface": 1
        },
        "model": {
            "requirements": [{
                "title": "sessions expire after inactivity",
                "domain": "session",
                "agreement": "disagreed",
                "claims": [
                    { "source": "docs", "id": "session.timeout", "kind": "requirement" },
                    { "source": "code", "id": "session.timeout", "kind": "requirement" }
                ],
                "statement": "Sessions expire after 30 minutes of inactivity.",
                "scenarios": ["An idle session expires"],
                "notes": "Documentation states 30 minutes; observed behaviour is 15."
            }],
            "tasks": [
                { "id": "TASK-001", "text": "Align the session TTL with the documented policy.", "satisfies": ["REQ-001"] }
            ]
        },
        "artifacts": {
            "proposal": "# session-policy\n\n## Why\n\nThe sources disagree on expiry.\n\n## Domains\n\n- session — the affected surface\n\n## Non-goals\n\n- Nothing else.\n",
            "design": "# Design\n\nHow session-policy lands.\n",
            "tasks": "# Tasks\n\n## Implementation\n\n- [ ] 1.1 Align the TTL (TASK-001)\n",
            "specs": [{ "domain": "session", "content": "## session\nAgent prose body.\n" }]
        }
    }))
    .expect("synthesis serialises")
}

/// Synthesis for `password-reset`: the evidence gap — the sole lead
/// carries no behavioural detail, so the faithful answer records an
/// unanchored requirement (zero claims) the kernel marks `[unknown]`.
fn reset_synthesis_answer() -> String {
    serde_json::to_string(&json!({
        "version": 4,
        "kind": "proceed",
        "slice": "password-reset",
        "assessment": {
            "behavioural-breadth": 1,
            "coupling": 1,
            "uncertainty": 1,
            "context-volume": 1,
            "verification-surface": 1
        },
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

fn author(session: &Session) {
    support::write_adversarial_plan(session.root());
}

// A documentation-vs-behaviour disagreement resolves as a divergence
// with the docs claim winning.
#[tokio::test]
async fn divergence_docs_wins() {
    let session = Session::scripted("mock", vec![session_synthesis_answer()]);
    author(&session);

    let refined = support::refine(&session, "session-policy")
        .await
        .expect("refine synthesises the divergent slice");
    let slice::orchestrate::RefineOutcome::Refined { slice, .. } = refined else {
        panic!("expected proceed, got {refined:?}");
    };
    assert_eq!(slice, "session-policy");

    // The synthesis prompt is path-first: each source row carries the
    // stage-relative `evidence-path` into the lent staged tree
    // (RFC-96 D10), and no claim bodies are inlined in the user
    // message.
    let requests = session.model().requests();
    let request = requests.last().expect("synthesis request recorded");
    let prompt = &request.messages[0].content;
    assert!(prompt.contains("\"evidence-path\": \"evidence/docs.yaml\""), "{prompt}");
    assert!(prompt.contains("\"evidence-path\": \"evidence/code.yaml\""), "{prompt}");
    // The lent workspace is the staged tree, not the project root.
    let workspace = request.workspace.as_deref().expect("synthesis lends the staged tree");
    assert_ne!(workspace, ".");
    assert!(!prompt.contains("\"claims\""), "claims must not be inlined: {prompt}");
    assert!(
        !prompt.contains("Sessions expire after 30 minutes of inactivity."),
        "claim bodies must not be inlined: {prompt}"
    );

    // The kernel resolved the disagreement: divergence, docs winning.
    let model = run::<slice::handlers::ModelShow, _, _>(
        session.provider(),
        slice::handlers::ModelShowInput {
            name: "session-policy".to_string(),
        },
    )
    .await
    .expect("model.yaml loads");
    let requirement = &model.requirements[0];
    assert_eq!(requirement.status.map(|s| s.to_string()), Some("divergence".to_string()));
    // Documentation outranks behaviour: the docs claim wins, sources
    // render highest-authority first.
    assert_eq!(requirement.sources, ["docs", "code"]);
    let winners: Vec<(String, Option<bool>)> =
        requirement.claims.iter().map(|c| (c.source.clone(), c.winner)).collect();
    assert_eq!(winners, [("docs".to_string(), Some(true)), ("code".to_string(), Some(false))]);

    // The written spec carries the inline `[divergence]` tag.
    let spec = fs::read_to_string(
        session.root().join(".emery/change/slices/session-policy/specs/session/spec.md"),
    )
    .expect("slice spec written");
    assert!(spec.contains("[divergence]"), "{spec}");

    // The provenance projection recomputes the authority-resolved
    // label with the docs source as winner.
    let provenance = run::<slice::handlers::Provenance, _, _>(
        session.provider(),
        slice::handlers::ProvenanceInput {
            name: "session-policy".to_string(),
        },
    )
    .await
    .expect("provenance projects");
    let req = &provenance.requirements[0];
    assert_eq!(req.resolution.to_string(), "authority-resolved");
    let trace = req.resolution_trace.as_ref().expect("authority-resolved carries a trace");
    assert_eq!(trace.winner.as_deref(), Some("docs"));
}

/// [`session_synthesis_answer`] plus one accepted Decision Record
/// superseding the baseline `DEC-0001`.
fn synthesis_with_decision() -> String {
    let mut answer: serde_json::Value =
        serde_json::from_str(&session_synthesis_answer()).expect("answer parses");
    answer["artifacts"]["decisions"] = json!([{
        "slug": "session-ttl-source",
        "status": "accepted",
        "title": "Documented TTL wins over observed behaviour",
        "context": "Documentation and observed behaviour disagree on the session TTL.",
        "decision": "The documented 30-minute TTL is authoritative.",
        "consequences": "The observed 15-minute TTL is treated as a defect.",
        "supersedes": ["DEC-0001"],
        "related": ["REQ-001"],
        "topics": ["session"]
    }]);
    serde_json::to_string(&answer).expect("answer serialises")
}

// Decisions persist with baseline context surfaced to synthesis, and
// re-synthesis replaces the slice's decision set exactly: the
// re-refine re-issues the same synthesis prompt and receives a
// *different* answer from the script.
#[tokio::test]
async fn decisions_exact_set() {
    let session =
        Session::scripted("mock", vec![synthesis_with_decision(), session_synthesis_answer()]);
    let root = session.root().to_path_buf();

    // A baseline Decision Record the slice can legally supersede — and
    // the projection the synthesis inputs must surface.
    let baseline_dir = root.join(".emery/decisions");
    fs::create_dir_all(&baseline_dir).expect("create baseline decisions");
    fs::write(
        baseline_dir.join("DEC-0001-session-ttl.md"),
        "---\nid: DEC-0001\nslug: session-ttl\nstatus: accepted\nslice: earlier\ndate: \
         2026-01-01\ntopics: [session]\n---\n# Session TTL\n\n## Context\n\nContext.\n\n## \
         Decision\n\nDecision.\n\n## Consequences\n\nConsequences.\n",
    )
    .expect("write baseline decision");

    author(&session);

    support::refine(&session, "session-policy")
        .await
        .expect("refine persists the decision sidecar");

    // The sidecar carries only slice-authored fields; the engine stamps
    // `id` / `slice` / `date` at merge.
    let sidecar = root.join(".emery/change/slices/session-policy/decisions/session-ttl-source.md");
    let record = fs::read_to_string(&sidecar).expect("decision sidecar written");
    assert!(record.contains("slug: session-ttl-source"), "{record}");
    assert!(record.contains("status: accepted"), "{record}");
    assert!(record.contains("- DEC-0001"), "{record}");
    assert!(record.contains("## Consequences"), "{record}");
    assert!(!record.contains("id:"), "{record}");
    assert!(!record.contains("date:"), "{record}");

    // Re-refine with a decision-free response: the exact-set
    // replacement clears both the generated record and any stray file.
    let slice_dir = root.join(".emery/change/slices/session-policy");
    fs::write(slice_dir.join("decisions/stale.md"), "stale").expect("plant stray file");
    support::refine(&session, "session-policy").await.expect("re-refine replaces the decision set");

    let survivors: Vec<String> = fs::read_dir(slice_dir.join("decisions"))
        .expect("decisions dir")
        .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert!(survivors.is_empty(), "{survivors:?}");

    session.model().assert_exhausted();
}

#[tokio::test]
async fn evidence_gap_projects() {
    let session = Session::scripted("mock", vec![reset_synthesis_answer()]);
    author(&session);

    support::refine(&session, "password-reset").await.expect("refine synthesises the gapped slice");

    let model = run::<slice::handlers::ModelShow, _, _>(
        session.provider(),
        slice::handlers::ModelShowInput {
            name: "password-reset".to_string(),
        },
    )
    .await
    .expect("model.yaml loads");
    let requirement = &model.requirements[0];
    assert_eq!(requirement.status.map(|s| s.to_string()), Some("unknown".to_string()));
    assert!(requirement.claims.is_empty(), "{requirement:?}");

    let spec = fs::read_to_string(
        session.root().join(".emery/change/slices/password-reset/specs/password-reset/spec.md"),
    )
    .expect("slice spec written");
    assert!(spec.contains("[unknown]"), "{spec}");
}
