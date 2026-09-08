//! The `specify` → `show` product arc
//!
//! The scenarios an operator lives through: binding sources, generating a
//! specification, reviewing it, regenerating it, and hitting every refusal
//! along the way — an invalid binding, an untrusted adapter, a model answer
//! that breaks the spec grammar.
//!
//! Each scenario drives the real command façade over scripted capabilities,
//! so it reads as usage documentation while still asserting the exact
//! envelope, exit code, and stored revision the operator would see.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::fs;
use std::path::Path;
use std::sync::Arc;

use emery_engine::{CONTAINER, CURRENT};
use emery_source::types::{Authority, ClaimKind, SourceContent};
use emery_source::{DispatchError, types};
use omnia_guest::model::Error as ModelError;
use omnia_guest::plugins::{Digest, Error as LoadError, Location};
use omnia_test::guest::{Memory, Namespaced, Scripted};
use serde_json::Value;
use support::{Provider, claim, cli, cli_ok, digest, evidence, fail, requirement};

const SPEC_ANSWER: &str = include_str!("specify/1-spec.md");
const DESIGN_ANSWER: &str = include_str!("specify/2-design.md");
const PRECEDENCE_ANSWER: &str = include_str!("specify/3-precedence.md");
const SOURCES: &str = include_str!("specify/emery.toml");

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

// One `specify` loads, extracts, and commits — no prior verb; `show`
// renders the committed bytes alone; an identical re-run is
// byte-stable and says so.
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
    let resp = cli_ok(&provider, &["emery", "specify", &component]).await;

    // --------------------------------------------------
    // Observe: the load, the current id, and the revision.
    // --------------------------------------------------
    let request = provider.plugins.loads().first().cloned().expect("one load request");
    assert_eq!(request.package, "source:source", "the routed id is the loaded package identity");
    let Location::Path(path) = &request.location else {
        panic!("a local component loads by path");
    };
    assert!(path.ends_with("source.wasm"), "the preopen-relative path rides the request: {path}");
    assert!(request.digest.is_none(), "an unpinned binding requests no digest");
    // TOFU: the resolved digest rides the success envelope for the
    // operator to commit as the binding's pin.
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains(&format!("digest source: {}", digest("ab"))), "{stdout}");
    assert!(
        provider.storage.objects("adapters").is_empty(),
        "nothing mirrors into engine storage; the loader reads the file fresh"
    );
    assert!(provider.storage.state("project.yaml").is_none(), "no project record exists");
    let id = current(&provider.storage);
    let spec = document(&provider.storage, &id, "spec.md");
    assert!(String::from_utf8_lossy(&spec).contains("[unknown]"));
    let design = document(&provider.storage, &id, "design.md");
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

// `--config` is the other specify authority: entry names become
// binding keys, and a local adapter resolves relative to the file.
#[tokio::test]
async fn from_file() {
    let workspace = project_tempdir();
    fs::write(workspace.path().join("source.wasm"), b"\0asm-stub").expect("stub wasm");
    let config = workspace.path().join("emery.toml");
    fs::write(&config, SOURCES).expect("write emery.toml");
    let config = project_arg(&config);

    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [greeting]");
    let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);

    let resp = cli_ok(&provider, &["emery", "specify", "--config", &config]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("sources: 1"), "{stdout}");
    let request = provider.plugins.loads().first().cloned().expect("one load request");
    let Location::Path(path) = &request.location else {
        panic!("a local component loads by path");
    };
    assert!(
        path.ends_with("source.wasm") && !path.starts_with("./"),
        "the file-relative selector resolves against the config directory: {path}"
    );

    let id = current(&provider.storage);
    let spec = document(&provider.storage, &id, "spec.md");
    assert!(
        String::from_utf8_lossy(&spec).contains("Sources: [greeting]"),
        "the entry name is the binding key"
    );
    let shown = cli_ok(&provider, &["emery", "show", "spec"]).await;
    assert_eq!(shown.stdout, spec, "show renders the committed spec.md alone");

    provider.model.assert_exhausted();
}

