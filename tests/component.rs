//! The component rung: the shipped engine component and the mock adapter
//! component under the real omnia runtime, over a scripted model backend
//! and in-memory storage hosts. This suite owns the boundary the native
//! scenarios cannot reach — the `wasi:cli/run` wrapper, the WIT lowering
//! on both sides of the `emery:adapter/source` seam, the real plugin
//! loader, the reference-tool closure over real wasi-model streams — and
//! nothing `tests/specify.rs` already asserts. The runtime inherits
//! stdout, so scenarios observe the exit status and the storage handles.

#![cfg(not(target_arch = "wasm32"))]

use std::fs;

use omnia::{ExitStatus, sha256_digest};
use serde_json::{Value, json};
use test_utils::{Backends, Deployment, MOCK_ADAPTER, ScriptedModel, scratch};

/// The engine component the root build script produced for this profile
/// (a raw component under `dev`; the rung runs under `make test`).
const ENGINE: &str = concat!(env!("OUT_DIR"), "/emery.cwasm");

const SPEC_ANSWER: &str = include_str!("specify/1-spec.md");
const DESIGN_ANSWER: &str = include_str!("specify/2-design.md");
const EXTRACT_PROMPT: &str = include_str!("../examples/adapter/prose/prompts/extract.md");

/// The greeting requirement the mock adapter is scripted to extract.
fn evidence() -> Value {
    json!({
        "authority": "documentation",
        "claims": [{
            "kind": "requirement",
            "id": "greeting.behaviour",
            "statement": "GET /greeting returns the static string 'hello'."
        }]
    })
}

/// One full `specify` script: extract, then the spec and design syntheses.
fn answers(binding: &str) -> [Value; 3] {
    let spec = SPEC_ANSWER.replace("Sources: [source]", &format!("Sources: [{binding}]"));
    [evidence(), Value::String(spec), Value::String(DESIGN_ANSWER.to_string())]
}

/// The current generation id, read back through the keyvalue handle.
async fn pointer(backends: &Backends) -> String {
    let pointer = backends.state("spec/current").await.expect("a committed generation");
    String::from_utf8(pointer).expect("utf-8 pointer").trim().to_string()
}

/// A committed generation document, read back through the blobstore handle.
async fn generation(backends: &Backends, id: &str, name: &str) -> String {
    let bytes = backends
        .blob("spec", &format!("generations/{id}/{name}"))
        .await
        .unwrap_or_else(|| panic!("{name} committed"));
    String::from_utf8(bytes).expect("utf-8 document")
}

// A bare adapter name dispatches to the statically declared guest: the
// engine's `wasi:cli/run` wrapper parses argv, `metadata` and `extract`
// cross the seam through the SDK's export macro, the evidence lowers
// back with its extras intact, synthesis runs, and the generation
// commits through the storage hosts.
#[tokio::test]
async fn mock_bare_name() {
    // --------------------------------------------------
    // Arrange: the mock component declared as `source:greeting`.
    // --------------------------------------------------
    let project = scratch();
    project.write("docs/greeting.md", "The greeting endpoint returns hello.");
    let backends = Backends::scripted(ScriptedModel::answering(answers("greeting"))).await;

    // --------------------------------------------------
    // Act.
    // --------------------------------------------------
    let status = test_utils::run(
        Deployment {
            engine: ENGINE,
            argv: &["specify", "greeting"],
            project: &project,
            guests: &[("source:greeting", MOCK_ADAPTER)],
        },
        backends.clone(),
    )
    .await
    .expect("deployment runs");

    // --------------------------------------------------
    // Observe: the exit, the model legs, and the committed generation.
    // --------------------------------------------------
    assert_eq!(status, ExitStatus::SUCCESS);
    backends.model.assert_exhausted();
    let requests = backends.model.requests();
    assert_eq!(requests.len(), 3, "extract, spec, design");
    assert!(
        requests[0]
            .system
            .as_deref()
            .is_some_and(|system| system.starts_with(EXTRACT_PROMPT.trim_end())),
        "the adapter's embedded prompt is the extract system prompt"
    );
    assert_eq!(requests[0].tools, ["list_docs", "read_doc"], "the reference tools are declared");
    assert!(
        requests[0].messages[0].contains("source key `greeting`"),
        "{:?}",
        requests[0].messages
    );

    let id = pointer(&backends).await;
    let spec = generation(&backends, &id, "spec.md").await;
    assert!(spec.contains("Sources: [greeting]"), "{spec}");
    assert!(spec.contains("GET /greeting returns the static string 'hello'."), "{spec}");
    assert!(!generation(&backends, &id, "design.md").await.is_empty());
}

