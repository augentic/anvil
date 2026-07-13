//! Reconciliation through the native fixture pair: surveyed leads
//! become complete, non-duplicated slice assignments — cross-source
//! overlap merges into one slice, a recorded divergence survives the
//! projection, and a grouping that drops a lead is refused by the
//! kernel inside the repair loop.

use std::fs;
use std::path::{Path, PathBuf};

use change::{Divergence, plan};
use serde_json::json;
use testkit::{ReplayProvider, answers, run};

/// The committed replay fixtures for one test — regenerate with
/// `REGENERATE_FIXTURES=1 cargo nextest run -p change reconciliation`.
/// One directory per test: both tests answer the same author prompt
/// differently.
fn fixtures(test: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/replay/reconciliation").join(test)
}

/// One initial dispatch plus every repair attempt. Mirrors the
/// private `project::judgment::MAX_REPAIRS` (2) — kept local rather
/// than widening the module for tests; a budget change shows up here
/// as an off-by-one request count.
const JUDGMENT_BUDGET: usize = 3;

/// A defective grouping that never references the `password-reset`
/// lead — the coverage rule the kernel enforces inside the repair loop.
fn uncovered_grouping_answer() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": [
            {
                "name": "login-flow",
                "sources": [
                    { "source": "docs", "lead": "login-flow" },
                    { "source": "code", "lead": "login-flow" }
                ]
            },
            {
                "name": "session-policy",
                "sources": [
                    { "source": "docs", "lead": "session-timeout" },
                    { "source": "code", "lead": "session-timeout" }
                ]
            }
        ],
        "gate": {
            "change": "## Intent\n\nIncomplete grouping.\n\n## Scope\n\nTwo slices.",
            "discovery-summary": "Sources: 2. Leads: 5.",
            "discovery-source-inventory": "| key | adapter | binding |\n|---|---|---|\n| docs | fixture-docs | x |\n| code | fixture-code | x |"
        }
    }))
    .expect("grouping serialises")
}

async fn author(
    provider: &ReplayProvider,
) -> Result<plan::handlers::AuthorBody, project::handler::Error> {
    run::<plan::handlers::Author, _, _>(
        provider,
        plan::handlers::AuthorInput {
            name: "auth".to_string(),
            sources: answers::adversarial_bindings(),
            intent: None,
        },
    )
    .await
}

// Cross-source overlap merges into one slice, and a recorded
// divergence survives the projection.
#[tokio::test]
async fn overlap_merges() {
    let provider = ReplayProvider::replay(
        "fixture",
        &fixtures("overlap_merges"),
        vec![answers::adversarial_grouping()],
    );

    let authored = author(&provider).await.expect("author walks to pending");
    assert_eq!(authored.slices, ["login-flow", "session-policy", "password-reset"]);
    // Both fixture sources surveyed (key order), docs with its three
    // leads including the docs-only password-reset.
    assert_eq!(authored.surveyed.len(), 2);
    assert_eq!(authored.surveyed[0].source, "code");
    assert_eq!(authored.surveyed[0].leads, ["login-flow", "session-timeout"]);
    assert_eq!(authored.surveyed[1].source, "docs");
    assert_eq!(authored.surveyed[1].leads, ["login-flow", "session-timeout", "password-reset"]);

    let plan: change::Plan = serde_saphyr::from_str(
        &fs::read_to_string(provider.root.join("plan.yaml")).expect("read plan.yaml"),
    )
    .expect("parse plan.yaml");

    // The overlap merged: one slice carries both sources' leads.
    let login = plan.entries.iter().find(|e| e.name == "login-flow").expect("login slice");
    let mut login_sources: Vec<(String, String)> = login
        .sources
        .iter()
        .map(|b| (b.source.clone(), b.lead.clone().unwrap_or_else(|| "login-flow".to_string())))
        .collect();
    login_sources.sort();
    assert_eq!(
        login_sources,
        [
            ("code".to_string(), "login-flow".to_string()),
            ("docs".to_string(), "login-flow".to_string())
        ]
    );

    // The divergence flag and its recorded disagreement survive.
    let session = plan.entries.iter().find(|e| e.name == "session-policy").expect("session slice");
    assert_eq!(session.divergence, Some(Divergence::Likely));
    assert_eq!(session.disagreements.len(), 1);
    assert_eq!(session.disagreements[0].field, "session-timeout-minutes");
}

#[tokio::test]
async fn uncovered_lead_exhausts() {
    // The same defective grouping for the whole budget: the first
    // dispatch plus every repair attempt, so the leg surfaces the
    // kernel's refusal.
    let provider = ReplayProvider::replay(
        "fixture",
        &fixtures("uncovered_exhausts"),
        vec![uncovered_grouping_answer(); JUDGMENT_BUDGET],
    );

    let err = author(&provider).await.expect_err("coverage gap refused");
    let detail = err.to_string();
    assert!(detail.contains("password-reset"), "{detail}");

    // One initial dispatch plus MAX_REPAIRS re-prompts.
    assert_eq!(provider.model().requests().len(), JUDGMENT_BUDGET);
}