// One adapter may bind several roots: the loader is asked once, and
// each binding extracts over its own workspace.
#[tokio::test]
async fn shared_adapter_several_roots() {
    let cases: &[(&str, &str, bool)] = &[
        ("emery:documentation@1.2.0", "emery:documentation@1.2.0", false),
        ("./source.wasm", "source:source", true),
    ];
    for (adapter, package, wasm) in cases {
        let dir = project_tempdir();
        if *wasm {
            fs::write(dir.path().join("source.wasm"), b"\0asm-stub").expect("stub wasm");
        }
        let config = dir.path().join("emery.toml");
        fs::write(
            &config,
            format!(
                "[[source]]\nname = \"docs\"\nadapter = \"{adapter}\"\npath = \"docs\"\n\n\
                 [[source]]\nname = \"api\"\nadapter = \"{adapter}\"\npath = \"api\"\n"
            ),
        )
        .expect("write emery.toml");
        let config = project_arg(&config);

        let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [docs, api]");
        let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);

        let resp = cli_ok(&provider, &["emery", "specify", "--config", &config]).await;
        let stdout = String::from_utf8_lossy(&resp.stdout);
        assert!(stdout.contains("sources: 2"), "{adapter}: {stdout}");
        assert!(stdout.contains(&format!("digest docs: {}", digest("ab"))), "{adapter}: {stdout}");
        assert!(stdout.contains(&format!("digest api: {}", digest("ab"))), "{adapter}: {stdout}");

        let loads = provider.plugins.loads();
        assert_eq!(loads.len(), 1, "{adapter}: one adapter identity loads once");
        assert_eq!(loads[0].package, *package, "{adapter}");

        let calls = provider.source.calls.lock().expect("calls");
        assert_eq!(calls.len(), 2, "{adapter}: each binding extracts");
        assert_eq!(calls[0].0, *package);
        assert_eq!(calls[0].1.key, "docs");
        assert_eq!(calls[1].0, *package);
        assert_eq!(calls[1].1.key, "api");
        drop(calls);

        provider.model.assert_exhausted();
    }
}

// A run naming no bindings at all discovers the project-root
// `emery.toml` before failing — never merged with argv bindings. The
// CWD move is hermetic under nextest's process-per-test isolation.
#[tokio::test]
async fn discovery() {
    let project = tempfile::TempDir::new().expect("project dir");
    fs::write(
        project.path().join("emery.toml"),
        "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\n",
    )
    .expect("write emery.toml");
    std::env::set_current_dir(project.path()).expect("enter project");

    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [docs]");
    let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify"]).await;

    let id = current(&provider.storage);
    let spec = document(&provider.storage, &id, "spec.md");
    assert!(String::from_utf8_lossy(&spec).contains("Sources: [docs]"));
    provider.model.assert_exhausted();
}

// `--description` binds inline text under the adapter's name: no
// filesystem lend reaches extract, and a bare adapter needs no local
// component.
#[tokio::test]
async fn description_binding() {
    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [intent]");
    let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", "--description", "intent=Ship it."]).await;

    let calls = provider.source.calls.lock().expect("calls");
    let (id, input) = calls.first().expect("one extract dispatch");
    assert_eq!(id, "source:intent", "a bare adapter dispatches by routed name");
    assert_eq!(input.key, "intent");
    assert_eq!(input.content, SourceContent::Value("Ship it.".to_string()));
    drop(calls);

    let id = current(&provider.storage);
    let spec = document(&provider.storage, &id, "spec.md");
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
            "--description",
            "intent=Sessions expire after 30.",
        ],
    )
    .await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("requirements: 3"), "{stdout}");
    assert!(stdout.contains("sources: 4"), "{stdout}");

    let id = current(&provider.storage);
    let spec = String::from_utf8(document(&provider.storage, &id, "spec.md")).expect("utf-8");
    assert!(spec.contains("login.flow [conflict]"), "tied docs peers conflict: {spec}");
    assert!(spec.contains("session.timeout [divergence]"), "intent outranks docs: {spec}");
    assert!(spec.contains("Sources: [intent, docs, code]"), "the intent directive wins: {spec}");
    assert!(spec.contains("[unknown]"), "the acceptance gap is preserved: {spec}");
    provider.model.assert_exhausted();
}

