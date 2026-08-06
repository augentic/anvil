//! Refine-time `base.yaml` pin assembly (RFC-86 D4 / D25).

mod support;

use std::fs;

use change::plan;
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::plan::{Plan, dir_cid, empty_cid, value_cid};
use slice::Base;

async fn author(session: &Session) {
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "auth".to_string(),
            sources: support::adversarial_bindings(),
            intent: None,
            force: false,
        },
    )
    .await
    .expect("author walks to pending");
}

#[tokio::test]
async fn refine_writes_base_yaml() {
    let session = Session::scripted(
        "mock",
        vec![mock::answers::adversarial_grouping(), mock::answers::login_flow_synthesis()],
    );
    author(&session).await;

    run::<slice::handlers::Refine, _, _>(
        session.provider(),
        slice::handlers::RefineInput {
            name: "login-flow".to_string(),
        },
    )
    .await
    .expect("refine synthesises login-flow");

    let root = session.root();
    let layout = Layout::new(root);
    let plan = Plan::load(&layout.plan_path()).expect("plan loads");
    let base = Base::load(&layout.slice_dir("login-flow")).expect("base.yaml loads");

    assert_eq!(
        base.sources.get("docs"),
        plan.sources["docs"].cid.as_ref(),
        "docs cid copied from plan pin"
    );
    assert_eq!(
        base.sources.get("code"),
        plan.sources["code"].cid.as_ref(),
        "code cid copied from plan pin"
    );
    assert_eq!(base.sources.len(), 2, "only bound sources are pinned");
    assert_eq!(
        base.sources["docs"],
        value_cid("The docs source."),
        "docs value pin matches author close"
    );
    assert_eq!(
        base.sources["code"],
        value_cid("The code source."),
        "code value pin matches author close"
    );

    // Greenfield: no baseline specs yet → empty-tree baseline-spec pin.
    assert_eq!(base.baseline_spec, empty_cid());
    assert_eq!(base.baseline_spec, dir_cid(&layout.specs_dir()).expect("dir cid"));

    // Target-base pin is a real store snapshot (refine freezes the
    // product tree); it must be present and distinct from the empty
    // baseline-spec identity on a non-empty project tree.
    assert!(
        base.target_base.as_str().starts_with("sha256:"),
        "target-base wire form: {}",
        base.target_base
    );

    let yaml =
        fs::read_to_string(Base::path(&layout.slice_dir("login-flow"))).expect("read base.yaml");
    assert!(yaml.contains("baseline-spec:"), "{yaml}");
    assert!(yaml.contains("target-base:"), "{yaml}");
    assert!(yaml.contains("sources:"), "{yaml}");
    assert!(yaml.contains("docs:"), "{yaml}");
    assert!(yaml.contains("code:"), "{yaml}");
}
