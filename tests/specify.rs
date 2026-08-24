//! The `emery specify` → `emery show` product arc over scripted
//! capabilities: how an operator binds sources, generates the spec
//! set, reviews it, and regenerates it — and how every refusal on
//! that path fails typed.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::fs;
use std::path::Path;
use std::sync::Arc;

use emery_adapter::types::{Authority, ClaimKind, SourceContent};
use emery_adapter::{DispatchError, types};
use emery_testkit::{Memory, Namespaced};
use serde_json::Value;
use support::{Provider, claim, cli, cli_ok, evidence, requirement};

const SPEC_ANSWER: &str = include_str!("specify/1-spec.md");
const DESIGN_ANSWER: &str = include_str!("specify/2-design.md");
const PRECEDENCE_ANSWER: &str = include_str!("specify/3-precedence.md");
const SOURCES: &str = include_str!("specify/sources.toml");

fn project_tempdir() -> tempfile::TempDir {
    tempfile::TempDir::new_in(env!("CARGO_MANIFEST_DIR")).expect("project tempdir")
}

fn project_arg(path: &Path) -> String {
    path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .expect("path under project")
        .to_str()
        .expect("utf-8 path")
        .to_string()
}

// One `specify` ensures, mirrors, extracts, and commits — no prior
// verb; `show` renders the committed bytes alone; an identical re-run
// is byte-stable and says so.
#[tokio::test]
async fn gen_spec() {
    // --------------------------------------------------
    // Arrange: only the operator-supplied component touches the
    // filesystem; engine state stays in scripted storage.
    // --------------------------------------------------
    let workspace = project_tempdir();
    let component = workspace.path().join("source.wasm");
    fs::write(&component, b"\0asm-stub").expect("stub wasm");
    let component = project_arg(&component);

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER, SPEC_ANSWER, DESIGN_ANSWER]);

    // --------------------------------------------------
    // Act: the first specify.
    // --------------------------------------------------
    cli_ok(&provider, &["emery", "specify", &component]).await;

    // --------------------------------------------------
    // Observe: the mirror, the pointer, and the generation.
    // --------------------------------------------------
    assert_eq!(
        provider.storage.object("adapters", "source.wasm").as_deref(),
        Some(b"\0asm-stub".as_slice()),
        "the component is mirrored into the cache container"
    );
    assert!(provider.storage.state("project.yaml").is_none(), "no project record exists");
    let id = pointer(&provider.storage);
    let spec = generation(&provider.storage, &id, "spec.md");
    assert!(String::from_utf8_lossy(&spec).contains("[unknown]"));
    let design = generation(&provider.storage, &id, "design.md");
    assert!(!design.is_empty());

    // Review is `show`: text stdout is the stored document, byte for byte.
    let shown = cli_ok(&provider, &["emery", "show", "spec"]).await;
    assert_eq!(shown.stdout, spec, "show renders the committed spec.md alone");
    let shown = cli_ok(&provider, &["emery", "show", "design"]).await;
    assert_eq!(shown.stdout, design, "show renders the committed design.md alone");

    // An identical re-run reports the empty diff.
    let resp = cli_ok(&provider, &["emery", "specify", &component]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("none (byte-stable)"), "{stdout}");

    provider.model.assert_exhausted();
}

// `--sources` is the other specify authority: table keys become binding
// keys, and a local adapter resolves relative to the file.
#[tokio::test]
async fn from_file() {
    let workspace = project_tempdir();
    fs::write(workspace.path().join("source.wasm"), b"\0asm-stub").expect("stub wasm");
    let sources = workspace.path().join("sources.toml");
    fs::write(&sources, SOURCES).expect("write sources.toml");
    let sources = project_arg(&sources);

    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [greeting]");
    let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);

    let resp = cli_ok(&provider, &["emery", "specify", "--sources", &sources]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("sources: 1"), "{stdout}");
    assert_eq!(
        provider.storage.object("adapters", "source.wasm").as_deref(),
        Some(b"\0asm-stub".as_slice()),
        "the file-relative component is mirrored"
    );

    let id = pointer(&provider.storage);
    let spec = generation(&provider.storage, &id, "spec.md");
    assert!(
        String::from_utf8_lossy(&spec).contains("Sources: [greeting]"),
        "the table key is the binding key"
    );
    let shown = cli_ok(&provider, &["emery", "show", "spec"]).await;
    assert_eq!(shown.stdout, spec, "show renders the committed spec.md alone");

    provider.model.assert_exhausted();
}

