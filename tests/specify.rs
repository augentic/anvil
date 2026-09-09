//! The `specify` → `show` product arc
//!
//! The scenarios an operator lives through: binding sources, generating a
//! specification, reviewing it, regenerating it, and hitting every refusal
//! along the way — an invalid binding, an untrusted adapter, a model draft
//! that does not fit the rows or the plan after every repair.
//!
//! Each scenario drives the real command façade over scripted capabilities,
//! so it reads as usage documentation while still asserting the exact
//! envelope, exit code, and stored revision the operator would see. The
//! model answers are typed drafts, so the scripted turns are JSON and the
//! stored documents are the engine's canonical renderings.

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

// Scripted drafts and the canonical documents the engine renders from them.
const SPEC_ANSWER: &str = include_str!("specify/spec-draft.json");
const SPEC_RENDERED: &str = include_str!("specify/1-spec.md");
const DESIGN_ANSWER: &str = include_str!("specify/design-draft.json");
const DESIGN_RENDERED: &str = include_str!("specify/2-design.md");
const GROUPING_ANSWER: &str = include_str!("specify/grouping.json");
const PRECEDENCE_ANSWER: &str = include_str!("specify/precedence-draft.json");
const PRECEDENCE_RENDERED: &str = include_str!("specify/3-precedence.md");
const SOURCES: &str = include_str!("specify/emery.toml");

// The grouping answer that merges `count` claims into one agreeing
// requirement — what a run over one id bound several times expects.
fn floor_grouping(count: usize) -> String {
    let indices = (0..count).map(|index| index.to_string()).collect::<Vec<_>>().join(", ");
    format!("{{\"groups\": [{{\"claims\": [{indices}], \"classes\": [[{indices}]]}}]}}")
}

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
    // The stored documents are the engine's canonical renderings of the
    // drafts: headings, provenance, the gap tag and note are all rendered.
    let spec = document(&provider.storage, &id, "spec.md");
    assert_eq!(String::from_utf8_lossy(&spec), SPEC_RENDERED, "spec.md is rendered canonically");
    let design = document(&provider.storage, &id, "design.md");
    assert_eq!(
        String::from_utf8_lossy(&design),
        DESIGN_RENDERED,
        "design.md is rendered canonically"
    );

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

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

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
        "the entry name is the binding key the renderer cites"
    );
    let shown = cli_ok(&provider, &["emery", "show", "spec"]).await;
    assert_eq!(shown.stdout, spec, "show renders the committed spec.md alone");

    provider.model.assert_exhausted();
}

// One adapter may bind several roots: the loader is asked once, each
// binding extracts over its own workspace, and the two claims of one id
// are one requirement citing both sources.
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

        let grouping = floor_grouping(2);
        let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);

        let resp = cli_ok(&provider, &["emery", "specify", "--config", &config]).await;
        let stdout = String::from_utf8_lossy(&resp.stdout);
        assert!(stdout.contains("sources: 2"), "{adapter}: {stdout}");
        assert!(stdout.contains("requirements: 1"), "{adapter}: {stdout}");
        let id = current(&provider.storage);
        let spec = document(&provider.storage, &id, "spec.md");
        assert!(
            String::from_utf8_lossy(&spec).contains("Sources: [docs, api]"),
            "{adapter}: both bindings contribute to the one requirement"
        );
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

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

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
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

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

// Requirement identity and agreement are one model partition over the
// byte-equal-id floor, and authority derives the rest: the grouping
// binds `code`'s `session-expiry` into the timeout requirement, where
// the intent directive outranks it as [divergence] with one templated
// loser note; tied documentation peers surface as [conflict] with no
// body; and the uncovered timeout keeps its tag and gains the gap note
// — no synthetic gap row, so the envelope counts two requirements.
#[tokio::test]
async fn authority_precedence() {
    let mut provider = Provider::answering([GROUPING_ANSWER, PRECEDENCE_ANSWER, DESIGN_ANSWER]);
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
                // Behaviour names the timeout differently; the grouping
                // call, not the id, joins it to the requirement.
                requirement("session-expiry", "Sessions expire after 15 minutes of inactivity."),
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
    assert!(stdout.contains("requirements: 2"), "{stdout}");
    assert!(stdout.contains("sources: 4"), "{stdout}");

    // The grouping request indexes every claim and withholds authority.
    let grouping = &provider.model.seen()[0];
    let request = grouping.messages.join("\n");
    assert!(request.contains("- 4 `code` `session-expiry`"), "{request}");
    assert!(request.contains("share the id `session.timeout`"), "the floor is stated: {request}");
    assert!(!request.contains("documentation"), "authority is withheld: {request}");
    assert!(!request.contains("behaviour"), "authority is withheld: {request}");

    let id = current(&provider.storage);
    let spec = String::from_utf8(document(&provider.storage, &id, "spec.md")).expect("utf-8");
    assert_eq!(spec, PRECEDENCE_RENDERED, "every resolution is rendered inline");
    provider.model.assert_exhausted();
}

