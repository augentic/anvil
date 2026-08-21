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
use omnia_guest::model::{Error, Reply, Request};
use omnia_testkit::model::{Harness, Scripted};
use serde_json::{Map, Value};

const SPEC_ANSWER: &str = include_str!("source/1-spec.md");
const DESIGN_ANSWER: &str = include_str!("source/2-design.md");

#[tokio::test]
async fn gen_spec() {
    // create a tempdir for the project
    let home = tempfile::tempdir().expect("tempdir");
    std::env::set_current_dir(home.path()).expect("should change directory");
    fs::write("source.wasm", b"\0asm-stub").expect("should fake source.wasm");

    let provider = Provider {
        model: Harness::answering(std::iter::repeat_n([SPEC_ANSWER, DESIGN_ANSWER], 2).flatten()),
    };

    // 1. init the project
    let argv = ["emery", "init", "source.wasm"].map(str::to_string).into();
    let init = command::execute(provider.clone(), argv).await;

    assert_eq!(init.exit, 0, "{}", String::from_utf8_lossy(&init.stderr));
    assert!(Path::new(".emery/project.yaml").is_file());
    assert!(Path::new(".emery-cache/components/source.wasm").is_file());

    // 2. generate specification
    let resp =
        command::execute(provider.clone(), ["emery", "specify"].map(str::to_string).into()).await;
    assert_eq!(resp.exit, 0, "{}", String::from_utf8_lossy(&resp.stderr));

    // 3. check the spec was generated
    let gen_dir = format!(
        ".emery/spec/generations/{}",
        fs::read_to_string(".emery/spec/current").expect("current").trim()
    );
    let spec = fs::read_to_string(format!("{gen_dir}/spec.md")).expect("spec.md");
    assert!(spec.contains("[unknown]"));
    assert!(!fs::read_to_string(format!("{gen_dir}/design.md")).expect("design.md").is_empty());

    // 4. rerun generation
    let resp =
        command::execute(provider.clone(), ["emery", "specify"].map(str::to_string).into()).await;
    assert_eq!(resp.exit, 0, "{}", String::from_utf8_lossy(&resp.stderr));

    // 5. check the spec was not changed
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("none (byte-stable)"), "{stdout}");

    provider.model.assert_exhausted();
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