// A re-run over changed evidence supersedes the revision: the old
// blobs are pruned, the current id swaps, and the success envelope
// reports the re-mine diff by heading subject — added, removed, and
// changed sections alike — while a block that only moved, taking a new
// positional id, is not a change.
#[tokio::test]
async fn remine_supersedes() {
    // --------------------------------------------------
    // First run: the docs describe a greeting, a legacy export, and a
    // session timeout.
    // --------------------------------------------------
    let mut provider = Provider::answering([REMINE_FIRST, DESIGN_ANSWER]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(docs_evidence(&[
            ("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            ("legacy.export", "Exports ship nightly."),
            ("session.timeout", "Sessions time out after an hour."),
        ])),
    );
    cli_ok(&provider, &["emery", "specify", "docs"]).await;
    let first = current(&provider.storage);

    // --------------------------------------------------
    // Second run: the timeout now leads, the greeting changed, the
    // export is gone, and an audit requirement appeared.
    // --------------------------------------------------
    let mut provider =
        Provider::over(Arc::clone(&provider.storage), [REMINE_SECOND, DESIGN_ANSWER]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(docs_evidence(&[
            ("session.timeout", "Sessions time out after an hour."),
            ("greeting.behaviour", "GET /greeting returns the static string 'howdy'."),
            ("access.audit", "Access is audited."),
        ])),
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
    assert!(!stdout.contains("session.timeout"), "a renumbered block is not a change: {stdout}");

    let second = current(&provider.storage);
    assert_ne!(first, second, "changed documents commit a new revision");
    assert!(
        provider.storage.object(CONTAINER, &format!("{first}/spec.md")).is_none(),
        "the superseded revision is pruned"
    );
    let spec = document(&provider.storage, &second, "spec.md");
    assert!(String::from_utf8_lossy(&spec).contains("howdy"));
    provider.model.assert_exhausted();
}

// Docs evidence over `(subject, statement)` requirements in row order,
// each covered by its own criterion.
fn docs_evidence(requirements: &[(&str, &str)]) -> types::Evidence {
    let claims = requirements
        .iter()
        .flat_map(|(subject, statement)| {
            [
                requirement(subject, statement),
                claim(
                    ClaimKind::Criterion,
                    &format!("{subject}.check"),
                    ("criterion", "The behaviour is observable."),
                ),
            ]
        })
        .collect();
    evidence(Authority::Documentation, claims)
}

const REMINE_FIRST: &str = "# Specification

### Requirement: greeting.behaviour

ID: REQ-001
Sources: [docs]
Status: agreed

GET /greeting returns the static string 'hello'.

#### Scenario: Greeting

- **WHEN** the greeting is requested
- **THEN** the response is hello

### Requirement: legacy.export

ID: REQ-002
Sources: [docs]
Status: agreed

Exports ship nightly.

#### Scenario: Export

- **WHEN** exports are produced
- **THEN** they ship nightly

### Requirement: session.timeout

ID: REQ-003
Sources: [docs]
Status: agreed

Sessions time out after an hour.

#### Scenario: Timeout

- **WHEN** a session is idle for an hour
- **THEN** it times out
";

const REMINE_SECOND: &str = "# Specification

### Requirement: session.timeout

ID: REQ-001
Sources: [docs]
Status: agreed

Sessions time out after an hour.

#### Scenario: Timeout

- **WHEN** a session is idle for an hour
- **THEN** it times out

### Requirement: greeting.behaviour

ID: REQ-002
Sources: [docs]
Status: agreed

GET /greeting returns the static string 'howdy'.

#### Scenario: Greeting

- **WHEN** the greeting is requested
- **THEN** the response is howdy

### Requirement: access.audit

ID: REQ-003
Sources: [docs]
Status: agreed

Access is audited.

#### Scenario: Audit

- **WHEN** access occurs
- **THEN** it is audited
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

    fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
    assert!(provider.storage.state(CURRENT).is_none(), "a refused run commits nothing");
}

// An adapter call failure surfaces as one typed error.
#[tokio::test]
async fn extract_fails() {
    let mut provider = Provider::idle();
    provider.source.evidence.insert(
        "docs".to_string(),
        Err(DispatchError::Call(types::Error::Internal("the adapter exploded".to_string()))),
    );

    fail(&provider, &["emery", "specify", "docs"], 4, "bad_gateway").await;
    assert!(provider.storage.state(CURRENT).is_none(), "a refused run commits nothing");
}

// An adapter declaring a newer minimum `emery-version` than the binary
// refuses with the dedicated version exit code.
#[tokio::test]
async fn version_too_new() {
    let mut provider = Provider::idle();
    provider.source.versions.insert("docs".to_string(), "99.0.0".to_string());

    fail(&provider, &["emery", "specify", "docs"], 1, "unsupported-version").await;
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
        "# S\n\n### Requirement: a\n\nID: REQ-001\nSources: [docs]\nStatus: agreed\n\n\
         A.\n\n#### Scenario: A\n\n- **WHEN** a\n- **THEN** a\n\n\
         ### Requirement: b\n\nID: REQ-001\nSources: [docs]\nStatus: agreed\n\n\
         B.\n\n#### Scenario: B\n\n- **WHEN** b\n- **THEN** b\n",
    ];
    for answer in answers {
        let provider = Provider::answering([answer]);
        fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
        assert!(provider.storage.state(CURRENT).is_none(), "a refused run commits nothing");
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
        "# S\n\n### Requirement: greeting.behaviour\n\nID: REQ-001\nSources: [docs]\n\
         Status: agreed\n\nHello.\n\n#### Scenario: Greeting\n\n\
         - **WHEN** greeted\n- **THEN** hello\n"
            .to_string(),
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
        fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
        assert!(provider.storage.state(CURRENT).is_none(), "a refused run commits nothing");
    }
}

