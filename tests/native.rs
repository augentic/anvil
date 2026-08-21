//! Native capability rung: the in-process `init` → `specify` journey
//! over scripted `Model` + `SourceDispatch` capabilities — no built
//! component. The component seam stays covered by `tests/journey.rs`.

use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};

use emery_adapter::seam::{
    Authority, Backing, Claim, ClaimKind, Evidence, SourceInput, SourceMetadata,
};
use emery_adapter::{DispatchError, SourceDispatch};
use emery_transport::command;
use omnia_guest::api::invoke::Invoker;

// The journey's scripted answers, reused verbatim: the spec renders
// the reconciliation rows the canned evidence reconciles to.
const SPEC_ANSWER: &str = include_str!("journey-script-minimal/1-spec.md");
const DESIGN_ANSWER: &str = include_str!("journey-script-minimal/2-design.md");

// The capability doubles: canned Evidence over the source seam and
// the testkit FIFO script over the model. Clones share the script
// queue, so the handle kept by the test can assert exhaustion.
#[derive(Clone, Debug)]
struct Doubles {
    model: omnia_testkit::model::Harness<omnia_testkit::model::Scripted>,
}

impl Doubles {
    // One spec + design answer pair per expected `specify` run —
    // identical pairs, so the re-run is byte-stable by construction.
    fn scripted(runs: usize) -> Self {
        let answers = std::iter::repeat_n([SPEC_ANSWER, DESIGN_ANSWER], runs).flatten();
        Self {
            model: omnia_testkit::model::Harness::answering(answers),
        }
    }
}

impl omnia_guest::Model for Doubles {
    fn create(
        &self, request: omnia_guest::model::Request,
    ) -> impl Future<Output = Result<omnia_guest::model::Reply, omnia_guest::model::Error>> + Send
    {
        self.model.create(request)
    }
}

impl SourceDispatch for Doubles {
    fn extract(
        &self, _id: &str, _input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, DispatchError>> + Send {
        std::future::ready(Ok(evidence()))
    }

    fn metadata(&self, _id: &str) -> SourceMetadata {
        SourceMetadata { emery_floor: None }
    }
}

// The journey mock's minimal greeting profile: one requirement, no
// criterion — reconciliation appends the acceptance gap row.
fn evidence() -> Evidence {
    let statement = "GET /greeting returns the static string 'hello'.";
    let mut extras = serde_json::Map::new();
    extras.insert("statement".to_string(), serde_json::Value::String(statement.to_string()));
    Evidence {
        authority: Authority::Documentation,
        claims: vec![Claim {
            kind: ClaimKind::Requirement,
            id: Some("greeting.behaviour".to_string()),
            path: None,
            synopsis: Some("Greeting behaviour".to_string()),
            backing: Some(Backing::Payload(statement.to_string())),
            extras,
        }],
    }
}

fn argv(args: &[&str]) -> Vec<String> {
    args.iter().map(ToString::to_string).collect()
}

#[tokio::test]
async fn native_capability_journey() {
    // Paths are CWD-relative constants; nextest gives this test its
    // own process, so the chdir cannot leak into another test.
    let home = tempfile::tempdir().expect("tempdir");
    std::env::set_current_dir(home.path()).expect("chdir into the scratch project root");
    // `init` mirrors the component file byte-for-byte and resolves
    // metadata over the scripted seam; nothing parses the stub.
    fs::write("source.wasm", b"\0asm-stub").expect("stage the stub component");

    let doubles = Doubles::scripted(2);
    let router = command::router(Invoker::new("emery", doubles.clone())).expect("router");

    let init = command::execute(&router, argv(&["emery", "init", "source.wasm"])).await;
    assert_eq!(init.exit, 0, "{}", String::from_utf8_lossy(&init.stderr));
    assert!(Path::new(".emery/project.yaml").is_file(), "init scaffolds the project");
    assert!(
        Path::new(".emery-cache/components/source.wasm").is_file(),
        "the local component mirrors into the CWD-relative project cache"
    );

    let specify = command::execute(&router, argv(&["emery", "specify"])).await;
    assert_eq!(specify.exit, 0, "{}", String::from_utf8_lossy(&specify.stderr));
    let spec = read_current("spec.md");
    assert!(
        spec.contains("[unknown]"),
        "gaps are preserved as `[unknown]`, never guessed:\n{spec}"
    );
    assert!(!read_current("design.md").is_empty(), "design.md carries the rebuild design");

    // A re-run over identical scripted capabilities is byte-stable
    // and reports the explicit empty re-mine diff (ADR-0010).
    let rerun = command::execute(&router, argv(&["emery", "specify"])).await;
    assert_eq!(rerun.exit, 0, "{}", String::from_utf8_lossy(&rerun.stderr));
    let stdout = String::from_utf8_lossy(&rerun.stdout);
    assert!(stdout.contains("none (byte-stable)"), "{stdout}");
    doubles.model.assert_exhausted();
}

fn read_current(name: &str) -> String {
    let path = find(Path::new(".emery/spec"), name)
        .unwrap_or_else(|| panic!("`{name}` must exist in the output home"));
    fs::read_to_string(path).expect("read spec artifact")
}

fn find(dir: &Path, suffix: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            if let Some(found) = find(&path, suffix) {
                return Some(found);
            }
        } else if path.ends_with(suffix) {
            return Some(path);
        }
    }
    None
}
