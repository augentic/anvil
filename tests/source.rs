//! Native capability rung: the in-process `init` → `specify` journey
//! over scripted `Model` + `SourceDispatch` capabilities — no built
//! component.

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::Path;

use emery_adapter::seam::{
    Authority, Backing, Claim, ClaimKind, Evidence, SourceInput, SourceMetadata,
};
use emery_adapter::{DispatchError, SourceDispatch};
use emery_transport::command;
use omnia_guest::Model;
use omnia_guest::api::command::CommandResponse;
use omnia_guest::model::{Error, Reply, Request};
use omnia_testkit::model::{Harness, Scripted};
use serde_json::{Map, Value};

const SPEC_ANSWER: &str = include_str!("source/1-spec.md");
const DESIGN_ANSWER: &str = include_str!("source/2-design.md");

#[tokio::test]
async fn gen_spec() {
    let project = tempfile::tempdir().expect("tempdir");
    std::env::set_current_dir(project.path()).expect("chdir into the scratch project root");
    fs::write("source.wasm", b"\0asm-stub").expect("stub wasm");

    let provider = Provider {
        model: Harness::answering([SPEC_ANSWER, DESIGN_ANSWER, SPEC_ANSWER, DESIGN_ANSWER]),
    };

    // 1. init the project
    cli_exec(&provider, &["emery", "init", "source.wasm"]).await;
    assert!(Path::new(".emery/project.yaml").is_file());
    assert!(Path::new(".emery-cache/components/source.wasm").is_file());

    // 2. generate specification
    cli_exec(&provider, &["emery", "specify"]).await;
    let pointer = fs::read_to_string(".emery/spec/current").expect("current");
    let generation = Path::new(".emery/spec/generations").join(pointer.trim());
    let spec = fs::read_to_string(generation.join("spec.md")).expect("spec.md");
    assert!(spec.contains("[unknown]"));
    assert!(!fs::read_to_string(generation.join("design.md")).expect("design.md").is_empty());

    // 3. rerun generation
    let resp = cli_exec(&provider, &["emery", "specify"]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("none (byte-stable)"), "{stdout}");

    provider.model.assert_exhausted();
}

async fn cli_exec(provider: &Provider, argv: &[&str]) -> CommandResponse {
    let args = argv.iter().copied().map(str::to_string).collect();
    let resp = command::execute(provider.clone(), args).await;
    assert_eq!(resp.exit, 0, "{}", String::from_utf8_lossy(&resp.stderr));
    resp
}

#[derive(Clone, Debug)]
struct Provider {
    model: Harness<Scripted>,
}

impl Model for Provider {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        self.model.create(request).await
    }
}

impl SourceDispatch for Provider {
    async fn extract(&self, _id: &str, _input: &SourceInput) -> Result<Evidence, DispatchError> {
        let statement = "GET /greeting returns the static string 'hello'.";
        let mut extras = Map::new();
        extras.insert("statement".to_string(), Value::String(statement.to_string()));

        Ok(Evidence {
            authority: Authority::Documentation,
            claims: vec![Claim {
                kind: ClaimKind::Requirement,
                id: Some("greeting.behaviour".to_string()),
                path: None,
                synopsis: Some("Greeting behaviour".to_string()),
                backing: Some(Backing::Payload(statement.to_string())),
                extras,
            }],
        })
    }

    fn metadata(&self, _id: &str) -> SourceMetadata {
        SourceMetadata { emery_floor: None }
    }
}
