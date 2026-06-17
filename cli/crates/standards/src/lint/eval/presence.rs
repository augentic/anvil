//! `kind: presence` evaluator.
//!
//! Flags the absence of a required artifact. `hint.value` selects one
//! of four mechanism selectors:
//!
//! - `frontmatter` — flag each candidate file (the rule's
//!   `path-pattern` set) that is absent from [`WorkspaceModel::frontmatter`]
//!   or whose frontmatter parsed to an empty field map (missing,
//!   unparseable, or empty frontmatter). For CORE-042.
//! - `file` — `config: { path }`; flag the single required `path`
//!   when it is absent from [`WorkspaceModel::files`]. Whole-tree (the
//!   `path-pattern` candidate set is a sentinel and unused). For
//!   CORE-011.
//! - `markdown-section` — `config: { title, level, when: { metric, min } }`;
//!   over the [`crate::lint::Skill`] facts whose `metric` reaches
//!   `min`, flag those lacking a [`crate::lint::MarkdownSection`] with
//!   the configured `title` and `level`. Whole-tree (the `path-pattern`
//!   candidate set is a sentinel and unused). For CORE-041.
//! - `directory-index` — `config: { roots, index, min-files? }`; over
//!   the directory prefixes of [`WorkspaceModel::files`], flag each
//!   directory matching a `roots` glob (`*` does not cross `/`) that
//!   holds at least `min-files` files beneath it but no `index` file
//!   directly inside it. Whole-tree. For CORE-059 (reference-corpus
//!   context-budget indexes).
//!
//! All policy (the required path, the section title / level, the metric
//! threshold, the corpus roots and index name) rides the rule's
//! `config:`; this arm names only mechanism — the selector tokens and
//! the single supported metric. Unknown selectors, an unsupported
//! metric, or a missing required config field are rejected as
//! [`super::HintError::Unsupported`] so authoring drift surfaces at
//! hint-evaluation time rather than silently passing.

use std::collections::BTreeSet;
use std::path::PathBuf;

use glob::{MatchOptions, Pattern};
use serde::Deserialize;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};

use super::{HintError, make_finding};
use crate::lint::WorkspaceModel;
use crate::rules::{HintKind, ResolvedRule, RuleHint};

const VALUE_FRONTMATTER: &str = "frontmatter";
const VALUE_FILE: &str = "file";
const VALUE_MARKDOWN_SECTION: &str = "markdown-section";
const VALUE_DIRECTORY_INDEX: &str = "directory-index";
/// The single fact metric the `markdown-section` selector gates on
/// today; naming a fact metric is mechanism, the threshold is policy.
const METRIC_SKILL_BODY_LINE_COUNT: &str = "skill-body-line-count";

/// Parsed `presence` hint configuration. Every field is optional at
/// parse time; each selector validates the fields it needs and rejects
/// the rest. The shape is schema-gated upstream by `presenceHintConfig`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct PresenceConfig {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    level: Option<u8>,
    #[serde(default)]
    when: Option<PresenceWhen>,
    #[serde(default)]
    roots: Option<Vec<String>>,
    #[serde(default)]
    index: Option<String>,
    #[serde(default)]
    min_files: Option<usize>,
}

/// The `when: { metric, min }` threshold gate for the
/// `markdown-section` selector.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct PresenceWhen {
    metric: String,
    min: u32,
}

impl PresenceConfig {
    fn parse(rule: &ResolvedRule, hint: &RuleHint) -> Result<Self, HintError> {
        let raw = hint.config.as_ref().ok_or_else(|| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Presence,
            reason: "this `presence` selector requires a `config`",
        })?;
        serde_json::from_value(raw.clone()).map_err(|_ignored| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Presence,
            reason: "invalid presence hint config JSON",
        })
    }
}

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, candidates: &[PathBuf], model: &WorkspaceModel,
    next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    match hint.value.trim() {
        VALUE_FRONTMATTER => Ok(evaluate_frontmatter(rule, candidates, model, next_id)),
        VALUE_FILE => evaluate_file(rule, hint, model, next_id),
        VALUE_MARKDOWN_SECTION => evaluate_markdown_section(rule, hint, model, next_id),
        VALUE_DIRECTORY_INDEX => evaluate_directory_index(rule, hint, model, next_id),
        _ => Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Presence,
            reason: "only `frontmatter`, `file`, `markdown-section`, or `directory-index` is supported in v1",
        }),
    }
}

