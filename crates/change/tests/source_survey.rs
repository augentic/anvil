//! Engine-owned focused survey: child merge, catalog revision retain,
//! and the unfocused debug-breakout that still does not bump the digest.

mod support;

use artifacts::leads::Leads;
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::plan::{Plan, retain_leads};
use project::snapshot::SnapshotId;

fn stamp_digest(root: &std::path::Path) -> SnapshotId {
    let layout = Layout::new(root);
    let digest = retain_leads(layout).expect("retain import catalog");
    let mut plan = Plan::load(&layout.plan_path()).expect("plan");
    plan.leads_digest = Some(digest.clone());
    plan.save(&layout.plan_path()).expect("stamp leads_digest");
    digest
}

#[tokio::test]
async fn focused_retains() {
    let session = Session::scripted("mock", Vec::new());
    support::write_plan_fixture(
        session.root(),
        "demo",
        &[("docs", "mock-docs", "the docs")],
        &[("auth", "docs", "login-flow")],
    );
    let old = stamp_digest(session.root());
    let layout = Layout::new(session.root());

    let body = run::<change::source::Survey, _, _>(
        session.provider(),
        change::source::SurveyInput {
            source: "docs".into(),
            plan: None,
            focus: Some("login-flow".into()),
        },
    )
    .await
    .expect("focused survey");
    assert_eq!(body.leads, vec!["login-lockout", "login-mfa"]);

    let catalog = Leads::load(&layout.leads_path()).expect("catalog");
    let lockout =
        catalog.leads().iter().find(|lead| lead.lead == "login-lockout").expect("child merged");
    assert_eq!(lockout.source, "docs");
    assert_eq!(lockout.parent.as_deref(), Some("login-flow"));
    assert_eq!(lockout.focus.as_deref(), Some("login-flow"));
    assert!(catalog.leads().iter().any(|lead| lead.lead == "login-flow"), "parent row stays");

    let plan = Plan::load(&layout.plan_path()).expect("plan");
    let next = plan.leads_digest.expect("focused survey stamps the new digest");
    assert_ne!(next, old);
    assert!(layout.leads_revision_path(&old).exists(), "prior revision is never overwritten");
    assert!(layout.leads_revision_path(&next).exists(), "new revision retained");
}

#[tokio::test]
async fn unfocused_skips_retain() {
    let session = Session::scripted("mock", Vec::new());
    support::write_plan_fixture(
        session.root(),
        "demo",
        &[("docs", "mock-docs", "the docs")],
        &[("auth", "docs", "login-flow")],
    );
    let old = stamp_digest(session.root());
    let layout = Layout::new(session.root());

    run::<change::source::Survey, _, _>(
        session.provider(),
        change::source::SurveyInput {
            source: "docs".into(),
            plan: None,
            focus: None,
        },
    )
    .await
    .expect("unfocused survey");

    let catalog = Leads::load(&layout.leads_path()).expect("catalog");
    let next = SnapshotId::from_digest(&catalog.digest_hex().expect("digest"));
    assert_ne!(next, old, "unfocused merge rewrites the live catalog");
    let plan = Plan::load(&layout.plan_path()).expect("plan");
    assert_eq!(plan.leads_digest.as_ref(), Some(&old), "debug survey does not bump the digest");
    assert!(
        !layout.leads_revision_path(&next).exists(),
        "unfocused survey does not retain the new revision"
    );
}

#[tokio::test]
async fn unknown_focus() {
    let session = Session::scripted("mock", Vec::new());
    support::write_plan_fixture(
        session.root(),
        "demo",
        &[("docs", "mock-docs", "the docs")],
        &[("auth", "docs", "login-flow")],
    );

    let err = run::<change::source::Survey, _, _>(
        session.provider(),
        change::source::SurveyInput {
            source: "docs".into(),
            plan: None,
            focus: Some("missing-parent".into()),
        },
    )
    .await
    .expect_err("unknown parent");
    let detail = err.to_string();
    assert!(detail.contains("leads-lead-unknown"), "{detail}");
    assert!(detail.contains("missing-parent"), "{detail}");
}