// A parseable two-block answer with one dishonest first block.
fn two_blocks(heading: &str, id: &str, sources: &str, status: &str) -> String {
    format!(
        "# Specification\n\n### Requirement: {heading}\n\nID: {id}\nSources: {sources}\n\
         Status: {status}\n\nGET /greeting returns the static string 'hello'.\n\n\
         #### Scenario: Greeting\n\n- **WHEN** greeted\n- **THEN** hello\n\n\
         ### Requirement: greeting.behaviour acceptance criteria [unknown]\n\n\
         ID: REQ-002\nSources: []\nStatus: unknown\n\nNo source contributed a criterion.\n\n\
         #### Scenario: Acceptance gap\n\n- **WHEN** checked\n- **THEN** behaviour is unknown\n"
    )
}

// The design leg is fail-closed too: an empty second answer refuses.
#[tokio::test]
async fn empty_design() {
    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [docs]");
    let provider = Provider::answering([spec_answer.as_str(), "   "]);
    fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
    assert!(provider.storage.state(CURRENT).is_none(), "a refused run commits nothing");
}

// A model transport failure surfaces as one typed synthesis error.
#[tokio::test]
async fn model_fails() {
    let provider = Provider {
        model: Scripted::new([Err(ModelError::Backend("scripted transport failure".into()))]),
        ..Provider::idle()
    };
    fail(&provider, &["emery", "specify", "docs"], 4, "bad_gateway").await;
    provider.model.assert_exhausted();
}

// The operator-owned `emery.toml` parses fail-closed: every malformed
// carrier refuses typed before anything commits.
#[tokio::test]
async fn config_file_refused() {
    let cases: &[(&str, u8, &str, &str)] = &[
        ("not toml [", 1, "bad_request", "TOML parse error"),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\nbranch = \"main\"\n",
            1,
            "bad_request",
            "unknown field `branch`",
        ),
        ("", 1, "specify-source-required", ""),
        // The superseded `[sources.<key>]` / `value` schema fails loudly.
        (
            "[sources.docs]\nadapter = \"documentation\"\n",
            1,
            "bad_request",
            "unknown field `sources`",
        ),
        (
            "[[source]]\nname = \"intent\"\nadapter = \"intent\"\nvalue = \"text\"\n",
            1,
            "bad_request",
            "unknown field `value`",
        ),
        ("[[source]]\nadapter = \"documentation\"\n", 1, "bad_request", "missing field `name`"),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\npath = \"docs\"\n\
             description = \"text\"\n",
            1,
            "bad_request",
            "more than one of `path`",
        ),
        // Duplicate names reuse argv's typed duplicate error.
        (
            "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\n\n\
             [[source]]\nname = \"docs\"\nadapter = \"intent\"\n",
            1,
            "bad_request",
            "bound twice",
        ),
        // A malformed pin on a local component refuses before any load.
        (
            "[[source]]\nname = \"pinned\"\nadapter = \"./source.wasm\"\n\
             digest = \"sha256:9f2c44aa\"\n",
            1,
            "bad_request",
            "64 hex characters",
        ),
        (
            "[[source]]\nname = \"upstream\"\nadapter = \"documentation\"\ngit = \"https://github.com/acme/api@v2\"\n",
            1,
            "bad_request",
            "`git` and `url` are not supported",
        ),
        (
            "[[source]]\nname = \"upstream\"\nadapter = \"documentation\"\nurl = \"https://example.com/openapi.yaml\"\n",
            1,
            "bad_request",
            "`git` and `url` are not supported",
        ),
        (
            "[[source]]\nname = \"upstream\"\nadapter = \"documentation\"\ngit = \"git+https://github.com/acme/api#deadbeef\"\n",
            1,
            "bad_request",
            "drop the `git+` prefix",
        ),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\npath = \"../../outside\"\n",
            1,
            "bad_request",
            "../../outside",
        ),
        (
            "[[source]]\nname = \"local\"\nadapter = \"/tmp/source.wasm\"\n",
            1,
            "bad_request",
            "`/tmp/source.wasm`",
        ),
    ];
    for (body, exit, code, fragment) in cases {
        let dir = project_tempdir();
        let path = dir.path().join("emery.toml");
        fs::write(&path, body).expect("write emery.toml");
        let path = project_arg(&path);
        let provider = Provider::idle();
        let envelope = fail(&provider, &["emery", "specify", "--config", &path], *exit, code).await;
        if !fragment.is_empty() {
            let message = envelope["message"].as_str().unwrap_or("");
            assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
        }
        assert!(provider.storage.is_empty(), "a refused run writes nothing: {code}");
    }

    // An unreadable file is a typed filesystem error.
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "--config", "nonexistent/emery.toml"], 3, "server_error")
        .await;

    // Host-absolute and escaping paths never cross into the guest namespace.
    for path in ["/nonexistent/emery.toml", "../emery.toml"] {
        fail(&provider, &["emery", "specify", "--config", path], 1, "bad_request").await;
    }
}

