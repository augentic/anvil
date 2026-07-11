//! Integration coverage for the AGENTS.md write planner
//! (`workflow::agents::fences::plan_agents_write`) and the deterministic
//! Markdown renderer (`workflow::agents::render`): the planner tests sweep
//! the four write dispositions plus non-UTF-8 byte preservation and the two
//! fence-policy errors; the renderer tests pin the section matrix and the
//! fenced document shape.

use workflow::agents::detect::Detection;
use workflow::agents::fences::{FenceError, WriteDisposition, plan_agents_write};
use workflow::agents::render::{Adapter, Dep, Input, Peer, Rule, render_body, render_document};

const GENERATED: &[u8] = b"# demo - Agent Instructions\n\n<!-- specify:context begin\nfingerprint: sha256:new\ngenerated-by: specify 0.2.0\n-->\n\n## Runtime\n- new\n\n<!-- specify:context end -->\n";

fn fenced_existing() -> Vec<u8> {
    b"# hand title\n\n<!-- specify:context begin\nfingerprint: sha256:old\n-->\n\nold body\n\n<!-- specify:context end -->\n\noperator notes\n".to_vec()
}

#[test]
fn plan_write_dispositions() {
    // Absent AGENTS.md → the full generated document is created.
    let planned = plan_agents_write(None, GENERATED, false).expect("plan ok");
    assert_eq!(planned.bytes, GENERATED);
    assert_eq!(planned.disposition, WriteDisposition::Create);

    // Unfenced existing + `--force` → full rewrite.
    let planned =
        plan_agents_write(Some(b"# hand-authored\n"), GENERATED, true).expect("force rewrite ok");
    assert_eq!(planned.bytes, GENERATED);
    assert_eq!(planned.disposition, WriteDisposition::ForceRewriteUnfenced);

    // Fenced existing → only the generated block is spliced, prefix/suffix kept.
    let existing = fenced_existing();
    let planned = plan_agents_write(Some(&existing), GENERATED, false).expect("plan ok");
    let expected = b"# hand title\n\n<!-- specify:context begin\nfingerprint: sha256:new\ngenerated-by: specify 0.2.0\n-->\n\n## Runtime\n- new\n\n<!-- specify:context end -->\n\noperator notes\n";
    assert_eq!(planned.bytes, expected);
    assert_eq!(planned.disposition, WriteDisposition::ReplaceFencedBlock);

    // Identical bytes → Unchanged.
    let planned = plan_agents_write(Some(GENERATED), GENERATED, false).expect("plan ok");
    assert_eq!(planned.bytes, GENERATED);
    assert_eq!(planned.disposition, WriteDisposition::Unchanged);

    // Non-UTF-8 operator bytes in the prefix/suffix are preserved verbatim.
    let existing = [
        b"prefix ".as_slice(),
        &[0xff, b'\n'],
        b"<!-- specify:context begin\nfingerprint: sha256:old\n-->\nold\n<!-- specify:context end -->",
        &[b'\n', 0xfe],
    ]
    .concat();
    let planned = plan_agents_write(Some(&existing), GENERATED, false).expect("plan ok");
    assert!(planned.bytes.starts_with(b"prefix \xff\n<!-- specify:context begin"));
    assert!(planned.bytes.ends_with(b"\n\xfe"));
}

#[test]
fn plan_write_errors() {
    // Unfenced existing without `--force` is refused.
    let err = plan_agents_write(Some(b"# hand-authored\n"), GENERATED, false)
        .expect_err("unfenced must refuse");
    assert_eq!(err, FenceError::ExistingUnfencedAgentsMd);

    // A generated document missing its own fences is rejected.
    let err = plan_agents_write(None, b"# generated but unfenced\n", false)
        .expect_err("generated document without fences must fail");
    assert_eq!(err, FenceError::GeneratedDocumentMissingFences);
}

fn regular_input() -> Input {
    Input {
        project_name: "demo".to_string(),
        is_workspace: false,
        detection: Detection::default(),
        description: Some("Rust services".to_string()),
        adapter: Some(Adapter {
            name: "demo-target".to_string(),
            version: semver::Version::new(1, 0, 0),
        }),
        rule_overrides: vec![Rule {
            brief_id: "proposal".to_string(),
            path: ".specify/rules/proposal.md".to_string(),
        }],
        active_slices: vec!["alpha".to_string(), "zeta".to_string()],
        workspace_peers: Vec::new(),
        dependencies: Vec::new(),
    }
}

