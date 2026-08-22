//! Synthesis integration tests.

use emery_artifacts::evidence::{AuthorityClass, Claim, ClaimKind};
use emery_artifacts::spec::ast::{Status, Tag};
use emery_engine::extract::SourceSet;
use emery_engine::synthesise::{Row, reconcile, synthesise};
use omnia_testkit::model::Harness;

fn claim(kind: ClaimKind, id: &str, extra: (&str, &str)) -> Claim {
    let mut claim = Claim::new(kind);
    claim.id = Some(id.to_string());
    claim.extras.insert(extra.0.into(), serde_json::Value::String(extra.1.into()));
    claim
}

fn set(key: &str, authority: AuthorityClass, claims: Vec<Claim>) -> SourceSet {
    SourceSet {
        key: key.to_string(),
        adapter: format!("source:{key}"),
        authority,
        claims,
    }
}

// Exercises disagreement, intent precedence, and an acceptance gap.
fn journey_sets() -> Vec<SourceSet> {
    vec![
        set(
            "mock-docs",
            AuthorityClass::Documentation,
            vec![
                claim(
                    ClaimKind::Requirement,
                    "login.flow",
                    ("statement", "Users sign in with a magic link."),
                ),
                claim(
                    ClaimKind::Requirement,
                    "session.timeout",
                    ("statement", "Sessions expire after 30 minutes of inactivity."),
                ),
                claim(
                    ClaimKind::Criterion,
                    "login.flow.success",
                    ("criterion", "A valid link signs the user in."),
                ),
            ],
        ),
        set(
            "mock-code",
            AuthorityClass::Behaviour,
            vec![
                claim(
                    ClaimKind::Requirement,
                    "login.flow",
                    ("statement", "Users sign in with email and password."),
                ),
                claim(
                    ClaimKind::Requirement,
                    "session.timeout",
                    ("statement", "Sessions expire after 15 minutes of inactivity."),
                ),
            ],
        ),
        set(
            "mock-intent",
            AuthorityClass::Intent,
            vec![claim(
                ClaimKind::Requirement,
                "session.timeout",
                ("statement", "Sessions must expire after 30 minutes of inactivity."),
            )],
        ),
    ]
}

fn docs_set(key: &str, statement: &str) -> SourceSet {
    set(
        key,
        AuthorityClass::Documentation,
        vec![claim(ClaimKind::Requirement, "greeting.behaviour", ("statement", statement))],
    )
}

fn spec_answer(rows: &[Row]) -> String {
    use std::fmt::Write as _;
    let mut spec = String::from("# Fixture spec\n");
    for row in rows {
        let tag = row.tag.map(|tag| format!(" [{tag}]")).unwrap_or_default();
        let _ = write!(
            spec,
            "\n### Requirement: {subject}{tag}\n\nID: {id}\nSources: [{sources}]\nStatus: {status}\n\n\
             Body for {subject}.\n\n#### Scenario: One\n\n- **WHEN** x\n- **THEN** y\n",
            subject = row.subject,
            id = row.id,
            sources = row.sources.join(", "),
            status = row.status,
        );
    }
    spec
}

#[test]
fn precedence_resolves() {
    let rows = reconcile(&journey_sets());

    assert_eq!(rows.len(), 3);

    let login = &rows[0];
    assert_eq!(login.id, "REQ-001");
    assert_eq!(login.subject, "login.flow");
    assert_eq!(login.status, Status::Divergence, "docs outrank behaviour");
    assert_eq!(login.sources, ["mock-docs", "mock-code"]);
    assert_eq!(login.winner, Some(0));

    let timeout = &rows[1];
    assert_eq!(timeout.subject, "session.timeout");
    assert_eq!(timeout.status, Status::Divergence, "intent outranks the pair");
    assert_eq!(timeout.sources, ["mock-intent", "mock-docs", "mock-code"]);
    assert_eq!(timeout.winner, Some(0), "the intent directive wins");
    assert!(timeout.contributors[0].statement.contains("30 minutes"));

    let gap = &rows[2];
    assert_eq!(gap.status, Status::Unknown, "no criterion evidences session.timeout");
    assert_eq!(gap.tag, Some(Tag::Unknown));
    assert!(gap.sources.is_empty());
}

#[test]
fn same_class_tie_conflicts() {
    let sets = [
        docs_set("docs-a", "The greeting is 'hello'."),
        docs_set("docs-b", "The greeting is 'hi'."),
    ];
    let rows = reconcile(&sets);
    assert_eq!(rows[0].status, Status::Conflict, "a top-authority tie is never auto-resolved");
    assert_eq!(rows[0].tag, Some(Tag::Conflict));
    assert_eq!(rows[0].winner, None);
}

#[tokio::test]
async fn gated_answers_persist() {
    let sets = journey_sets();
    let rows = reconcile(&sets);

    let model = Harness::answering([spec_answer(&rows), "# Design\n\nThe shape.\n".to_string()]);
    let documents = synthesise(&model, &sets, &rows).await.expect("gated answers pass");
    assert!(documents.spec.contains("[unknown]"), "the gap row surfaces inline");
    assert!(documents.spec.contains("[divergence]"));
    assert!(documents.design.contains("The shape"));
}

#[tokio::test]
async fn unparseable_answer_fails() {
    let sets = journey_sets();
    let rows = reconcile(&sets);

    let model = Harness::answering(["Not a spec at all."]);
    let err = synthesise(&model, &sets, &rows)
        .await
        .expect_err("an answer outside the AST is refused (A17)");
    assert!(err.to_string().contains("spec-invalid"), "{err}");
}

#[tokio::test]
async fn hidden_row_fails() {
    let sets = journey_sets();
    let rows = reconcile(&sets);

    // A syntactically valid answer must not erase conflicts or gaps.
    let mut dishonest = rows.clone();
    for row in &mut dishonest {
        row.status = Status::Agreed;
        row.tag = None;
        if row.sources.is_empty() {
            row.sources = ["mock-docs"].map(str::to_string).into();
        }
    }
    let model = Harness::answering([spec_answer(&dishonest)]);
    let err = synthesise(&model, &sets, &rows)
        .await
        .expect_err("an answer hiding a conflict or gap is refused");
    assert!(err.to_string().contains("spec-provenance-mismatch"), "{err}");
}

#[tokio::test]
async fn renamed_heading_fails() {
    let sets = journey_sets();
    let rows = reconcile(&sets);

    // Subject headings are stable re-mine-diff identity.
    let mut retitled = rows.clone();
    retitled[0].subject = "login.journey".to_string();
    let model = Harness::answering([spec_answer(&retitled)]);
    let err = synthesise(&model, &sets, &rows)
        .await
        .expect_err("an answer rewriting a subject heading is refused");
    assert!(err.to_string().contains("spec-provenance-mismatch"), "{err}");
    assert!(err.to_string().contains("login.flow"), "{err}");
}