// `--value` binds inline text under the adapter's name: no filesystem
// lend reaches extract, and a bare adapter needs no local component.
#[tokio::test]
async fn value_binding() {
    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [intent]");
    let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", "--value", "intent=Ship it."]).await;

    let calls = provider.source.calls.lock().expect("calls");
    let (id, input) = calls.first().expect("one extract dispatch");
    assert_eq!(id, "source:intent", "a bare adapter dispatches by routed name");
    assert_eq!(input.key, "intent");
    assert_eq!(input.content, SourceContent::Value("Ship it.".to_string()));
    drop(calls);

    let id = pointer(&provider.storage);
    let spec = generation(&provider.storage, &id, "spec.md");
    assert!(String::from_utf8_lossy(&spec).contains("Sources: [intent]"));
    provider.model.assert_exhausted();
}

// Authority reconciles disagreement: an intent directive outranks the
// docs, tied documentation peers surface as [conflict] for the
// operator, and the uncovered acceptance gap stays [unknown] — the
// committed spec carries every resolution inline.
#[tokio::test]
async fn authority_precedence() {
    let mut provider = Provider::answering([PRECEDENCE_ANSWER, DESIGN_ANSWER]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(
            Authority::Documentation,
            vec![
                requirement("login.flow", "Users sign in with a magic link."),
                requirement("session.timeout", "Sessions expire after 30 minutes of inactivity."),
                claim(
                    ClaimKind::Criterion,
                    "login.flow.success",
                    ("criterion", "A valid link signs the user in."),
                ),
                // Non-requirement kinds ride along as synthesis context.
                claim(ClaimKind::Decision, "auth.decision", ("body", "Sessions are cookie-bound.")),
            ],
        )),
    );
    provider.source.evidence.insert(
        "wiki-live".to_string(),
        Ok(evidence(
            Authority::Documentation,
            vec![requirement("login.flow", "Users sign in with a passkey.")],
        )),
    );
    provider.source.evidence.insert(
        "code".to_string(),
        Ok(evidence(
            Authority::Behaviour,
            vec![
                requirement("login.flow", "Users sign in with email and password."),
                requirement("session.timeout", "Sessions expire after 15 minutes of inactivity."),
            ],
        )),
    );
    provider.source.evidence.insert(
        "intent".to_string(),
        Ok(evidence(
            Authority::Intent,
            vec![requirement(
                "session.timeout",
                "Sessions must expire after 30 minutes of inactivity.",
            )],
        )),
    );

    let resp = cli_ok(
        &provider,
        &[
            "emery",
            "specify",
            "docs",
            "wiki-live",
            "code",
            "--value",
            "intent=Sessions expire after 30.",
        ],
    )
    .await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("requirements: 3"), "{stdout}");
    assert!(stdout.contains("sources: 4"), "{stdout}");

    let id = pointer(&provider.storage);
    let spec = String::from_utf8(generation(&provider.storage, &id, "spec.md")).expect("utf-8");
    assert!(spec.contains("login.flow [conflict]"), "tied docs peers conflict: {spec}");
    assert!(spec.contains("session.timeout [divergence]"), "intent outranks docs: {spec}");
    assert!(spec.contains("Sources: [intent, docs, code]"), "the intent directive wins: {spec}");
    assert!(spec.contains("[unknown]"), "the acceptance gap is preserved: {spec}");
    provider.model.assert_exhausted();
}

