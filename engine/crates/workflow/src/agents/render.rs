//! Deterministic Markdown renderer for generated `AGENTS.md` context.

use std::fmt::Write;

#[cfg(test)]
const PLACEHOLDER_FINGERPRINT: &str = "sha256:pending";
use super::detect::Detection;

/// Complete input needed to render repository context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub project_name: String,
    pub is_workspace: bool,
    pub detection: Detection,
    pub description: Option<String>,
    pub adapter: Option<Adapter>,
    pub rule_overrides: Vec<Rule>,
    pub declared_tools: Vec<Extension>,
    pub active_slices: Vec<String>,
    pub workspace_peers: Vec<Peer>,
    pub dependencies: Vec<Dep>,
}

/// Adapter details surfaced without embedding adapter-specific prose in
/// the binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapter {
    pub name: String,
    pub version: semver::Version,
    pub description: String,
    pub briefs: Vec<Brief>,
}

/// One resolved adapter brief.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brief {
    pub phase: String,
    pub id: String,
    pub description: String,
}

/// One `project.yaml.rules` override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub brief_id: String,
    pub path: String,
}

/// One project-scoped WASI tool declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension {
    pub name: String,
    pub version: String,
}

/// One materialized registry workspace slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    pub name: String,
    pub path: String,
}

/// One registry peer dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dep {
    pub name: String,
    pub adapter: String,
    pub url: String,
    pub description: Option<String>,
}

/// Render a complete fenced `AGENTS.md` document with a computed fingerprint.
#[must_use]
pub fn render_document_with_fingerprint(input: &Input, fingerprint: &str) -> String {
    format!(
        "# {name} - Agent Instructions\n\n\
         <!-- specify:context begin\n\
         fingerprint: {fingerprint}\n\
         generated-by: specify {version}\n\
         -->\n\n\
         {body}\
         <!-- specify:context end -->\n",
        name = one_line(&input.project_name),
        version = env!("CARGO_PKG_VERSION"),
        body = render_body(input),
    )
}

/// Render only the managed Markdown body between context fences.
#[must_use]
pub fn render_body(input: &Input) -> String {
    let mut sections = Vec::new();
    if !input.is_workspace {
        sections.push(render_section("Runtime", input.detection.runtime_bullets()));
        sections.push(render_section("Tests", input.detection.test_bullets()));
        sections.push(render_section("Linting", input.detection.lint_bullets()));
    }
    sections.push(render_section("Navigation", navigation_bullets(input)));
    sections.push(render_section("Conventions", conventions_bullets(input)));
    sections.push(render_section("Boundaries", boundaries_bullets(input)));
    sections.push(render_section("Dependencies", dependency_bullets(input)));

    let mut body = sections.join("\n");
    body.push('\n');
    body
}

fn render_section(title: &str, mut bullets: Vec<String>) -> String {
    bullets.sort();
    bullets.dedup();

    let mut out = format!("## {title}\n");
    for bullet in bullets {
        let _ = writeln!(&mut out, "- {bullet}");
    }
    out
}

fn navigation_bullets(input: &Input) -> Vec<String> {
    let mut bullets = vec![
        format!("active slices: {} in `.specify/slices/`.", input.active_slices.len()),
        "`.specify/archive/` contains merged or dropped slice history.".to_string(),
        "`.specify/project.yaml` stores Specify project metadata.".to_string(),
        "`.specify/slices/` contains active slice workspaces.".to_string(),
        "`change.md` is the repo-root change brief.".to_string(),
        "`plan.yaml` is the optional repo-root platform plan.".to_string(),
        "`registry.yaml` is the optional repo-root platform registry.".to_string(),
    ];
    for peer in &input.workspace_peers {
        bullets.push(format!(
            "`{}` is the materialized workspace clone for registry peer `{}`.",
            one_line(&peer.path),
            one_line(&peer.name)
        ));
    }
    bullets
}

fn conventions_bullets(input: &Input) -> Vec<String> {
    let mut bullets = Vec::new();
    if let Some(description) =
        input.description.as_deref().map(one_line).filter(|value| !value.is_empty())
    {
        bullets.push(format!("project description: {description}."));
    }
    if let Some(adapter) = &input.adapter {
        bullets.push(format!(
            "adapter `{}` {}: {}.",
            one_line(&adapter.name),
            adapter.version,
            one_line(&adapter.description)
        ));
        for brief in &adapter.briefs {
            bullets.push(format!(
                "pipeline `{}/{}`: {}.",
                one_line(&brief.phase),
                one_line(&brief.id),
                one_line(&brief.description)
            ));
        }
    }
    for rule in &input.rule_overrides {
        bullets.push(format!(
            "rule override `{}`: `{}`.",
            one_line(&rule.brief_id),
            one_line(&rule.path)
        ));
    }
    if bullets.is_empty() {
        bullets.push("no project rules declared.".to_string());
    }
    bullets
}