/// `frontmatter` selector: each candidate file lacking a non-empty
/// frontmatter fact (absent, unparseable, or an empty field map) is
/// flagged. Narrowed by the `path-pattern` candidate set.
fn evaluate_frontmatter(
    rule: &ResolvedRule, candidates: &[PathBuf], model: &WorkspaceModel, next_id: &mut u64,
) -> Vec<Diagnostic> {
    let present: BTreeSet<&str> = model
        .frontmatter
        .iter()
        .filter(|fm| !fm.fields.is_empty())
        .map(|fm| fm.path.as_str())
        .collect();
    let mut out: Vec<Diagnostic> = Vec::new();
    for candidate in super::candidate_set(candidates) {
        if present.contains(candidate.as_str()) {
            continue;
        }
        let summary = format!("missing or empty frontmatter in '{candidate}'");
        let finding = mint(rule, &candidate, &summary, next_id);
        out.push(finding);
    }
    out
}

/// `file` selector: flag the single required `config: { path }` when no
/// [`crate::lint::File`] fact carries that path. Whole-tree.
fn evaluate_file(
    rule: &ResolvedRule, hint: &RuleHint, model: &WorkspaceModel, next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let cfg = PresenceConfig::parse(rule, hint)?;
    let path = cfg.path.ok_or_else(|| HintError::Unsupported {
        rule_id: rule.rule_id.clone(),
        kind: HintKind::Presence,
        reason: "`file` requires a `config: { path }`",
    })?;
    if model.files.iter().any(|file| file.path == path) {
        return Ok(Vec::new());
    }
    let summary = format!("required file '{path}' is missing");
    Ok(vec![mint(rule, &path, &summary, next_id)])
}

/// `markdown-section` selector: over the skill fact family, flag each
/// skill whose `metric` reaches `min` but carries no markdown section
/// with the configured `title` and `level`. Whole-tree.
fn evaluate_markdown_section(
    rule: &ResolvedRule, hint: &RuleHint, model: &WorkspaceModel, next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let cfg = PresenceConfig::parse(rule, hint)?;
    let (Some(title), Some(level), Some(when)) = (cfg.title, cfg.level, cfg.when) else {
        return Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Presence,
            reason: "`markdown-section` requires `config: { title, level, when: { metric, min } }`",
        });
    };
    if when.metric != METRIC_SKILL_BODY_LINE_COUNT {
        return Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Presence,
            reason: "only the `skill-body-line-count` metric is supported in v1",
        });
    }
    let mut out: Vec<Diagnostic> = Vec::new();
    for skill in &model.skills {
        if skill.body_line_count.unwrap_or(0) < when.min {
            continue;
        }
        let has_section = model.markdown_sections.iter().any(|section| {
            section.path == skill.path && section.level == level && section.title == title
        });
        if has_section {
            continue;
        }
        let summary = format!("missing required '{title}' section (level {level})");
        out.push(mint(rule, &skill.path, &summary, next_id));
    }
    Ok(out)
}

/// `directory-index` selector: each directory prefix of a file fact
/// that matches a `roots` glob and holds at least `min-files` files
/// beneath it (recursive) must contain the `index` file directly. Glob
/// matching keeps `/` literal so a root pattern names one directory
/// depth. Whole-tree.
fn evaluate_directory_index(
    rule: &ResolvedRule, hint: &RuleHint, model: &WorkspaceModel, next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let cfg = PresenceConfig::parse(rule, hint)?;
    let (Some(roots), Some(index)) = (cfg.roots, cfg.index) else {
        return Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Presence,
            reason: "`directory-index` requires `config: { roots, index }`",
        });
    };
    let min_files = cfg.min_files.unwrap_or(1);
    let patterns: Vec<Pattern> =
        roots.iter().map(|root| Pattern::new(root)).collect::<Result<_, _>>().map_err(
            |_silenced| HintError::Unsupported {
                rule_id: rule.rule_id.clone(),
                kind: HintKind::Presence,
                reason: "invalid glob pattern in `roots`",
            },
        )?;
    let options = MatchOptions {
        require_literal_separator: true,
        ..MatchOptions::default()
    };

    let mut dirs: BTreeSet<&str> = BTreeSet::new();
    for file in &model.files {
        let mut prefix = file.path.as_str();
        while let Some(pos) = prefix.rfind('/') {
            prefix = &prefix[..pos];
            if patterns.iter().any(|pattern| pattern.matches_with(prefix, options)) {
                dirs.insert(prefix);
            }
        }
    }

    let mut out: Vec<Diagnostic> = Vec::new();
    for dir in dirs {
        let beneath = format!("{dir}/");
        let count = model.files.iter().filter(|file| file.path.starts_with(&beneath)).count();
        if count < min_files {
            continue;
        }
        let required = format!("{dir}/{index}");
        if model.files.iter().any(|file| file.path == required) {
            continue;
        }
        let summary =
            format!("reference directory '{dir}' ({count} files) is missing its '{index}' index");
        out.push(mint(rule, dir, &summary, next_id));
    }
    Ok(out)
}