// A re-run over changed evidence supersedes the generation: the old
// blobs are pruned, the pointer swaps, and the success envelope
// reports the re-mine diff by heading subject — added, removed, and
// changed sections alike.
#[tokio::test]
async fn remine_supersedes() {
    // --------------------------------------------------
    // First run: the docs describe a greeting and a legacy export.
    // --------------------------------------------------
    let mut provider = Provider::answering([REMINE_FIRST, DESIGN_ANSWER]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(docs_evidence("hello", "legacy.export", "Exports ship nightly.")),
    );
    cli_ok(&provider, &["emery", "specify", "docs"]).await;
    let first = pointer(&provider.storage);

    // --------------------------------------------------
    // Second run: the greeting changed, the export is gone, and an
    // audit requirement appeared.
    // --------------------------------------------------
    let mut provider =
        Provider::over(Arc::clone(&provider.storage), [REMINE_SECOND, DESIGN_ANSWER]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(docs_evidence("howdy", "access.audit", "Access is audited.")),
    );
    let resp = cli_ok(&provider, &["emery", "specify", "docs"]).await;

    // --------------------------------------------------
    // Observe: the diff, the swap, and the prune.
    // --------------------------------------------------
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains(&format!("diff vs {first}: spec.md")), "{stdout}");
    assert!(stdout.contains("+ access.audit"), "{stdout}");
    assert!(stdout.contains("- legacy.export"), "{stdout}");
    assert!(stdout.contains("~ greeting.behaviour"), "{stdout}");

    let second = pointer(&provider.storage);
    assert_ne!(first, second, "changed documents commit a new generation");
    assert!(
        provider.storage.object("spec", &format!("generations/{first}/spec.md")).is_none(),
        "the superseded generation is pruned"
    );
    let spec = generation(&provider.storage, &second, "spec.md");
    assert!(String::from_utf8_lossy(&spec).contains("howdy"));
    provider.model.assert_exhausted();
}

// Two-requirement docs evidence with criteria covering both subjects.
fn docs_evidence(greeting: &str, subject: &str, statement: &str) -> types::Evidence {
    evidence(
        Authority::Documentation,
        vec![
            requirement(
                "greeting.behaviour",
                &format!("GET /greeting returns the static string '{greeting}'."),
            ),
            requirement(subject, statement),
            claim(
                ClaimKind::Criterion,
                "greeting.behaviour.check",
                ("criterion", "The greeting body matches exactly."),
            ),
            claim(
                ClaimKind::Criterion,
                &format!("{subject}.check"),
                ("criterion", "The behaviour is observable."),
            ),
        ],
    )
}

const REMINE_FIRST: &str = "# Specification

### Requirement: greeting.behaviour

ID: REQ-001
Sources: [docs]
Status: agreed

GET /greeting returns the static string 'hello'.

### Requirement: legacy.export

ID: REQ-002
Sources: [docs]
Status: agreed

Exports ship nightly.
";

const REMINE_SECOND: &str = "# Specification

### Requirement: greeting.behaviour

ID: REQ-001
Sources: [docs]
Status: agreed

GET /greeting returns the static string 'howdy'.

### Requirement: access.audit

ID: REQ-002
Sources: [docs]
Status: agreed

Access is audited.
";

// A requirement claim missing its `statement` extra fails the whole
// run typed (A8 fail-closed) before anything commits.
#[tokio::test]
async fn extras_missing() {
    let mut provider = Provider::idle();
    let mut bare = requirement("greeting.behaviour", "");
    bare.extras.clear();
    provider
        .source
        .evidence
        .insert("docs".to_string(), Ok(evidence(Authority::Documentation, vec![bare])));

    fail(&provider, &["emery", "specify", "docs"], 2, "claim-extras-missing").await;
    assert!(provider.storage.state("spec/current").is_none(), "a refused run commits nothing");
}