// The loader keys are gated by selector kind: `registry` only steers
// registry acquisition, so it rides only a package-shaped selector,
// and a `digest` pin binds exact bytes the loader acquires, so a bare
// name — which never loads — cannot carry one.
#[tokio::test]
async fn loader_keys_gated() {
    let pinned_bare = format!(
        "[[source]]\nname = \"pinned\"\nadapter = \"documentation\"\ndigest = \"{}\"\n",
        digest("ab")
    );
    let cases: &[(&str, &str)] = &[
        (
            "[[source]]\nname = \"local\"\nadapter = \"./source.wasm\"\n\
             registry = \"registry.acme.example\"\n",
            "`registry` requires a package adapter",
        ),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\n\
             registry = \"registry.acme.example\"\n",
            "`registry` requires a package adapter",
        ),
        (pinned_bare.as_str(), "not a bare name"),
    ];
    for (body, fragment) in cases {
        let dir = project_tempdir();
        let path = dir.path().join("emery.toml");
        fs::write(&path, body).expect("write emery.toml");
        let path = project_arg(&path);
        let provider = Provider::idle();
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &path], 1, "bad_request").await;
        let message = envelope["message"].as_str().unwrap_or("");
        assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
        assert!(provider.storage.is_empty(), "a refused run writes nothing: {fragment}");
    }
}

// File-relative `path` entries anchor at the file's directory, fold
// `.` and `..` lexically, and stay `.`-relative so the guest preopen
// can open them; `description` entries lend nothing; `[[source]]`
// entries bind in declaration order, not name order — all observed on
// the `SourceInput` the adapter receives.
#[tokio::test]
async fn binding_paths() {
    let dir = project_tempdir();
    let path = dir.path().join("emery.toml");
    fs::write(
        &path,
        "[[source]]\nname = \"zulu\"\nadapter = \"documentation\"\npath = \"nested/../docs\"\n\n\
         [[source]]\nname = \"intent\"\nadapter = \"intent\"\ndescription = \"Ship it.\"\n\n\
         [[source]]\nname = \"alpha\"\nadapter = \"local\"\npath = \"./docs\"\n",
    )
    .expect("write emery.toml");

    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [zulu, intent, alpha]");
    let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);
    let path = project_arg(&path);
    cli_ok(&provider, &["emery", "specify", "--config", &path]).await;

    let calls = provider.source.calls.lock().expect("calls");
    let order: Vec<&str> = calls.iter().map(|(_, input)| input.key.as_str()).collect();
    assert_eq!(order, ["zulu", "intent", "alpha"], "entries bind in declaration order");
    for key in ["zulu", "alpha"] {
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
        "a file `description` entry binds inline text"
    );
    drop(calls);
    provider.model.assert_exhausted();
}

// Local components are read fresh on every run — nothing mirrors, so
// a re-run after the operator deletes the source file refuses typed.
#[tokio::test]
async fn deleted_component_refused() {
    let workspace = project_tempdir();
    let component = workspace.path().join("source.wasm");
    fs::write(&component, b"\0asm-stub").expect("stub wasm");
    let component = project_arg(&component);

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    cli_ok(&provider, &["emery", "specify", &component]).await;

    fs::remove_file(&component).expect("remove the operator's file");
    fail(&provider, &["emery", "specify", &component], 2, "not_found").await;
    provider.model.assert_exhausted();
}

// A path that is not a `.wasm` component file refuses typed.
#[tokio::test]
async fn component_missing() {
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "./missing.wasm"], 2, "not_found").await;
    for path in ["/tmp/missing.wasm", "../missing.wasm"] {
        fail(&provider, &["emery", "specify", path], 1, "bad_request").await;
    }
}

// A pin that matches the resolved bytes loads and extracts, and the
// success envelope confirms the digest beside the binding key.
#[tokio::test]
async fn pinned_component() {
    let dir = project_tempdir();
    fs::write(dir.path().join("source.wasm"), b"\0asm-stub").expect("stub wasm");
    let config = dir.path().join("emery.toml");
    fs::write(
        &config,
        format!(
            "[[source]]\nname = \"local\"\nadapter = \"./source.wasm\"\ndigest = \"{}\"\n",
            digest("ab")
        ),
    )
    .expect("write emery.toml");
    let config = project_arg(&config);

    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [local]");
    let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);

    let resp = cli_ok(&provider, &["emery", "specify", "--config", &config]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains(&format!("digest local: {}", digest("ab"))), "{stdout}");

    let loads = provider.plugins.loads();
    let request = loads.first().expect("one load request");
    assert_eq!(request.digest, Some(digest("ab")), "the binding's pin rides the load request");
    provider.model.assert_exhausted();
}