/// Mint one presence finding located at `path`, with structured
/// evidence carrying the offending path, and bump the id counter.
fn mint(rule: &ResolvedRule, path: &str, summary: &str, next_id: &mut u64) -> Diagnostic {
    let location = FindingLocation {
        path: path.to_owned(),
        line: None,
        column: None,
        end_line: None,
        end_column: None,
    };
    let evidence = FindingEvidence::Structured {
        summary: summary.to_owned(),
        data: serde_json::json!({ "path": path }),
        locations: None,
    };
    let title = format!("{}: {summary}", rule.title);
    let finding = make_finding(rule, *next_id, title, Some(location), evidence);
    *next_id += 1;
    finding
}

#[cfg(test)]
mod unit {
    use serde_json::json;

    use super::*;
    use crate::lint::eval::testkit::{
        candidates, empty_model, hint, hint_with_config, model_with_paths, rule,
    };
    use crate::lint::{Frontmatter, MarkdownSection, Skill};

    fn frontmatter(path: &str, fields: serde_json::Map<String, serde_json::Value>) -> Frontmatter {
        Frontmatter {
            path: path.to_string(),
            schema_id: None,
            fields,
        }
    }

    #[test]
    fn missing_or_empty_frontmatter_flagged() {
        let mut model = empty_model();
        let mut fields = serde_json::Map::new();
        fields.insert("name".to_string(), json!("x"));
        model.frontmatter = vec![
            frontmatter("docs/full.md", fields),
            frontmatter("docs/empty.md", serde_json::Map::new()),
        ];
        let cands = candidates(&["docs/full.md", "docs/empty.md", "docs/none.md"]);
        let hint = hint(HintKind::Presence, "frontmatter");
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        let paths: Vec<&str> =
            out.iter().filter_map(|f| f.location.as_ref().map(|l| l.path.as_str())).collect();
        assert_eq!(paths, vec!["docs/empty.md", "docs/none.md"]);
    }

    #[test]
    fn required_file_absence_flagged() {
        let model = model_with_paths(&["README.md"]);
        let cfg = json!({ "path": "AGENTS.md" });
        let hint = hint_with_config(HintKind::Presence, "file", Some(cfg.clone()));
        let out = evaluate(&rule(), &hint, &[], &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("'AGENTS.md'"), "{}", out[0].title);

        let model = model_with_paths(&["AGENTS.md"]);
        let hint = hint_with_config(HintKind::Presence, "file", Some(cfg));
        let out = evaluate(&rule(), &hint, &[], &model, &mut 1).expect("evaluate");
        assert!(out.is_empty());
    }

    #[test]
    fn section_required_above_metric_threshold() {
        let mut model = empty_model();
        let big = "plugins/p/skills/big/SKILL.md";
        let small = "plugins/p/skills/small/SKILL.md";
        model.skills = vec![
            Skill {
                name: "big".to_string(),
                path: big.to_string(),
                plugin: "p".to_string(),
                frontmatter_ref: big.to_string(),
                body_line_count: Some(100),
            },
            Skill {
                name: "small".to_string(),
                path: small.to_string(),
                plugin: "p".to_string(),
                frontmatter_ref: small.to_string(),
                body_line_count: Some(5),
            },
        ];
        model.markdown_sections = vec![MarkdownSection {
            path: small.to_string(),
            level: 2,
            title: "Critical Path".to_string(),
            line_start: 3,
            line_end: 9,
            body_line_count: 6,
        }];
        let cfg = json!({
            "title": "Critical Path",
            "level": 2,
            "when": { "metric": "skill-body-line-count", "min": 50 },
        });
        let hint = hint_with_config(HintKind::Presence, "markdown-section", Some(cfg));
        let out = evaluate(&rule(), &hint, &[], &model, &mut 1).expect("evaluate");
        // Only the big skill crosses the threshold, and it lacks the section.
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].location.as_ref().map(|l| l.path.as_str()), Some(big));
    }

    #[test]
    fn directory_missing_index_flagged() {
        let model = model_with_paths(&[
            "refs/corpus/a.md",
            "refs/corpus/b.md",
            "refs/indexed/INDEX.md",
            "refs/indexed/a.md",
        ]);
        let cfg = json!({ "roots": ["refs/*"], "index": "INDEX.md", "min-files": 2 });
        let hint = hint_with_config(HintKind::Presence, "directory-index", Some(cfg));
        let out = evaluate(&rule(), &hint, &[], &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("'refs/corpus'"), "{}", out[0].title);
    }

    #[test]
    fn unsupported_metric_rejected() {
        let model = empty_model();
        let cfg = json!({
            "title": "T",
            "level": 2,
            "when": { "metric": "no-such-metric", "min": 1 },
        });
        let hint = hint_with_config(HintKind::Presence, "markdown-section", Some(cfg));
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }

    #[test]
    fn unknown_selector_is_unsupported() {
        let model = empty_model();
        let hint = hint(HintKind::Presence, "no-such-selector");
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }
}