// An adapter call failure surfaces as one typed error.
#[tokio::test]
async fn extract_fails() {
    let mut provider = Provider::idle();
    provider.source.evidence.insert(
        "docs".to_string(),
        Err(DispatchError::Call(types::Error::Internal("the adapter exploded".to_string()))),
    );

    fail(&provider, &["emery", "specify", "docs"], 1, "source-extract-failed").await;
    assert!(provider.storage.state("spec/current").is_none(), "a refused run commits nothing");
}

// An adapter declaring a newer `emery` floor than the binary refuses
// with the dedicated version exit code.
#[tokio::test]
async fn floor_too_new() {
    let mut provider = Provider::idle();
    provider.source.floors.insert("docs".to_string(), "99.0.0".to_string());

    fail(&provider, &["emery", "specify", "docs"], 3, "adapter-cli-too-old").await;
}

// A model answer outside the fail-closed spec AST is refused, one
// grammar breach per case: no blocks, malformed metadata, a bare
// block, and duplicated requirement ids.
#[tokio::test]
async fn unparseable_answer() {
    let answers = [
        "Not a spec at all.",
        "# S\n\n### Requirement: greeting [wat]\n\nID: REQ-1\nSources: [Docs!]\nStatus: maybe\n\nBody.\n",
        "# S\n\n### Requirement: greeting\n\nBody without any metadata.\n",
        "# S\n\n### Requirement: [unknown]\n\nID: REQ-001\nSources: []\nStatus: unknown\n\nNo name.\n",
        "# S\n\n### Requirement: a\n\nID: REQ-001\nSources: [docs]\nStatus: agreed\n\nA.\n\n\
         ### Requirement: b\n\nID: REQ-001\nSources: [docs]\nStatus: agreed\n\nB.\n",
    ];
    for answer in answers {
        let provider = Provider::answering([answer]);
        fail(&provider, &["emery", "specify", "docs"], 2, "spec-invalid").await;
        assert!(provider.storage.state("spec/current").is_none(), "a refused run commits nothing");
    }
}

// A syntactically valid answer must not drop, rename, retag, recite,
// or renumber reconciliation rows: every dishonest rendering is a
// provenance mismatch.
#[tokio::test]
async fn dishonest_answer() {
    // The honest rows: `REQ-001 greeting.behaviour` agreed over
    // `[docs]`, then the `REQ-002` acceptance gap.
    let answers = [
        // The gap row is hidden.
        "# S\n\n### Requirement: greeting.behaviour\n\nID: REQ-001\nSources: [docs]\nStatus: agreed\n\nHello.\n".to_string(),
        // The subject heading is renamed.
        two_blocks("greeting.renamed", "REQ-001", "[docs]", "agreed"),
        // The status is retagged.
        two_blocks("greeting.behaviour [divergence]", "REQ-001", "[docs]", "divergence"),
        // The sources are recited.
        two_blocks("greeting.behaviour", "REQ-001", "[other]", "agreed"),
        // The requirement ids are renumbered.
        two_blocks("greeting.behaviour", "REQ-009", "[docs]", "agreed"),
    ];
    for answer in answers {
        let provider = Provider::answering([answer.as_str()]);
        fail(&provider, &["emery", "specify", "docs"], 2, "spec-provenance-mismatch").await;
        assert!(provider.storage.state("spec/current").is_none(), "a refused run commits nothing");
    }
}

// A parseable two-block answer with one dishonest first block.
fn two_blocks(heading: &str, id: &str, sources: &str, status: &str) -> String {
    format!(
        "# Specification\n\n### Requirement: {heading}\n\nID: {id}\nSources: {sources}\n\
         Status: {status}\n\nGET /greeting returns the static string 'hello'.\n\n\
         ### Requirement: greeting.behaviour acceptance criteria [unknown]\n\n\
         ID: REQ-002\nSources: []\nStatus: unknown\n\nNo source contributed a criterion.\n"
    )
}

// The design leg is fail-closed too: an empty second answer refuses.
#[tokio::test]
async fn empty_design() {
    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [docs]");
    let provider = Provider::answering([spec_answer.as_str(), "   "]);
    fail(&provider, &["emery", "specify", "docs"], 2, "design-empty").await;
    assert!(provider.storage.state("spec/current").is_none(), "a refused run commits nothing");
}

