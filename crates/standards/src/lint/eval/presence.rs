//! `kind: presence` evaluator.
//!
//! Flags the absence of a required artifact. `hint.value` selects one
//! of three mechanism selectors:
//!
//! - `frontmatter` — flag each candidate file (the rule's
//!   `path-pattern` set) that is absent from [`WorkspaceModel::frontmatter`]
//!   or whose frontmatter parsed to an empty field map (missing,
//!   unparseable, or empty frontmatter). For CORE-042.
//! - `file` — `config: { path }`; flag the single required `path`
//!   when it is absent from [`WorkspaceModel::files`]. Whole-tree (the
//!   `path-pattern` candidate set is a sentinel and unused). For
//!   CORE-011.
//! - `directory-index` — `config: { roots, index, min-files? }`; over
//!   the directory prefixes of [`WorkspaceModel::files`], flag each
//!   directory matching a `roots` glob (`*` does not cross `/`) that
//!   holds at least `min-files` files beneath it but no `index` file
//!   directly inside it. Whole-tree. For CORE-059 (reference-corpus
//!   context-budget indexes).
//!
//! All policy (the required path, the corpus roots and index name)
//! rides the rule's `config:`; this arm names only mechanism — the
//! selector tokens. Unknown selectors or a missing required config
//! field are rejected as [`super::HintError::Unsupported`] so
//! authoring drift surfaces at hint-evaluation time rather than
//! silently passing.

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
const VALUE_DIRECTORY_INDEX: &str = "directory-index";

/// Parsed `presence` hint configuration. Every field is optional at
/// parse time; each selector validates the fields it needs and rejects
/// the rest. The shape is schema-gated upstream by `presenceHintConfig`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct PresenceConfig {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    roots: Option<Vec<String>>,
    #[serde(default)]
    index: Option<String>,
    #[serde(default)]
    min_files: Option<usize>,
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
        VALUE_DIRECTORY_INDEX => evaluate_directory_index(rule, hint, model, next_id),
        _ => Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Presence,
            reason: "only `frontmatter`, `file`, or `directory-index` is supported in v1",
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
    use crate::lint::Frontmatter;
    use crate::lint::eval::testkit::{
        candidates, empty_model, hint, hint_with_config, model_with_paths, rule,
    };

    fn frontmatter(path: &str, fields: serde_json::Map<String, serde_json::Value>) -> Frontmatter {
        Frontmatter {
            path: path.to_string(),
            schema_id: None,
            fields,
        }
    }

    // The CLI e2e defers per-`kind` eval semantics, so the three presence
    // selectors collapse into one finding matrix and the unsupported-config
    // rejections into a second test. Every former input is preserved.

    #[test]
    fn selectors_flag_absences() {
        // `frontmatter`: candidates absent or empty in the model are flagged.
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

        // `file`: the required `config: { path }` is flagged when absent,
        // silent when present.
        let cfg = json!({ "path": "AGENTS.md" });
        let model = model_with_paths(&["README.md"]);
        let hint = hint_with_config(HintKind::Presence, "file", Some(cfg.clone()));
        let out = evaluate(&rule(), &hint, &[], &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("'AGENTS.md'"), "{}", out[0].title);
        let model = model_with_paths(&["AGENTS.md"]);
        let hint = hint_with_config(HintKind::Presence, "file", Some(cfg));
        assert!(evaluate(&rule(), &hint, &[], &model, &mut 1).expect("evaluate").is_empty());

        // `directory-index`: a roots-matching directory over `min-files`
        // without its index file is flagged.
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
    fn rejects_unsupported_config() {
        let model = empty_model();
        // A `file` selector with no `config: { path }` is rejected.
        let hint = hint_with_config(HintKind::Presence, "file", Some(json!({})));
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();

        // An unknown selector token is rejected.
        let hint = hint_with_config(HintKind::Presence, "no-such-selector", None);
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }
}