// A pinned local component must hash to exactly the pinned bytes: the
// loader's typed mismatch refusal surfaces on the exit contract before
// anything extracts or commits.
#[tokio::test]
async fn digest_mismatch_refused() {
    let dir = project_tempdir();
    fs::write(dir.path().join("source.wasm"), b"\0asm-stub").expect("stub wasm");
    let config = dir.path().join("emery.toml");
    fs::write(
        &config,
        format!(
            "[[source]]\nname = \"local\"\nadapter = \"./source.wasm\"\ndigest = \"{}\"\n",
            digest("11")
        ),
    )
    .expect("write emery.toml");
    let config = project_arg(&config);

    let mut provider = Provider::idle();
    provider.plugins = provider.plugins.clone().digest("source:source", digest("ab"));

    fail(&provider, &["emery", "specify", "--config", &config], 1, "refused").await;
    assert!(provider.storage.is_empty(), "a refused run writes nothing");
}

// TOFU: an unpinned local component reports its resolved digest in the
// JSON success envelope so the operator can commit it as the pin.
#[tokio::test]
async fn tofu_digest_reported() {
    let workspace = project_tempdir();
    let component = workspace.path().join("source.wasm");
    fs::write(&component, b"\0asm-stub").expect("stub wasm");
    let component = project_arg(&component);

    let mut provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    provider.plugins = provider.plugins.clone().digest("source:source", digest("cd"));

    let resp = cli(&provider, &["emery", "--format", "json", "specify", &component]).await;
    assert_eq!(resp.exit, 0, "{}", String::from_utf8_lossy(&resp.stderr));
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["digests"][0]["source"], "source", "{envelope}");
    assert_eq!(envelope["digests"][0]["digest"], digest("cd").as_str(), "{envelope}");
    provider.model.assert_exhausted();
}

// GitHub URLs are refused: a source checkout is not an adapter.
#[tokio::test]
async fn github_refused() {
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "https://github.com/acme/api"], 1, "bad_request").await;
}

// An exact package reference (`emery:<name>@<semver>`, or the
// first-party shorthand as sugar for the `emery` namespace) loads
// through the deployment loader from the acquirer's default registry
// and dispatches by its own package identity — no parallel routed id.
#[tokio::test]
async fn package_loads() {
    for reference in ["emery:demo@1.2.0", "demo@1.2.0"] {
        let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [demo]");
        let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);

        cli_ok(&provider, &["emery", "specify", reference]).await;

        let loads = provider.plugins.loads();
        let request = loads.first().expect("one load request");
        assert_eq!(
            request.package, "emery:demo@1.2.0",
            "the package reference is the load identity: {reference}"
        );
        assert_eq!(
            request.location,
            Location::Registry(None),
            "no override selects the acquirer's default registry"
        );
        assert!(request.digest.is_none(), "an unpinned binding requests no digest");
        let calls = provider.source.calls.lock().expect("calls");
        let (id, input) = calls.first().expect("one extract dispatch");
        assert_eq!(id, "emery:demo@1.2.0", "the routed id is the loaded package identity");
        assert_eq!(input.key, "demo", "the binding key is the adapter name");
        drop(calls);
        provider.model.assert_exhausted();
    }
}

// The binding's `registry` key overrides the acquirer's default
// endpoint per source, and an unpinned registry load reports its
// resolved digest for the operator to commit (TOFU).
#[tokio::test]
async fn registry_override() {
    let dir = project_tempdir();
    let config = dir.path().join("emery.toml");
    fs::write(
        &config,
        "[[source]]\nname = \"ledger\"\nadapter = \"acme:ledger@2.1.0\"\n\
         registry = \"registry.acme.example\"\n",
    )
    .expect("write emery.toml");
    let config = project_arg(&config);

    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [ledger]");
    let mut provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);
    provider.plugins = provider.plugins.clone().digest("acme:ledger@2.1.0", digest("cd"));

    let resp = cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

    let loads = provider.plugins.loads();
    let request = loads.first().expect("one load request");
    assert_eq!(request.package, "acme:ledger@2.1.0", "third-party namespaces pass through");
    assert_eq!(
        request.location,
        Location::Registry(Some("registry.acme.example".to_string())),
        "the binding's override rides the load request"
    );
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains(&format!("digest ledger: {}", digest("cd"))), "{stdout}");
    provider.model.assert_exhausted();
}