// A model transport failure surfaces as one typed synthesis error.
#[tokio::test]
async fn model_fails() {
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "docs"], 1, "synthesis-model-failed").await;
}

// The operator-owned `sources.toml` parses fail-closed: every
// malformed carrier refuses typed before anything commits.
#[tokio::test]
async fn sources_file_refused() {
    let cases: &[(&str, u8, &str)] = &[
        ("not toml [", 1, "sources-toml-malformed"),
        (
            "[sources.docs]\nadapter = \"documentation\"\nbranch = \"main\"\n",
            1,
            "sources-toml-malformed",
        ),
        ("", 2, "specify-source-required"),
        (
            "[sources.docs]\nadapter = \"documentation\"\npath = \"docs\"\nvalue = \"text\"\n",
            2,
            "argument",
        ),
        (
            "[sources.upstream]\nadapter = \"documentation\"\ngit = \"https://github.com/acme/api@v2\"\n",
            1,
            "source-remote-unsupported",
        ),
        (
            "[sources.upstream]\nadapter = \"documentation\"\nurl = \"https://example.com/openapi.yaml\"\n",
            1,
            "source-remote-unsupported",
        ),
        (
            "[sources.upstream]\nadapter = \"documentation\"\ngit = \"git+https://github.com/acme/api#deadbeef\"\n",
            2,
            "argument",
        ),
        ("[sources.docs]\nadapter = \"documentation\"\npath = \"../../outside\"\n", 2, "argument"),
        ("[sources.local]\nadapter = \"/tmp/source.wasm\"\n", 2, "argument"),
    ];
    for (body, exit, code) in cases {
        let dir = project_tempdir();
        let path = dir.path().join("sources.toml");
        fs::write(&path, body).expect("write sources.toml");
        let path = project_arg(&path);
        let provider = Provider::idle();
        fail(&provider, &["emery", "specify", "--sources", &path], *exit, code).await;
        assert!(provider.storage.is_empty(), "a refused run writes nothing: {code}");
    }

    // An unreadable file is a typed filesystem error.
    let provider = Provider::idle();
    fail(
        &provider,
        &["emery", "specify", "--sources", "nonexistent/sources.toml"],
        1,
        "filesystem-read",
    )
    .await;

    // Host-absolute and escaping paths never cross into the guest namespace.
    for path in ["/nonexistent/sources.toml", "../sources.toml"] {
        fail(&provider, &["emery", "specify", "--sources", path], 2, "argument").await;
    }
}

// File-relative `path` entries anchor at the file's directory, fold
// `.` and `..` lexically, and stay `.`-relative so the guest preopen
// can open them; `value` entries lend nothing — all observed on the
// `SourceInput` the adapter receives.
#[tokio::test]
async fn binding_paths() {
    let dir = project_tempdir();
    let path = dir.path().join("sources.toml");
    fs::write(
        &path,
        "[sources.docs]\nadapter = \"documentation\"\npath = \"nested/../docs\"\n\n\
         [sources.intent]\nadapter = \"intent\"\nvalue = \"Ship it.\"\n\n\
         [sources.local]\nadapter = \"local\"\npath = \"./docs\"\n",
    )
    .expect("write sources.toml");

    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [docs, intent, local]");
    let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);
    let path = project_arg(&path);
    cli_ok(&provider, &["emery", "specify", "--sources", &path]).await;

    let calls = provider.source.calls.lock().expect("calls");
    for key in ["docs", "local"] {
        let (_, input) = calls.iter().find(|(_, input)| input.key == key).expect("dispatched");
        let SourceContent::Workspace(workspace) = &input.content else {
            panic!("a path binding lends a workspace");
        };
        assert!(
            !Path::new(&workspace.root).is_absolute(),
            "the lend must stay `.`-relative for the guest preopen: {}",
            workspace.root
        );
        assert!(
            workspace.root.ends_with("docs") && !workspace.root.contains(".."),
            "`.` and `..` fold away lexically against the file's directory: {}",
            workspace.root
        );
    }
    let (_, input) = calls.iter().find(|(_, input)| input.key == "intent").expect("dispatched");
    assert_eq!(
        input.content,
        SourceContent::Value("Ship it.".to_string()),
        "a file `value` entry binds inline text"
    );
    drop(calls);
    provider.model.assert_exhausted();
}

