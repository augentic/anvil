//! Extract over a read-only CID view: the terminal lead carries
//! parent/focus from the catalog, and the source dispatch never sees
//! the change home.

use std::path::Path;
use std::sync::Mutex;

use artifacts::evidence::AuthorityClass;
use artifacts::leads::{Lead as CatalogLead, Leads};
use mock::session::Session;
use project::adapter::catalog::Pin;
use project::config::Layout;
use project::handler::Anchor as _;
use project::plan::{Plan, SourceBinding, TargetBinding};
use project::seam::{self, Evidence, Source, SourceContent, SourceInput, SurveyResult};
use project::snapshot::SnapshotId;

struct Seen {
    input: SourceInput,
    has_login: bool,
    has_emery: bool,
    has_leads: bool,
}

struct Recording {
    seen: Mutex<Option<Seen>>,
}

impl Source for Recording {
    async fn survey(&self, _id: String, _input: SourceInput) -> Result<SurveyResult, seam::Error> {
        Err(seam::Error::Internal("survey unused".into()))
    }

    async fn extract(&self, _id: String, input: SourceInput) -> Result<Evidence, seam::Error> {
        let (has_login, has_emery, has_leads) = match &input.content {
            SourceContent::Workspace(view) => {
                let root = Path::new(&view.root);
                (
                    root.join("login.md").is_file(),
                    root.join(".emery").exists(),
                    root.join("leads.md").exists(),
                )
            }
            SourceContent::Value(_) => (false, false, false),
        };
        *self.seen.lock().expect("lock") = Some(Seen {
            input,
            has_login,
            has_emery,
            has_leads,
        });
        Ok(Evidence {
            authority: AuthorityClass::Documentation,
            claims: Vec::new(),
        })
    }
}

fn stub_target() -> TargetBinding {
    TargetBinding::new(
        Pin::parse("emery:mock@0.0.0").expect("pin"),
        ".",
        SnapshotId::from_digest(&"0".repeat(64)),
    )
}

#[tokio::test]
async fn view_carries_catalog_lead() {
    let session = Session::scripted("mock", Vec::new());
    let source_tree = session.root().parent().expect("temp home").join("docs-source");
    std::fs::create_dir_all(&source_tree).expect("source tree");
    std::fs::write(source_tree.join("login.md"), "Users sign in with email and password.\n")
        .expect("source file");

    let cid = session.store().snapshot(&source_tree).await.expect("snapshot source tree");
    let layout = Layout::new(session.root());
    std::fs::create_dir_all(layout.change_root()).expect("change home");

    let mut plan = Plan::named("demo");
    plan.targets.insert("default".into(), stub_target());
    plan.sources.insert(
        "docs".into(),
        SourceBinding::located(
            Pin::parse("emery:mock-docs@0.0.0").expect("pin"),
            source_tree.display().to_string(),
            cid,
        ),
    );
    plan.save(&layout.plan_path()).expect("plan.yaml");

    let mut lockout =
        CatalogLead::new("login-lockout", "docs", "Five failed attempts lock the account.");
    lockout.parent = Some("login-flow".into());
    lockout.focus = Some("login-flow".into());
    Leads::from_leads(vec![CatalogLead::new("login-flow", "docs", "Users sign in."), lockout])
        .write_atomic(&layout.leads_path())
        .expect("leads.md");

    let recording = Recording {
        seen: Mutex::new(None),
    };
    slice::orchestrate::extract(
        &recording,
        session.provider(),
        session.provider(),
        session.provider().paths(),
        jiff::Timestamp::now(),
        "docs",
        "login-lockout",
        "auth",
    )
    .await
    .expect("extract over view");

    let seen = recording.seen.lock().expect("lock").take().expect("extract dispatched");
    assert_eq!(seen.input.key, "docs");
    let focus = seen.input.focus.as_ref().expect("terminal lead");
    assert_eq!(focus.lead, "login-lockout");
    assert_eq!(focus.parent.as_deref(), Some("login-flow"));
    assert_eq!(focus.focus.as_deref(), Some("login-flow"));

    let SourceContent::Workspace(view) = &seen.input.content else {
        panic!("location-backed extract must pass a CID view, got {:?}", seen.input.content);
    };
    assert!(seen.has_login, "view carries the source tree at {}", view.root);
    assert!(!seen.has_emery, "view is not the project or change home");
    assert!(!seen.has_leads, "catalog is not on the view");
    assert_ne!(Path::new(&view.root), session.root());

    let evidence = layout.slice_dir("auth").join("evidence/docs.yaml");
    let body = std::fs::read_to_string(&evidence).expect("evidence written");
    assert!(body.contains("lead: login-lockout"), "{body}");
}

#[tokio::test]
async fn cid_missing() {
    let session = Session::scripted("mock", Vec::new());
    let layout = Layout::new(session.root());
    std::fs::create_dir_all(layout.change_root()).expect("change home");

    let mut plan = Plan::named("demo");
    plan.targets.insert("default".into(), stub_target());
    plan.sources.insert(
        "docs".into(),
        SourceBinding {
            adapter: Pin::parse("emery:mock-docs@0.0.0").expect("pin"),
            locator: Some("/tmp/docs".into()),
            value: None,
            cid: None,
        },
    );
    plan.save(&layout.plan_path()).expect("plan.yaml");
    Leads::from_leads(vec![CatalogLead::new("login-flow", "docs", "Users sign in.")])
        .write_atomic(&layout.leads_path())
        .expect("leads.md");

    let err = slice::orchestrate::extract(
        &Recording {
            seen: Mutex::new(None),
        },
        session.provider(),
        session.provider(),
        session.provider().paths(),
        jiff::Timestamp::now(),
        "docs",
        "login-flow",
        "auth",
    )
    .await
    .expect_err("missing cid");
    let detail = err.to_string();
    assert!(detail.contains("source-cid-missing"), "{detail}");
}