// A local `.wasm` loads through the real path loader on every run: the
// unpinned load succeeds, a config pin that matches the file's bytes
// loads again, and a pin that disagrees refuses typed before any
// extraction — the loader's verify-before-validate, not a double's.
#[tokio::test]
async fn mock_path_load() {
    let project = scratch();
    let bytes = fs::read(MOCK_ADAPTER).expect("the built mock component");
    project.write("adapter.wasm", &bytes);

    // Unpinned: the binding key is the file stem.
    let backends = Backends::scripted(ScriptedModel::answering(answers("adapter"))).await;
    let status = test_utils::run(
        Deployment {
            engine: ENGINE,
            argv: &["specify", "./adapter.wasm"],
            project: &project,
            guests: &[],
        },
        backends.clone(),
    )
    .await
    .expect("deployment runs");
    assert_eq!(status, ExitStatus::SUCCESS, "an unpinned path load extracts");
    backends.model.assert_exhausted();

    // Pinned to the staged bytes: loads.
    let config = |digest: &str| {
        format!(
            "[[source]]\nname = \"greeting\"\nadapter = \"./adapter.wasm\"\ndigest = \"{digest}\"\n"
        )
    };
    project.write("emery.toml", config(&sha256_digest(&bytes)));
    let backends = Backends::scripted(ScriptedModel::answering(answers("greeting"))).await;
    let status = test_utils::run(
        Deployment {
            engine: ENGINE,
            argv: &["specify", "--config", "emery.toml"],
            project: &project,
            guests: &[],
        },
        backends.clone(),
    )
    .await
    .expect("deployment runs");
    assert_eq!(status, ExitStatus::SUCCESS, "a matching pin loads");
    backends.model.assert_exhausted();

    // Pinned to other bytes: refused (exit 1) before the model is reached.
    project.write("emery.toml", config(&format!("sha256:{}", "ab".repeat(32))));
    let backends = Backends::scripted(ScriptedModel::answering(answers("greeting"))).await;
    let status = test_utils::run(
        Deployment {
            engine: ENGINE,
            argv: &["specify", "--config", "emery.toml"],
            project: &project,
            guests: &[],
        },
        backends.clone(),
    )
    .await
    .expect("deployment runs");
    assert_eq!(status.code(), 1, "a disagreeing pin refuses typed");
    assert!(backends.model.requests().is_empty(), "no extraction after a refused load");
    assert!(backends.state("spec/current").await.is_none(), "nothing commits");
}

// A component on the path slot that does not export the seam is not an
// adapter: the loader refuses it typed (exit 1) and the model is never
// reached. The engine component itself is the fixture.
#[tokio::test]
async fn path_load_no_seam_refuses() {
    let project = scratch();
    project.write("adapter.wasm", fs::read(ENGINE).expect("the engine component"));
    let backends = Backends::scripted(ScriptedModel::answering(answers("adapter"))).await;

    let status = test_utils::run(
        Deployment {
            engine: ENGINE,
            argv: &["specify", "./adapter.wasm"],
            project: &project,
            guests: &[],
        },
        backends.clone(),
    )
    .await
    .expect("deployment runs");

    assert_eq!(status.code(), 1, "a seamless component refuses");
    assert!(backends.model.requests().is_empty(), "no extraction after a refused load");
}

// The reference tools are served in-process from the adapter's embedded
// corpus across the real wasi-model tool streams: a `read_doc` the
// backend drives during extract comes back with the embedded body.
#[tokio::test]
async fn read_doc_served_in_process() {
    let project = scratch();
    let model = ScriptedModel::answering(answers("greeting"))
        .calling(0, [("read_doc", r#"{"path":"prompts/extract.md"}"#)]);
    let backends = Backends::scripted(model).await;

    let status = test_utils::run(
        Deployment {
            engine: ENGINE,
            argv: &["specify", "greeting"],
            project: &project,
            guests: &[("source:greeting", MOCK_ADAPTER)],
        },
        backends.clone(),
    )
    .await
    .expect("deployment runs");

    assert_eq!(status, ExitStatus::SUCCESS);
    let exchanges = backends.model.exchanges();
    assert_eq!(exchanges.len(), 1, "one driven tool call");
    assert_eq!(exchanges[0].tool, "read_doc");
    let answer: Value =
        serde_json::from_str(exchanges[0].outcome.as_ref().expect("the tool answered"))
            .expect("a JSON answer");
    assert_eq!(answer["path"], "prompts/extract.md");
    assert_eq!(answer["body"], EXTRACT_PROMPT, "the embedded document body crosses the seam");
}