// The mirrored component keeps a recorded selector resolvable after
// the operator deletes the source file.
#[tokio::test]
async fn mirror_survives_removal() {
    let workspace = project_tempdir();
    let component = workspace.path().join("source.wasm");
    fs::write(&component, b"\0asm-stub").expect("stub wasm");
    let component = project_arg(&component);

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER, SPEC_ANSWER, DESIGN_ANSWER]);
    cli_ok(&provider, &["emery", "specify", &component]).await;

    fs::remove_file(&component).expect("remove the operator's file");
    cli_ok(&provider, &["emery", "specify", &component]).await;

    assert!(
        provider.storage.object("adapters", "source.wasm").is_some(),
        "the mirror serves the recorded selector"
    );
    provider.model.assert_exhausted();
}

// Mirror recovery distinguishes an unavailable cache from a confirmed
// absent mirror and therefore never falls through to a path error.
#[tokio::test]
async fn mirror_probe_fault() {
    let provider = Provider::idle();
    provider.storage.fail_blob_has("cache backend unavailable");

    fail(&provider, &["emery", "specify", "./missing.wasm"], 1, "storage-failed").await;
}

// A path that is not a `.wasm` component file refuses typed.
#[tokio::test]
async fn component_missing() {
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "./missing.wasm"], 1, "adapter-component-missing").await;
    for path in ["/tmp/missing.wasm", "../missing.wasm"] {
        fail(&provider, &["emery", "specify", path], 2, "argument").await;
    }
}

// GitHub URLs are refused: a source checkout is not an adapter (ADR-0002).
#[tokio::test]
async fn github_refused() {
    let provider = Provider::idle();
    fail(
        &provider,
        &["emery", "specify", "https://github.com/acme/api"],
        1,
        "adapter-github-uri-unsupported",
    )
    .await;
}

// An exact package pin (`emery:<name>@<semver>` or the first-party
// shorthand) dispatches by its versioned routed id; admission stays
// static — there is no download path (ADR-0002).
#[tokio::test]
async fn package_pin() {
    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [demo]");
    let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", "emery:demo@1.2.0"]).await;

    let calls = provider.source.calls.lock().expect("calls");
    let (id, input) = calls.first().expect("one extract dispatch");
    assert_eq!(id, "source:demo@1.2.0", "the routed id carries the exact pin");
    assert_eq!(input.key, "demo", "the binding key is the adapter name");
    drop(calls);
    provider.model.assert_exhausted();
}

// Package references pin an exact SemVer — no branches, tags, or
// namespace-less names.
#[tokio::test]
async fn package_ref_refused() {
    let cases: &[(&str, &str)] = &[
        ("emery:demo", "adapter-package-ref-version-required"),
        ("emery:demo@main", "adapter-package-ref-version-required"),
        ("emery:@1.2.0", "adapter-package-ref-malformed"),
    ];
    for (reference, code) in cases {
        let provider = Provider::idle();
        fail(&provider, &["emery", "specify", reference], 1, code).await;
        assert!(provider.storage.is_empty(), "a refused run writes nothing: {code}");
    }
}

// A pointer naming a missing generation is corruption, never an empty
// result.
#[tokio::test]
async fn corrupt_pointer() {
    let provider = Provider::idle();
    provider.storage.insert_state("spec/current", b"0123456789abcdef\n");
    fail(&provider, &["emery", "show", "spec"], 1, "spec-home-corrupt").await;
}

