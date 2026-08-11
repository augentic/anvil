//! Integration coverage for `emery plan gaps` (RFC-86 S16).

mod support;

use change::plan::handlers::{Gaps as GapsOp, GapsInput};
use change::{GapsBody, Plan, SliceSourceBinding};
use mock::invoke::run;
use mock::session::Session;
use support::{change, plan_with_changes};

fn write_plan(project: &Session, plan: &Plan) {
    let yaml = serde_saphyr::to_string(plan).expect("serialize plan");
    std::fs::write(project.root().join("plan.yaml"), yaml).expect("write plan.yaml");
}

fn write_slice_model(root: &std::path::Path, name: &str, model: &str) {
    let slice_dir = root.join(".emery").join("slices").join(name);
    std::fs::create_dir_all(&slice_dir).expect("slice dir");
    std::fs::write(slice_dir.join("metadata.yaml"), "target: demo-target@1.0.0\n")
        .expect("metadata");
    std::fs::write(slice_dir.join("model.yaml"), model).expect("model");
}

#[tokio::test]
async fn gaps_operation_projects_multi_homed_lead() {
    let project = Session::scripted("demo", Vec::new());
    let mut auth = change("auth-login");
    auth.sources = vec![SliceSourceBinding::structured("docs", "conventions")];
    let mut payments = change("payments");
    payments.sources = vec![SliceSourceBinding::structured("docs", "conventions")];
    let plan = plan_with_changes(vec![auth, payments]);
    write_plan(&project, &plan);

    write_slice_model(
        project.root(),
        "auth-login",
        r"requirements:
  - id: REQ-003
    title: password-reset path not evidenced
    statement: ''
    status: unknown
    sources: [docs]
",
    );
    write_slice_model(
        project.root(),
        "payments",
        r"requirements:
  - id: REQ-008
    title: reset copy not evidenced
    statement: ''
    status: unknown
    sources: [docs]
",
    );

    let body: GapsBody =
        run::<GapsOp, _, _>(project.provider(), GapsInput {}).await.expect("plan gaps");
    assert_eq!(body.rows.len(), 2);
    assert_eq!(body.rows[0].shared_lead.as_deref(), Some("docs:conventions"));
    assert_eq!(body.rows[1].shared_lead.as_deref(), Some("docs:conventions"));
    assert_eq!(body.rollups.len(), 1);
    assert_eq!(body.rollups[0].selectors, ["auth-login", "payments"]);
}