// A registry package pin verifies like a local component pin: the pin
// rides the load request, and a mismatch refuses typed before
// anything extracts or commits.
#[tokio::test]
async fn pinned_package() {
    let pinned = |pin: &Digest| {
        format!("[[source]]\nname = \"demo\"\nadapter = \"emery:demo@1.2.0\"\ndigest = \"{pin}\"\n")
    };

    let dir = project_tempdir();
    let config = dir.path().join("emery.toml");
    fs::write(&config, pinned(&digest("ab"))).expect("write emery.toml");
    let config = project_arg(&config);

    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [demo]");
    let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);
    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;
    let request = provider.plugins.loads().first().cloned().expect("one load request");
    assert_eq!(request.digest, Some(digest("ab")), "the binding's pin rides the load request");
    provider.model.assert_exhausted();

    let mismatched = project_tempdir();
    let config = mismatched.path().join("emery.toml");
    fs::write(&config, pinned(&digest("11"))).expect("write emery.toml");
    let config = project_arg(&config);

    let mut provider = Provider::idle();
    provider.plugins = provider.plugins.clone().digest("emery:demo@1.2.0", digest("ab"));
    fail(&provider, &["emery", "specify", "--config", &config], 1, "refused").await;
    assert!(provider.storage.is_empty(), "a refused run writes nothing");
}

// A second binding that re-pins an already-loaded adapter refuses
// already-active: the loader cannot re-bind the identity.
#[tokio::test]
async fn shared_adapter_conflicting_pin() {
    let dir = project_tempdir();
    let config = dir.path().join("emery.toml");
    fs::write(
        &config,
        format!(
            "[[source]]\nname = \"a\"\nadapter = \"emery:demo@1.2.0\"\ndigest = \"{}\"\n\n\
             [[source]]\nname = \"b\"\nadapter = \"emery:demo@1.2.0\"\ndigest = \"{}\"\n",
            digest("ab"),
            digest("cd"),
        ),
    )
    .expect("write emery.toml");
    let config = project_arg(&config);

    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "--config", &config], 1, "already-active").await;
    assert!(provider.storage.is_empty(), "a refused run writes nothing");
    let loads = provider.plugins.loads();
    assert_eq!(loads.len(), 1, "the conflicting pin never reaches the loader");
}

// Load failures land on the exit contract: an acquisition (registry
// or network) failure is the loader's `unavailable` on the
// BadGateway exit; a component refused host-side validation is
// `refused` on the BadRequest exit.
#[tokio::test]
async fn load_failures_typed() {
    let mut provider = Provider::idle();
    provider.plugins = provider.plugins.clone().refuse(
        "emery:demo@1.2.0",
        LoadError::Unavailable("resolving `emery:demo@1.2.0`: endpoint unreachable".to_string()),
    );
    fail(&provider, &["emery", "specify", "emery:demo@1.2.0"], 4, "unavailable").await;
    assert!(provider.storage.is_empty(), "a refused run writes nothing");

    let mut provider = Provider::idle();
    provider.plugins = provider
        .plugins
        .clone()
        .refuse("emery:demo@1.2.0", LoadError::Refused("not a raw wasm component".to_string()));
    fail(&provider, &["emery", "specify", "emery:demo@1.2.0"], 1, "refused").await;
    assert!(provider.storage.is_empty(), "a refused run writes nothing");
}

// Package references pin an exact SemVer — no branches, tags, or
// namespace-less names.
#[tokio::test]
async fn package_ref_refused() {
    let cases: &[(&str, &str)] = &[
        ("emery:demo", "missing `@<version>`"),
        ("emery:demo@main", "invalid version `main`"),
        ("emery:@1.2.0", "missing a name before `@`"),
    ];
    for (reference, fragment) in cases {
        let provider = Provider::idle();
        let envelope = fail(&provider, &["emery", "specify", reference], 1, "bad_request").await;
        let message = envelope["message"].as_str().unwrap_or("");
        assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
        assert!(provider.storage.is_empty(), "a refused run writes nothing: {reference}");
    }
}

// A current id naming a missing revision is corruption, never an empty
// result.
#[tokio::test]
async fn corrupt_current() {
    let provider = Provider::idle();
    provider.storage.insert_state(CURRENT, b"0123456789abcdef");
    fail(&provider, &["emery", "show", "spec"], 3, "server_error").await;
}

// The store is content-addressed: a committed document rewritten under
// its id no longer hashes to it, and `show` refuses rather than render
// bytes the id never named.
#[tokio::test]
async fn tampered_revision() {
    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [docs]");
    let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);
    cli_ok(&provider, &["emery", "specify", "docs"]).await;
    let id = current(&provider.storage);

    provider.storage.insert_object(CONTAINER, &format!("{id}/spec.md"), b"# Rewritten\n");

    fail(&provider, &["emery", "show", "spec"], 3, "server_error").await;
}

