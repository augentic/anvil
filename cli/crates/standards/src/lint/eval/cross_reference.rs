//! `kind: cross-reference` evaluator.
//!
//! A generic relational join over a *source* and a *target* fact family
//! (presence-only set difference): `hint.value` names a source fact
//! family; every element must have a corresponding element in the
//! `config: { target }` fact family (joined on a per-family key). Each
//! unmatched source item is flagged. v1 pair: source `adapter-dir` (the
//! [`crate::lint::AdapterDir`] facts, one per immediate child directory
//! under `adapters/{sources,targets}`, keyed by directory `path`) against
//! target `adapter-manifest` (the [`crate::lint::AdapterManifest`] facts,
//! keyed by the directory *containing* each manifest). For CORE-010 — an
//! adapter directory with no resolvable `adapter.yaml`.
//!
//! All policy (which family is source / target) rides the rule's
//! `value` / `config:`; this arm names only mechanism — the closed
//! source / target selector tokens and the structural join key each
//! family contributes (a directory path, computed by the per-selector
//! accessor like every other fact-iterating kind). The evaluator carries
//! no rule id and no `adapters/...` path literal. Unknown selectors are
//! rejected as [`super::HintError::Unsupported`] so authoring drift
//! surfaces at hint-evaluation time rather than silently passing.
//!
//! The join is whole-tree: it reads the fact families directly and ignores
//! the candidate set (the source families — directories — never appear in
//! `model.files`).

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::Deserialize;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};

use super::{HintError, make_finding};
use crate::lint::WorkspaceModel;
use crate::rules::{HintKind, ResolvedRule, RuleHint};

const SOURCE_ADAPTER_DIR: &str = "adapter-dir";
const TARGET_ADAPTER_MANIFEST: &str = "adapter-manifest";

/// Parsed `cross-reference` hint configuration. The target family
/// selector is supplied by the rule; the shape is schema-gated upstream
/// by `crossReferenceHintConfig` (the required `target` key).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct CrossReferenceConfig {
    target: String,
}

impl CrossReferenceConfig {
    fn parse(rule: &ResolvedRule, hint: &RuleHint) -> Result<Self, HintError> {
        let raw = hint.config.as_ref().ok_or_else(|| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::CrossReference,
            reason: "`cross-reference` requires a `config: { target }`",
        })?;
        serde_json::from_value(raw.clone()).map_err(|_ignored| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::CrossReference,
            reason: "invalid cross-reference hint config JSON",
        })
    }
}

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, _candidates: &[PathBuf], model: &WorkspaceModel,
    next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let cfg = CrossReferenceConfig::parse(rule, hint)?;
    let target = cfg.target.trim();
    evaluate_family(rule, hint.value.trim(), target, model, next_id)
}

/// Fact-family presence join: flag every source-family key with no
/// corresponding target-family key.
fn evaluate_family(
    rule: &ResolvedRule, source: &str, target: &str, model: &WorkspaceModel, next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let source_keys = source_keys(rule, source, model)?;
    let target_keys = presence_target_keys(rule, target, model)?;

    let mut out: Vec<Diagnostic> = Vec::new();
    for (key, path) in source_keys {
        if target_keys.contains(&key) {
            continue;
        }
        let summary = format!("'{path}' has no corresponding '{target}' entry");
        out.push(mint(rule, &path, &summary, next_id));
    }
    Ok(out)
}

/// `(join-key, finding-location-path)` pairs for a fact-family source. The
/// join key and the finding location coincide (the directory path).
fn source_keys(
    rule: &ResolvedRule, selector: &str, model: &WorkspaceModel,
) -> Result<Vec<(String, String)>, HintError> {
    match selector {
        SOURCE_ADAPTER_DIR => {
            Ok(model.adapter_dirs.iter().map(|dir| (dir.path.clone(), dir.path.clone())).collect())
        }
        _ => Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::CrossReference,
            reason: "only the `adapter-dir` cross-reference source is supported in v1",
        }),
    }
}

/// The set of join keys present in a presence-join target family. For
/// `adapter-manifest` the key is the directory containing each manifest —
/// the structural counterpart of the `adapter-dir` directory key.
fn presence_target_keys(
    rule: &ResolvedRule, selector: &str, model: &WorkspaceModel,
) -> Result<BTreeSet<String>, HintError> {
    match selector {
        TARGET_ADAPTER_MANIFEST => Ok(model
            .adapter_manifests
            .iter()
            .filter_map(|manifest| containing_dir(&manifest.path))
            .collect()),
        _ => Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::CrossReference,
            reason: "only the `adapter-manifest` presence target is supported in v1",
        }),
    }
}

/// The directory containing a file fact's path (everything before the
/// final `/`), or `None` when the path has no directory component.
fn containing_dir(path: &str) -> Option<String> {
    path.rfind('/').map(|idx| path[..idx].to_owned())
}

/// Mint one cross-reference finding located at `path`, with structured
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
    use crate::lint::eval::testkit::{empty_model, hint_with_config, rule};
    use crate::lint::{AdapterAxis, AdapterDir, AdapterManifest};

    fn dir(path: &str) -> AdapterDir {
        AdapterDir {
            path: path.to_string(),
            axis: AdapterAxis::Targets,
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
        }
    }

    fn manifest(name: &str) -> AdapterManifest {
        AdapterManifest {
            axis: AdapterAxis::Targets,
            name: name.to_string(),
            path: format!("adapters/targets/{name}/adapter.yaml"),
            version: None,
            brief_keys: vec![],
        }
    }

    #[test]
    fn orphan_adapter_dir_flagged() {
        let mut model = empty_model();
        model.adapter_dirs =
            vec![dir("adapters/targets/with-manifest"), dir("adapters/targets/orphan")];
        model.adapter_manifests = vec![manifest("with-manifest")];
        let cfg = json!({ "target": "adapter-manifest" });
        let hint = hint_with_config(HintKind::CrossReference, "adapter-dir", Some(cfg));
        let out = evaluate(&rule(), &hint, &[], &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("adapters/targets/orphan"), "{}", out[0].title);
    }

    #[test]
    fn unknown_selectors_rejected() {
        let model = empty_model();
        let source = hint_with_config(
            HintKind::CrossReference,
            "no-such-source",
            Some(json!({ "target": "adapter-manifest" })),
        );
        evaluate(&rule(), &source, &[], &model, &mut 1).unwrap_err();
        let target = hint_with_config(
            HintKind::CrossReference,
            "adapter-dir",
            Some(json!({ "target": "no-such-target" })),
        );
        evaluate(&rule(), &target, &[], &model, &mut 1).unwrap_err();
    }
}