// One shared store, two project-scoped views: multi-project isolation
// is host policy over the engine's flat keys, with no engine change
// (portable-storage step 8).
#[tokio::test]
async fn multi_project_isolation() {
    let workspace = project_tempdir();
    let component = workspace.path().join("source.wasm");
    fs::write(&component, b"\0asm-stub").expect("stub wasm");
    let component = project_arg(&component);

    let shared = Arc::new(Memory::default());
    let alpha = Provider::over(
        Arc::new(Namespaced::new("alpha", Arc::clone(&shared))),
        [SPEC_ANSWER, DESIGN_ANSWER],
    );
    let beta_spec = SPEC_ANSWER.replace("hello", "howdy");
    let beta_design = DESIGN_ANSWER.replace("hello", "howdy");
    let beta = Provider::over(
        Arc::new(Namespaced::new("beta", Arc::clone(&shared))),
        [beta_spec, beta_design],
    );

    cli_ok(&alpha, &["emery", "specify", &component]).await;
    cli_ok(&beta, &["emery", "specify", &component]).await;

    // Every write landed under its project prefix; nothing landed flat.
    assert!(shared.state("spec/current").is_none(), "no unprefixed pointer exists");
    assert!(shared.objects("spec").is_empty(), "no unprefixed generation exists");
    assert!(shared.object("alpha/adapters", "source.wasm").is_some());
    assert!(shared.object("beta/adapters", "source.wasm").is_some());

    let id_alpha = project_pointer(&shared, "alpha");
    let id_beta = project_pointer(&shared, "beta");
    assert_ne!(id_alpha, id_beta, "distinct documents commit distinct generations");

    // Each project's `show` renders its own committed bytes alone.
    let spec_alpha =
        shared.object("alpha/spec", &format!("generations/{id_alpha}/spec.md")).expect("spec.md");
    let spec_beta =
        shared.object("beta/spec", &format!("generations/{id_beta}/spec.md")).expect("spec.md");
    let shown = cli_ok(&alpha, &["emery", "show", "spec"]).await;
    assert_eq!(shown.stdout, spec_alpha, "alpha shows its own generation");
    let shown = cli_ok(&beta, &["emery", "show", "spec"]).await;
    assert_eq!(shown.stdout, spec_beta, "beta shows its own generation");
    assert!(String::from_utf8_lossy(&spec_beta).contains("howdy"));
    assert!(!String::from_utf8_lossy(&spec_alpha).contains("howdy"));

    alpha.model.assert_exhausted();
    beta.model.assert_exhausted();
}

// Reads the current-generation id from a project's store.
fn pointer(storage: &Memory) -> String {
    let pointer = storage.state("spec/current").expect("current");
    String::from_utf8(pointer).expect("utf-8 pointer").trim().to_string()
}

// Reads a committed generation document from the store.
fn generation(storage: &Memory, id: &str, name: &str) -> Vec<u8> {
    storage.object("spec", &format!("generations/{id}/{name}")).unwrap_or_else(|| panic!("{name}"))
}

// Reads a namespaced project's current-generation id from the shared store.
fn project_pointer(shared: &Memory, project: &str) -> String {
    let pointer = shared.state(&format!("{project}/spec/current")).expect("current");
    String::from_utf8(pointer).expect("utf-8 pointer").trim().to_string()
}

// Runs `argv` in JSON mode and asserts the typed failure envelope.
async fn fail(provider: &Provider, argv: &[&str], exit: u8, code: &str) {
    let mut json = vec!["emery", "--format", "json"];
    json.extend(argv.iter().skip(1).copied());
    let resp = cli(provider, &json).await;
    assert_eq!(resp.exit, exit, "{code}: {}", String::from_utf8_lossy(&resp.stderr));
    let envelope: Value = serde_json::from_slice(&resp.stderr).expect("one JSON envelope");
    assert_eq!(envelope["error"], code, "{envelope}");
    assert_eq!(envelope["exit-code"], exit, "{envelope}");
}
