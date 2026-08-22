//! Native capability rung: the in-process `init` → `specify` journey
//! over scripted `Model` + `Source` + storage capabilities — no built
//! component, no filesystem engine state.

#![cfg(not(target_arch = "wasm32"))]

#[path = "../crates/engine/tests/support/storage.rs"]
mod storage;

use std::fs;
use std::future::Future;
use std::sync::Arc;

use emery_adapter::seam::{
    Authority, Backing, Claim, ClaimKind, Evidence, SourceInput, SourceMetadata,
};
use emery_adapter::{DispatchError, Source};
use emery_transport::command;
use omnia_guest::Model;
use omnia_guest::api::command::CommandResponse;
use omnia_guest::api::invoke::Invoker;
use omnia_guest::model::{Error, Reply, Request};
use omnia_testkit::model::{Harness, Scripted};
use serde_json::{Map, Value};
use storage::Memory;

const SPEC_ANSWER: &str = include_str!("source/1-spec.md");
const DESIGN_ANSWER: &str = include_str!("source/2-design.md");

#[tokio::test]
async fn gen_spec() {
    // The stub component is an operator-supplied workspace file — the
    // one filesystem touchpoint left; engine state lives in the
    // scripted store, so no chdir and no `.emery/` tree.
    let workspace = tempfile::tempdir().expect("tempdir");
    let component = workspace.path().join("source.wasm");
    fs::write(&component, b"\0asm-stub").expect("stub wasm");

    let provider = Provider {
        model: Harness::answering([SPEC_ANSWER, DESIGN_ANSWER, SPEC_ANSWER, DESIGN_ANSWER]),
        storage: Arc::new(Memory::default()),
    };

    // 1. init the project
    cli_exec(&provider, &["emery", "init", component.to_str().expect("utf-8 path")]).await;
    let record = provider.storage.state("project.yaml").expect("project record committed");
    assert!(String::from_utf8_lossy(&record).contains("key: source"), "the binding is recorded");
    assert_eq!(
        provider.storage.object("adapters", "source.wasm").as_deref(),
        Some(b"\0asm-stub".as_slice()),
        "the component is mirrored into the cache container"
    );

    // 2. generate specification
    cli_exec(&provider, &["emery", "specify"]).await;
    let pointer = provider.storage.state("spec/current").expect("current");
    let id = String::from_utf8(pointer).expect("utf-8 pointer").trim().to_string();
    let spec =
        provider.storage.object("spec", &format!("generations/{id}/spec.md")).expect("spec.md");
    assert!(String::from_utf8_lossy(&spec).contains("[unknown]"));
    let design =
        provider.storage.object("spec", &format!("generations/{id}/design.md")).expect("design.md");
    assert!(!design.is_empty());

    // 3. rerun generation
    let resp = cli_exec(&provider, &["emery", "specify"]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("none (byte-stable)"), "{stdout}");

    provider.model.assert_exhausted();
}

async fn cli_exec(provider: &Provider, argv: &[&str]) -> CommandResponse {
    let router = command::router(Invoker::new("emery", provider.clone())).expect("command grammar");
    let resp = router.execute(argv.iter().copied()).await;
    assert_eq!(resp.exit, 0, "{}", String::from_utf8_lossy(&resp.stderr));
    resp
}

#[derive(Clone, Debug)]
struct Provider {
    model: Harness<Scripted>,
    storage: Arc<Memory>,
}

crate::scripted_storage!(Provider, storage);

impl Model for Provider {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        self.model.create(request).await
    }
}

impl Source for Provider {
    fn extract(
        &self, _id: &str, _input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, DispatchError>> + Send {
        let statement = "GET /greeting returns the static string 'hello'.";
        let mut extras = Map::new();
        extras.insert("statement".to_string(), Value::String(statement.to_string()));

        std::future::ready(Ok(Evidence {
            authority: Authority::Documentation,
            claims: vec![Claim {
                kind: ClaimKind::Requirement,
                id: Some("greeting.behaviour".to_string()),
                path: None,
                synopsis: Some("Greeting behaviour".to_string()),
                backing: Some(Backing::Payload(statement.to_string())),
                extras,
            }],
        }))
    }

    fn metadata(&self, _id: &str) -> SourceMetadata {
        SourceMetadata { emery_floor: None }
    }
}