// `render_body` is a deterministic `(Input -> Markdown)` projection with no
// CLI fixture pinning the section shapes, so the body cases live here as
// one matrix.
#[test]
fn render_body_matrix() {
    // A regular (non-workspace) project renders all seven sections in
    // order, with the detection sections defaulting to "not detected".
    let rendered = render_body(&regular_input());
    let headings: Vec<&str> = rendered.lines().filter(|line| line.starts_with("## ")).collect();
    assert_eq!(
        headings,
        vec![
            "## Runtime",
            "## Tests",
            "## Linting",
            "## Navigation",
            "## Conventions",
            "## Boundaries",
            "## Dependencies",
        ]
    );
    assert!(rendered.contains("## Runtime\n- not detected\n"));
    assert!(rendered.contains("## Tests\n- not detected\n"));
    assert!(rendered.contains("## Linting\n- not detected\n"));
    assert!(
        rendered.contains(
            "During execute/build/merge, agents consume Specify and adapters — they do not maintain them."
        ),
        "consumer tooling boundary must render in Boundaries:\n{rendered}"
    );
    assert!(
        rendered.contains(
            "stop, print CLI `stop:` / `hint:` / `resume:` output, and exit; never patch `specify`, `specify-adapters`"
        ),
        "consumer tooling stop bullet must render in Boundaries:\n{rendered}"
    );

    // A workspace omits the per-language detection sections.
    let mut input = regular_input();
    input.is_workspace = true;
    input.adapter = None;
    let rendered = render_body(&input);
    assert!(!rendered.contains("## Runtime"));
    assert!(!rendered.contains("## Tests"));
    assert!(!rendered.contains("## Linting"));
    assert!(rendered.contains("## Navigation"));
    assert!(rendered.contains("## Dependencies"));

    // Dependencies render in sorted order.
    let mut input = regular_input();
    input.dependencies = vec![
        Dep {
            name: "zeta".to_string(),
            adapter: "demo-target@1.0.0".to_string(),
            url: "../zeta".to_string(),
            description: None,
        },
        Dep {
            name: "alpha".to_string(),
            adapter: "demo-target@1.0.0".to_string(),
            url: "../alpha".to_string(),
            description: None,
        },
    ];
    let rendered = render_body(&input);
    let alpha = rendered.find("`alpha` @ `demo-target@1.0.0`").expect("alpha dependency rendered");
    let zeta = rendered.find("`zeta` @ `demo-target@1.0.0`").expect("zeta dependency rendered");
    assert!(alpha < zeta, "dependencies must render in sorted order:\n{rendered}");

    // A dependency description renders when present.
    let mut input = regular_input();
    input.dependencies = vec![Dep {
        name: "alpha".to_string(),
        adapter: "demo-target@1.0.0".to_string(),
        url: "../alpha".to_string(),
        description: Some("Alpha service".to_string()),
    }];
    let rendered = render_body(&input);
    assert!(
        rendered
            .contains("`alpha` @ `demo-target@1.0.0` -> `../alpha`. Description: Alpha service."),
        "dependency description must render when present:\n{rendered}"
    );

    // Navigation lists materialized workspace peers with repo-relative paths.
    let mut input = regular_input();
    input.workspace_peers = vec![Peer {
        name: "billing".to_string(),
        path: "workspace/billing/".to_string(),
    }];
    let rendered = render_body(&input);
    assert!(
        rendered.contains(
            "`workspace/billing/` is the materialized workspace clone for registry peer `billing`."
        ),
        "workspace peer path must be repo-relative:\n{rendered}"
    );
}

#[test]
fn full_document_contains_context_fences() {
    let rendered = render_document(&regular_input(), "sha256:pending");

    assert!(rendered.starts_with("# demo - Agent Instructions\n\n"));
    assert!(rendered.contains("<!-- specify:context begin\n"));
    assert!(rendered.contains("generated-by: specify "));
    assert!(rendered.ends_with("<!-- specify:context end -->\n"));
}