fn boundaries_bullets(input: &Input) -> Vec<String> {
    let mut bullets = vec![
        "`metadata.yaml` files are framework-managed; update them through `specify slice` commands."
            .to_string(),
        "`.specify/archive/` is framework-managed history.".to_string(),
        "`project.yaml` is the source of truth for Specify project metadata.".to_string(),
    ];
    if let Some(adapter) = &input.adapter {
        bullets
            .push(format!("adapter `{}` owns generated artifact layout.", one_line(&adapter.name)));
    }
    if input.declared_tools.is_empty() {
        bullets.push("no project-scoped WASI tools declared.".to_string());
    } else {
        for tool in &input.declared_tools {
            bullets.push(format!(
                "declared WASI tool `{}` at version `{}`.",
                one_line(&tool.name),
                one_line(&tool.version)
            ));
        }
    }
    bullets
}

fn dependency_bullets(input: &Input) -> Vec<String> {
    if input.dependencies.is_empty() {
        return vec!["single-repo project; no registered peers.".to_string()];
    }
    input
        .dependencies
        .iter()
        .map(|peer| {
            let mut line = format!(
                "`{}` @ `{}` -> `{}`.",
                one_line(&peer.name),
                one_line(&peer.adapter),
                one_line(&peer.url)
            );
            if let Some(description) =
                peer.description.as_deref().map(one_line).filter(|value| !value.is_empty())
            {
                line.push_str(" Description: ");
                line.push_str(&description);
                line.push('.');
            }
            line
        })
        .collect()
}

fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_document(input: &Input) -> String {
        render_document_with_fingerprint(input, PLACEHOLDER_FINGERPRINT)
    }

    fn regular_input() -> Input {
        Input {
            project_name: "demo".to_string(),
            is_workspace: false,
            detection: Detection::default(),
            description: Some("Rust services".to_string()),
            adapter: Some(Adapter {
                name: "omnia".to_string(),
                version: semver::Version::new(1, 0, 0),
                description: "Omnia Rust WASM workflow".to_string(),
                briefs: vec![
                    Brief {
                        phase: "define".to_string(),
                        id: "specs".to_string(),
                        description: "Write requirements".to_string(),
                    },
                    Brief {
                        phase: "define".to_string(),
                        id: "proposal".to_string(),
                        description: "Establish why".to_string(),
                    },
                ],
            }),
            rule_overrides: vec![Rule {
                brief_id: "proposal".to_string(),
                path: ".specify/rules/proposal.md".to_string(),
            }],
            declared_tools: vec![Extension {
                name: "contract".to_string(),
                version: "1.0.0".to_string(),
            }],
            active_slices: vec!["alpha".to_string(), "zeta".to_string()],
            workspace_peers: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn regular_project_section_order() {
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
    }

    #[test]
    fn workspace_omits_detection_sections() {
        let mut input = regular_input();
        input.is_workspace = true;
        input.adapter = None;

        let rendered = render_body(&input);

        assert!(!rendered.contains("## Runtime"));
        assert!(!rendered.contains("## Tests"));
        assert!(!rendered.contains("## Linting"));
        assert!(rendered.contains("## Navigation"));
        assert!(rendered.contains("## Dependencies"));
    }

    #[test]
    fn dependency_bullets_sorted() {
        let mut input = regular_input();
        input.dependencies = vec![
            Dep {
                name: "zeta".to_string(),
                adapter: "omnia@1.0.0".to_string(),
                url: "../zeta".to_string(),
                description: None,
            },
            Dep {
                name: "alpha".to_string(),
                adapter: "omnia@1.0.0".to_string(),
                url: "../alpha".to_string(),
                description: None,
            },
        ];

        let rendered = render_body(&input);
        let alpha = rendered.find("`alpha` @ `omnia@1.0.0`").expect("alpha dependency rendered");
        let zeta = rendered.find("`zeta` @ `omnia@1.0.0`").expect("zeta dependency rendered");

        assert!(alpha < zeta, "dependencies must render in sorted order:\n{rendered}");
    }

    #[test]
    fn dependency_bullets_include_descriptions() {
        let mut input = regular_input();
        input.dependencies = vec![Dep {
            name: "alpha".to_string(),
            adapter: "omnia@1.0.0".to_string(),
            url: "../alpha".to_string(),
            description: Some("Alpha service".to_string()),
        }];

        let rendered = render_body(&input);

        assert!(
            rendered.contains("`alpha` @ `omnia@1.0.0` -> `../alpha`. Description: Alpha service."),
            "dependency description must render when present:\n{rendered}"
        );
    }

    #[test]
    fn navigation_lists_workspace_peers() {
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
        let rendered = render_document(&regular_input());

        assert!(rendered.starts_with("# demo - Agent Instructions\n\n"));
        assert!(rendered.contains("<!-- specify:context begin\n"));
        assert!(rendered.contains("generated-by: specify "));
        assert!(rendered.ends_with("<!-- specify:context end -->\n"));
    }
}
