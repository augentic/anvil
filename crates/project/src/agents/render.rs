//! Deterministic Markdown renderer for generated `AGENTS.md` context.

use std::fmt::Write;

use super::detect::Detection;

/// Complete input needed to render repository context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    /// `project.yaml.name`, used in the document title.
    pub project_name: String,
    /// Registry-only workspace mode — omits the per-language sections.
    pub is_workspace: bool,
    /// Root-marker detection summary.
    pub detection: Detection,
    /// `project.yaml.description`, when set.
    pub description: Option<String>,
    /// The bound target adapter, when the project declares one.
    pub adapter: Option<Adapter>,
    /// `project.yaml.rules` overrides.
    pub rule_overrides: Vec<Rule>,
    /// Names of active slices under `.specify/slices/`.
    pub active_slices: Vec<String>,
    /// Materialized workspace slots.
    pub workspace_peers: Vec<Peer>,
    /// Registry peer dependencies.
    pub dependencies: Vec<Dep>,
}

/// Adapter details surfaced without embedding adapter-specific prose in
/// the binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapter {
    /// Adapter kebab name.
    pub name: String,
    /// Pinned adapter version; `None` for an unpinned cache resolve.
    pub version: Option<semver::Version>,
}

/// One `project.yaml.rules` override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Rule brief identifier (the `rules:` key).
    pub brief_id: String,
    /// Repo-relative override path.
    pub path: String,
}

/// One materialized registry workspace slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    /// Registry project name.
    pub name: String,
    /// Repo-relative slot path (`workspace/<project>/`).
    pub path: String,
}

/// One registry peer dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dep {
    /// Registry project name.
    pub name: String,
    /// Peer's recorded adapter value.
    pub adapter: String,
    /// Peer's registry URL.
    pub url: String,
    /// Peer's registry description, when set.
    pub description: Option<String>,
}

/// Render a complete fenced `AGENTS.md` document carrying the computed
/// `fingerprint`.
#[must_use]
pub fn render_document(input: &Input, fingerprint: &str) -> String {
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
        let name = one_line(&adapter.name);
        bullets.push(adapter.version.as_ref().map_or_else(
            || format!("adapter `{name}`."),
            |version| format!("adapter `{name}` {version}."),
        ));
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
        "During execute/build/merge, agents consume Specify and adapters — they do not maintain them."
            .to_string(),
        "On scaffold, verify, finalize, or toolchain failure: stop, print CLI `stop:` / `hint:` / `resume:` output, and exit; never patch `specify`, `specify-adapters`, templates, `adapter.wasm`, or `~/.specify/{store,cache}/**` in-band."
            .to_string(),
        "`metadata.yaml` files are framework-managed; update them through `specify slice` commands."
            .to_string(),
        "`plan.yaml` is framework-managed; write entries through `specify plan add` / `amend`, lifecycle through `specify plan transition`, and close-out through `specify plan archive` — never hand-edit it."
            .to_string(),
        "`.specify/archive/` is framework-managed history.".to_string(),
        "`project.yaml` is the source of truth for Specify project metadata.".to_string(),
    ];
    if let Some(adapter) = &input.adapter {
        bullets
            .push(format!("adapter `{}` owns generated artifact layout.", one_line(&adapter.name)));
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