// A grouping the partition rules refuse — a floor pair split, a claim
// in no group, a claim in two classes — is repaired with the findings
// attached; a run out of repairs refuses typed and commits nothing.
#[tokio::test]
async fn grouping_refused() {
    let bind = |provider: &mut Provider| {
        provider.source.evidence.insert(
            "docs".to_string(),
            Ok(evidence(
                Authority::Documentation,
                vec![requirement("session.timeout", "Sessions expire after 30 minutes.")],
            )),
        );
        provider.source.evidence.insert(
            "code".to_string(),
            Ok(evidence(
                Authority::Behaviour,
                vec![requirement("session.timeout", "Sessions expire after 15 minutes.")],
            )),
        );
    };
    let cases: &[(&str, &str)] = &[
        (
            r#"{"groups": [{"claims": [0], "classes": [[0]]}, {"claims": [1], "classes": [[1]]}]}"#,
            "claims sharing the id `session.timeout` are split across groups",
        ),
        (r#"{"groups": [{"claims": [0], "classes": [[0]]}]}"#, "claim 1 is in no group"),
        (
            r#"{"groups": [{"claims": [0, 1], "classes": [[0, 1], [1]]}]}"#,
            "claim 1 appears in more than one class",
        ),
        (r#"{"groups": [{"claims": [0, 1], "classes": [[0]]}]}"#, "claim 1 is in no class"),
        ("not json", "did not deserialize"),
    ];
    for (answer, fragment) in cases {
        let mut provider = Provider::answering([*answer, *answer, *answer]);
        bind(&mut provider);
        let envelope =
            fail(&provider, &["emery", "specify", "docs", "code"], 1, "bad_request").await;
        let message = envelope["message"].as_str().unwrap_or("");
        assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
        assert!(message.contains("after 2 repairs"), "{envelope}");
        assert!(provider.storage.state(CURRENT).is_none(), "a refused run commits nothing");
        provider.model.assert_exhausted();
    }

    // The repaired answer commits: the same statements in two classes
    // diverge, and the winner is the documentation.
    let refused = r#"{"groups": [{"claims": [0], "classes": [[0]]}]}"#;
    let repaired = r#"{"groups": [{"claims": [0, 1], "classes": [[0], [1]]}]}"#;
    let spec = SPEC_ANSWER.replace("greeting.behaviour", "session.timeout");
    let mut provider = Provider::answering([refused, repaired, spec.as_str(), DESIGN_ANSWER]);
    bind(&mut provider);
    cli_ok(&provider, &["emery", "specify", "docs", "code"]).await;
    let repair = &provider.model.seen()[1];
    let request = repair.messages.join("\n");
    assert!(request.contains("## Findings"), "the findings ride the repair: {request}");
    assert!(request.contains("claim 1 is in no group"), "{request}");
    let id = current(&provider.storage);
    let spec = String::from_utf8(document(&provider.storage, &id, "spec.md")).expect("utf-8");
    assert!(spec.contains("### Requirement: session.timeout [divergence]"), "{spec}");
    assert!(
        spec.contains("Note: code (behaviour, session.timeout): Sessions expire after 15 minutes."),
        "{spec}"
    );
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
    // export is gone, an audit requirement appeared, and the design
    // overview follows the greeting.
    // --------------------------------------------------
    let second_design = DESIGN_ANSWER.replace("hello", "howdy");
    let mut provider =
        Provider::over(Arc::clone(&provider.storage), [REMINE_SECOND, second_design.as_str()]);
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
    assert!(stdout.contains(&format!("diff vs {first}: spec.md, design.md")), "{stdout}");
    assert!(stdout.contains("spec.md + access.audit"), "{stdout}");
    assert!(stdout.contains("spec.md - legacy.export"), "{stdout}");
    assert!(stdout.contains("spec.md ~ greeting.behaviour"), "{stdout}");
    assert!(stdout.contains("design.md ~ Overview"), "{stdout}");
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

// The JSON envelope carries the re-mine diff per document: the changed
// artifacts, then `spec` and `design` section lists keyed by heading.
#[tokio::test]
async fn diff_envelope() {
    let second_design = DESIGN_ANSWER.replace("hello", "howdy");
    let provider =
        Provider::answering([SPEC_ANSWER, DESIGN_ANSWER, SPEC_ANSWER, second_design.as_str()]);
    cli_ok(&provider, &["emery", "specify", "docs"]).await;
    let first = current(&provider.storage);

    let resp = cli(&provider, &["emery", "--format", "json", "specify", "docs"]).await;
    assert_eq!(resp.exit, 0, "{}", String::from_utf8_lossy(&resp.stderr));
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    let diff = &envelope["diff"];
    assert_eq!(diff["from"], first, "{envelope}");
    assert_eq!(diff["artifacts"], serde_json::json!(["design.md"]), "{envelope}");
    assert_eq!(diff["spec"]["changed"], serde_json::json!([]), "{envelope}");
    assert_eq!(diff["design"]["changed"], serde_json::json!(["Overview"]), "{envelope}");
    assert_eq!(diff["design"]["added"], serde_json::json!([]), "{envelope}");
    assert_eq!(diff["design"]["removed"], serde_json::json!([]), "{envelope}");
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

// The drafts are keyed by subject, so their order is immaterial; the
// renderer places each under its row.
const REMINE_FIRST: &str = r#"{
  "preamble": ["The docs describe a greeting, a legacy export, and a session timeout."],
  "requirements": [
    {
      "subject": "greeting.behaviour",
      "body": ["GET /greeting returns the static string 'hello'."],
      "scenarios": [{"name": "Greeting", "when": "the greeting is requested", "then": "the response is hello"}]
    },
    {
      "subject": "legacy.export",
      "body": ["Exports ship nightly."],
      "scenarios": [{"name": "Export", "when": "exports are produced", "then": "they ship nightly"}]
    },
    {
      "subject": "session.timeout",
      "body": ["Sessions time out after an hour."],
      "scenarios": [{"name": "Timeout", "when": "a session is idle for an hour", "then": "it times out"}]
    }
  ]
}"#;

const REMINE_SECOND: &str = r#"{
  "preamble": ["The docs describe a greeting, a legacy export, and a session timeout."],
  "requirements": [
    {
      "subject": "session.timeout",
      "body": ["Sessions time out after an hour."],
      "scenarios": [{"name": "Timeout", "when": "a session is idle for an hour", "then": "it times out"}]
    },
    {
      "subject": "greeting.behaviour",
      "body": ["GET /greeting returns the static string 'howdy'."],
      "scenarios": [{"name": "Greeting", "when": "the greeting is requested", "then": "the response is howdy"}]
    },
    {
      "subject": "access.audit",
      "body": ["Access is audited."],
      "scenarios": [{"name": "Audit", "when": "access occurs", "then": "it is audited"}]
    }
  ]
}"#;

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

// A spec draft outside its schema or its rows is refused after every
// repair, one finding per case: not JSON, a row left undrafted, a subject
// that is not a row, a subject drafted twice, no scenario, no body on a
// non-conflict row, and a paragraph opening with a reserved marker. The
// operator never sees a half-committed run.
#[tokio::test]
async fn invalid_draft() {
    let one = |subject: &str, body: &str, scenarios: &str| {
        format!(
            r#"{{"preamble": [], "requirements": [{{"subject": "{subject}", "body": [{body}], "scenarios": [{scenarios}]}}]}}"#
        )
    };
    let scenario = r#"{"name": "Greeting", "when": "greeted", "then": "hello"}"#;
    let cases: Vec<(String, &str)> = vec![
        ("Not a spec at all.".to_string(), "did not deserialize"),
        (
            r#"{"preamble": [], "requirements": []}"#.to_string(),
            "row `greeting.behaviour` is not drafted",
        ),
        (
            one("greeting.renamed", r#""Hello.""#, scenario),
            "`greeting.renamed` is not a requirement row",
        ),
        (
            format!(
                r#"{{"preamble": [], "requirements": [{{"subject": "greeting.behaviour", "body": ["Hello."], "scenarios": [{scenario}]}}, {{"subject": "greeting.behaviour", "body": ["Again."], "scenarios": [{scenario}]}}]}}"#
            ),
            "drafted more than once",
        ),
        (one("greeting.behaviour", r#""Hello.""#, ""), "has no scenario"),
        (one("greeting.behaviour", "", scenario), "has no body paragraph"),
        (
            one("greeting.behaviour", "\"### Requirement: smuggled\"", scenario),
            "opens with the reserved marker `#`",
        ),
        (
            one("greeting.behaviour", r#""Hello.\nSources: [other]""#, scenario),
            "opens with the reserved marker `Sources:`",
        ),
    ];
    for (answer, fragment) in cases {
        let provider = Provider::answering([answer.as_str(), answer.as_str(), answer.as_str()]);
        let envelope = fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
        let message = envelope["message"].as_str().unwrap_or("");
        assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
        assert!(provider.storage.state(CURRENT).is_none(), "a refused run commits nothing");
        provider.model.assert_exhausted();
    }
}

// A finding is fed back with the previous answer, and the repaired
// draft commits: the operator sees one committed revision, not the
// intermediate miss.
#[tokio::test]
async fn repaired_draft() {
    let missing_scenario =
        SPEC_ANSWER.replace(r#""then": "the response is `hello`""#, r#""then": """#);
    assert_ne!(missing_scenario, SPEC_ANSWER, "the fixture carries the patched line");
    let provider = Provider::answering([missing_scenario.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", "source"]).await;

    let repair = &provider.model.seen()[1];
    let request = repair.messages.join("\n");
    assert!(request.contains("## Previous answer (failed validation)"), "{request}");
    assert!(request.contains("## Findings"), "{request}");
    assert!(request.contains("scenario `then` is blank"), "{request}");
    let id = current(&provider.storage);
    let spec = document(&provider.storage, &id, "spec.md");
    assert_eq!(String::from_utf8_lossy(&spec), SPEC_RENDERED, "the repaired draft is committed");
    provider.model.assert_exhausted();
}

// The design leg is fail-closed too: a draft outside its schema or plan
// is refused after every repair, one finding per case — not JSON, the
// required overview absent, a section outside the closed vocabulary, a
// requirement heading smuggled into a paragraph, and a citation of a
// source the run never bound.
#[tokio::test]
async fn invalid_design() {
    let overview = |text: &str| {
        format!(
            r#"{{"preamble": [], "sections": [{{"kind": "overview", "blocks": [{{"text": "{text}"}}]}}]}}"#
        )
    };
    let cases: Vec<(String, &str)> = vec![
        ("   ".to_string(), "did not deserialize"),
        (r#"{"preamble": [], "sections": []}"#.to_string(), "`## Overview` is required but absent"),
        (
            r#"{"preamble": [], "sections": [{"kind": "decisions", "blocks": [{"text": "Static."}]}]}"#
                .to_string(),
            "did not deserialize",
        ),
        (overview("### Requirement: greeting.behaviour"), "opens with the reserved marker `#`"),
        (overview("The endpoint is static (from nobody)."), "cites source `nobody`, which is not bound"),
    ];
    for (answer, fragment) in cases {
        let provider =
            Provider::answering([SPEC_ANSWER, answer.as_str(), answer.as_str(), answer.as_str()]);
        let envelope = fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
        let message = envelope["message"].as_str().unwrap_or("");
        assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
        assert!(provider.storage.state(CURRENT).is_none(), "a refused run commits nothing");
        provider.model.assert_exhausted();
    }
}

// The evidence plans `design.md`'s sections: a `type` claim requires a
// `domain-model` section referencing it exactly once, and a section no
// claim informs may not appear. Every dishonest draft is refused; the
// honest one commits with the signature rendered verbatim, and `show`
// renders it.
#[tokio::test]
async fn dishonest_design() {
    let signature = "interface Greeting { text: string }";
    let evidence = || {
        Ok(evidence(
            Authority::Documentation,
            vec![
                requirement(
                    "greeting.behaviour",
                    "GET /greeting returns the static string 'hello'.",
                ),
                claim(ClaimKind::Type, "greeting.type", ("signature", signature)),
            ],
        ))
    };
    let draft = |sections: &str| format!(r#"{{"preamble": [], "sections": [{sections}]}}"#);
    let overview = r#"{"kind": "overview", "blocks": [{"text": "One endpoint (from docs)."}]}"#;
    let domain = r#"{"kind": "domain-model", "blocks": [{"text": "The greeting payload is one string field."}, {"type": "greeting.type"}]}"#;
    let honest = draft(&format!("{overview}, {domain}"));
    let cases: Vec<(String, &str)> = vec![
        // The required `domain-model` is missing.
        (draft(overview), "`## Domain model` is required but absent"),
        // `ui-layout` appears with no spatial claim behind it.
        (
            draft(&format!(
                r#"{overview}, {domain}, {{"kind": "ui-layout", "blocks": [{{"text": "- page"}}]}}"#
            )),
            "`## UI / layout` is present but no claim informs it",
        ),
        // The signature is quoted as prose instead of referenced.
        (
            draft(&format!(
                r#"{overview}, {{"kind": "domain-model", "blocks": [{{"text": "`{signature}`"}}]}}"#
            )),
            "type `greeting.type` is never referenced",
        ),
        // The type is referenced twice.
        (
            draft(&format!(
                r#"{overview}, {{"kind": "domain-model", "blocks": [{{"type": "greeting.type"}}, {{"type": "greeting.type"}}]}}"#
            )),
            "type `greeting.type` is referenced 2 times",
        ),
        // A type block outside `domain-model`, naming no type claim.
        (
            draft(&format!(
                r#"{{"kind": "overview", "blocks": [{{"type": "greeting.other"}}]}}, {domain}"#
            )),
            "type blocks belong under `## Domain model`",
        ),
    ];
    for (answer, fragment) in cases {
        let mut provider =
            Provider::answering([SPEC_ANSWER, answer.as_str(), answer.as_str(), answer.as_str()]);
        provider.source.evidence.insert("docs".to_string(), evidence());
        let envelope = fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
        let message = envelope["message"].as_str().unwrap_or("");
        assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
        assert!(provider.storage.state(CURRENT).is_none(), "a refused run commits nothing");
        provider.model.assert_exhausted();
    }

    let mut provider = Provider::answering([SPEC_ANSWER, honest.as_str()]);
    provider.source.evidence.insert("docs".to_string(), evidence());
    cli_ok(&provider, &["emery", "specify", "docs"]).await;
    let shown = cli_ok(&provider, &["emery", "show", "design"]).await;
    let rendered = format!(
        "# Design\n\n## Overview\n\nOne endpoint (from docs).\n\n## Domain model\n\n\
         The greeting payload is one string field.\n\n```\n{signature}\n```\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&shown.stdout),
        rendered,
        "the signature is rendered verbatim where the draft placed it"
    );
    provider.model.assert_exhausted();
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

    // Three sources contribute one id: the grouping turn merges them.
    let grouping = floor_grouping(3);
    let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);
    let path = project_arg(&path);
    cli_ok(&provider, &["emery", "specify", "--config", &path]).await;

    let calls = provider.source.calls.lock().expect("calls");
    let order: Vec<&str> = calls.iter().map(|(_, input)| input.key.as_str()).collect();
    assert_eq!(order, ["zulu", "intent", "alpha"], "entries bind in declaration order");
    for key in ["zulu", "alpha"] {
        let (_, input) = calls.iter().find(|(_, input)| input.key == key).expect("dispatched");
        let SourceContent::Workspace(root) = &input.content else {
            panic!("a path binding lends a workspace");
        };
        assert!(
            !Path::new(root).is_absolute(),
            "the lend must stay `.`-relative for the guest preopen: {root}"
        );
        assert!(
            root.ends_with("docs") && !root.contains(".."),
            "`.` and `..` fold away lexically against the file's directory: {root}"
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

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

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
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

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

    let mut provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
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

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
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
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
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
    let second_spec = SPEC_ANSWER.replace("hello", "howdy");
    let second_design = DESIGN_ANSWER.replace("hello", "howdy");
    let provider = Provider::answering([
        SPEC_ANSWER,
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
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
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
