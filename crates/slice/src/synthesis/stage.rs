//! The staged synthesis tree (RFC-96 D10): read the change-artifact
//! bundle the agent wrote into the lent workspace — the persisted
//! slice-bundle layout — failing typed for the repair loop.

use std::path::Path;

use artifacts::decision::{DecisionStatus, parse_decision, split_frontmatter};
use error::{Error, Result};

use crate::model::SliceModel;
use crate::synthesis::wire::{SynthesisArtifacts, SynthesisDecision, SynthesisSpec};

/// The validated staged bundle: the parsed structured model plus the
/// prose artifacts, in the same currency the persist tail consumes.
#[derive(Debug)]
pub struct StagedBundle {
    /// The agent's structured model, parsed from `model.yaml`.
    pub model: SliceModel,
    /// The prose-only artifact bodies.
    pub artifacts: SynthesisArtifacts,
}

/// Read and validate the staged tree at `root`.
///
/// # Errors
///
/// - `slice-synthesize-stage-missing` — a required bundle file is
///   absent or empty.
/// - `slice-synthesize-stage-model` — `model.yaml` fails the typed
///   parse.
/// - `slice-synthesize-stage-decision` — a `decisions/<slug>.md` fails
///   the Decision Record parse or grammar.
pub fn read(root: &Path) -> Result<StagedBundle> {
    let model_text = required(root, "model.yaml")?;
    let model = SliceModel::parse_yaml(&model_text).map_err(|err| {
        Error::validation_failed(
            "slice-synthesize-stage-model",
            "the staged model.yaml parses as a slice model",
            format!("model.yaml: {err}"),
        )
    })?;
    let artifacts = SynthesisArtifacts {
        proposal: required(root, "proposal.md")?,
        design: required(root, "design.md")?,
        tasks: required(root, "tasks.md")?,
        specs: specs(root)?,
        decisions: decisions(root)?,
    };
    Ok(StagedBundle { model, artifacts })
}

/// Read one required bundle file, failing typed when it is absent or
/// blank.
fn required(root: &Path, rel: &str) -> Result<String> {
    let path = root.join(rel);
    let text = std::fs::read_to_string(&path).map_err(|_err| missing(rel, "file is missing"))?;
    if text.trim().is_empty() {
        return Err(missing(rel, "file is empty"));
    }
    Ok(text)
}

fn missing(rel: &str, why: &str) -> Error {
    Error::validation_failed(
        "slice-synthesize-stage-missing",
        "the staged tree carries the full change-artifact bundle",
        format!("`{rel}`: {why}; write it into the lent workspace"),
    )
}

/// Read every `specs/<domain>/spec.md`, in domain order. At least one
/// spec domain is required on `proceed`.
fn specs(root: &Path) -> Result<Vec<SynthesisSpec>> {
    let dir = root.join("specs");
    let mut domains: Vec<String> = std::fs::read_dir(&dir).map_or_else(
        |_err| Vec::new(),
        |entries| {
            entries
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    entry
                        .file_type()
                        .ok()?
                        .is_dir()
                        .then(|| entry.file_name().to_string_lossy().into_owned())
                })
                .collect()
        },
    );
    domains.sort();
    let specs: Vec<SynthesisSpec> = domains
        .into_iter()
        .filter_map(|domain| {
            let content = std::fs::read_to_string(dir.join(&domain).join("spec.md")).ok()?;
            (!content.trim().is_empty()).then_some(SynthesisSpec { domain, content })
        })
        .collect();
    if specs.is_empty() {
        return Err(missing("specs/<domain>/spec.md", "no spec domain found"));
    }
    Ok(specs)
}

/// Read every staged `decisions/<slug>.md` (slug order) back into the
/// typed decision currency; the persist tail re-renders them, so
/// engine-stamped fields the agent supplied are normalised away.
fn decisions(root: &Path) -> Result<Vec<SynthesisDecision>> {
    let dir = root.join("decisions");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = project::decisions::list_md_files(&dir)?;
    paths.sort();
    paths.iter().map(|path| decision(path)).collect()
}

fn decision(path: &Path) -> Result<SynthesisDecision> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let fail = |detail: String| {
        Error::validation_failed(
            "slice-synthesize-stage-decision",
            "each staged decisions/<slug>.md is a well-formed Decision Record",
            format!("`decisions/{name}`: {detail}"),
        )
    };
    let text = std::fs::read_to_string(path).map_err(|err| fail(err.to_string()))?;
    let parsed = parse_decision(&text);
    if let Some(finding) = parsed.findings.first() {
        return Err(fail(finding.detail.clone()));
    }
    let record = parsed.record.ok_or_else(|| fail("missing front-matter".to_string()))?;
    let title = parsed.title.ok_or_else(|| fail("missing `# <title>` heading".to_string()))?;
    let body = split_frontmatter(&text).map_or("", |(_, body)| body);
    Ok(SynthesisDecision {
        slug: record.slug,
        status: match record.status {
            DecisionStatus::Rejected => DecisionStatus::Rejected,
            // `superseded` is engine-only; normalise like the persist
            // tail does.
            DecisionStatus::Accepted | DecisionStatus::Superseded => DecisionStatus::Accepted,
        },
        title,
        context: section(body, "Context").ok_or_else(|| missing_section(&name, "Context"))?,
        decision: section(body, "Decision").ok_or_else(|| missing_section(&name, "Decision"))?,
        consequences: section(body, "Consequences")
            .ok_or_else(|| missing_section(&name, "Consequences"))?,
        supersedes: record.supersedes,
        related: record.related,
        topics: record.topics,
    })
}

fn missing_section(name: &str, heading: &str) -> Error {
    Error::validation_failed(
        "slice-synthesize-stage-decision",
        "each staged decisions/<slug>.md is a well-formed Decision Record",
        format!("`decisions/{name}`: missing `## {heading}` section"),
    )
}

/// Extract the text between `## <name>` and the next `##` heading.
fn section(body: &str, name: &str) -> Option<String> {
    let mut lines = body.lines();
    lines.by_ref().find(|line| heading_matches(line, name))?;
    let text: Vec<&str> = lines.take_while(|line| !is_h2(line)).collect();
    let text = text.join("\n").trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn heading_matches(line: &str, name: &str) -> bool {
    let trimmed = line.trim();
    let Some(rest) = trimmed.strip_prefix("##") else {
        return false;
    };
    if rest.starts_with('#') {
        return false;
    }
    rest.trim() == name || rest.trim().starts_with(&format!("{name} "))
}

fn is_h2(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.strip_prefix("##").is_some_and(|rest| !rest.starts_with('#'))
}