// Regeneration is the recovery path: a `specify` over a tampered
// predecessor commits, prunes the tampered blobs, and suppresses only
// the advisory diff.
#[tokio::test]
async fn specify_repairs_tampered() {
    let first_spec = SPEC_ANSWER.replace("Sources: [source]", "Sources: [docs]");
    let second_spec = first_spec.replace("hello", "howdy");
    let second_design = DESIGN_ANSWER.replace("hello", "howdy");
    let provider = Provider::answering([
        first_spec.as_str(),
        DESIGN_ANSWER,
        second_spec.as_str(),
        second_design.as_str(),
    ]);

    cli_ok(&provider, &["emery", "specify", "docs"]).await;
    let first = current(&provider.storage);
    provider.storage.insert_object(CONTAINER, &format!("{first}/spec.md"), b"# Rewritten\n");

    let resp = cli_ok(&provider, &["emery", "specify", "docs"]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(!stdout.contains("diff vs"), "an unreadable predecessor yields no diff: {stdout}");

    let second = current(&provider.storage);
    assert_ne!(first, second, "the repaired store names the new revision");

    for name in ["spec.md", "design.md"] {
        assert!(
            provider.storage.object(CONTAINER, &format!("{first}/{name}")).is_none(),
            "the tampered predecessor is pruned: {name}"
        );
    }

    let shown = cli_ok(&provider, &["emery", "show", "spec"]).await;
    assert!(String::from_utf8_lossy(&shown.stdout).contains("howdy"), "show renders the repair");

    provider.model.assert_exhausted();
}

// The current id is a raw compare-and-swap token: bytes that decode to
// no id fail `show` closed, yet the next `specify` swaps over them, so
// a corrupt store never dead-ends the grammar.
#[tokio::test]
async fn specify_repairs_current() {
    let spec_answer = SPEC_ANSWER.replace("Sources: [source]", "Sources: [docs]");
    let provider = Provider::answering([spec_answer.as_str(), DESIGN_ANSWER]);
    provider.storage.insert_state(CURRENT, b"\xff\xfe");
    fail(&provider, &["emery", "show", "spec"], 3, "server_error").await;

    cli_ok(&provider, &["emery", "specify", "docs"]).await;

    let id = current(&provider.storage);
    assert!(provider.storage.object(CONTAINER, &format!("{id}/spec.md")).is_some());
    cli_ok(&provider, &["emery", "show", "spec"]).await;
    provider.model.assert_exhausted();
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

    // `Memory` is a shared handle: every clone reads the same store.
    let shared = Memory::default();
    let alpha = Provider::over(
        Arc::new(Namespaced::new("alpha", shared.clone())),
        [SPEC_ANSWER, DESIGN_ANSWER],
    );
    let beta_spec = SPEC_ANSWER.replace("hello", "howdy");
    let beta_design = DESIGN_ANSWER.replace("hello", "howdy");
    let beta =
        Provider::over(Arc::new(Namespaced::new("beta", shared.clone())), [beta_spec, beta_design]);

    cli_ok(&alpha, &["emery", "specify", &component]).await;
    cli_ok(&beta, &["emery", "specify", &component]).await;

    // Every write landed under its project prefix; nothing landed flat.
    assert!(shared.state(CURRENT).is_none(), "no unprefixed current id exists");
    assert!(shared.objects(CONTAINER).is_empty(), "no unprefixed revision exists");

    let id_alpha = project_current(&shared, "alpha");
    let id_beta = project_current(&shared, "beta");
    assert_ne!(id_alpha, id_beta, "distinct documents commit distinct revisions");

    // Each project's `show` renders its own committed bytes alone.
    let spec_alpha = shared
        .object(&format!("alpha/{CONTAINER}"), &format!("{id_alpha}/spec.md"))
        .expect("spec.md");
    let spec_beta = shared
        .object(&format!("beta/{CONTAINER}"), &format!("{id_beta}/spec.md"))
        .expect("spec.md");
    let shown = cli_ok(&alpha, &["emery", "show", "spec"]).await;
    assert_eq!(shown.stdout, spec_alpha, "alpha shows its own revision");
    let shown = cli_ok(&beta, &["emery", "show", "spec"]).await;
    assert_eq!(shown.stdout, spec_beta, "beta shows its own revision");
    assert!(String::from_utf8_lossy(&spec_beta).contains("howdy"));
    assert!(!String::from_utf8_lossy(&spec_alpha).contains("howdy"));

    alpha.model.assert_exhausted();
    beta.model.assert_exhausted();
}

// Reads the current revision id from a project's store.
fn current(storage: &Memory) -> String {
    let raw = storage.state(CURRENT).expect("current");
    String::from_utf8(raw).expect("utf-8 revision id")
}

// Reads a committed revision document from the store.
fn document(storage: &Memory, id: &str, name: &str) -> Vec<u8> {
    storage.object(CONTAINER, &format!("{id}/{name}")).unwrap_or_else(|| panic!("{name}"))
}

// Reads a namespaced project's current revision id from the shared store.
fn project_current(shared: &Memory, project: &str) -> String {
    let raw = shared.state(&format!("{project}/{CURRENT}")).expect("current");
    String::from_utf8(raw).expect("utf-8 revision id")
}
