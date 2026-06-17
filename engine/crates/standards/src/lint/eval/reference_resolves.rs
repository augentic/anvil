//! `kind: reference-resolves` evaluator.
//!
//! Asserts that every reference of the requested kind that originates
//! from a candidate file resolves to a real path on disk. Two source
//! discriminators are supported:
//!
//! - `markdown-link` consumes the [`crate::lint::MarkdownLink`] facts
//!   the indexer produced and whose `resolves` flag the umbrella
//!   sequential pass populates by joining each link target against
//!   `from_path`'s parent (see [`crate::lint::index::build`]
//!   `resolve_link`). The interpreter emits one
//!   [`specify_diagnostics::Diagnostic`] per `resolves == Some(false)`
//!   link, with a 1-indexed `line` location and the raw target captured
//!   in [`specify_diagnostics::FindingEvidence::Snippet`].
//! - `symlink` consumes the [`crate::lint::Symlink`] facts and emits one
//!   finding per `broken == true` link. Broken symlinks are not file
//!   facts, so they are scoped by the `path-prefix` config value rather
//!   than the `path-pattern` candidate set.
//!
//! URL-style targets (`https://…`, `mailto://…`, anchor-only `#frag`,
//! etc.) leave `resolves` unset upstream and are silently skipped here
//! — the interpreter only fires on references the resolver attempted
//! and rejected.
//!
//! All policy values — which link variant to inspect, a required target
//! prefix or suffix, the symlink path scope — live in the rule's
//! `config:` per the standards-layer policy-in-`specify` rule; this
//! interpreter embeds none of them. Unknown discriminators are rejected
//! as [`super::HintError::Unsupported`] so authoring drift surfaces at
//! hint-evaluation time rather than silently passing.

use std::path::PathBuf;

use serde::Deserialize;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};

use super::{HintError, make_finding};
use crate::lint::WorkspaceModel;
use crate::rules::{HintKind, ResolvedRule, RuleHint};

const SOURCE_MARKDOWN_LINK: &str = "markdown-link";
const SOURCE_SYMLINK: &str = "symlink";

/// Parsed `reference-resolves` hint configuration. All fields are
/// policy supplied by the rule file; absent config means "every
/// resolvable plain link in the candidate set".
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ReferenceResolvesConfig {
    /// `markdown-link`: inspect image embeds (`true`) or plain links
    /// (`false`, the default). Mirrors [`crate::lint::MarkdownLink::image`].
    #[serde(default)]
    image: bool,
    /// `markdown-link`: only consider targets whose path part ends with
    /// this literal (e.g. `.svg`).
    #[serde(default)]
    target_suffix: Option<String>,
    /// `markdown-link`: only consider targets whose path part starts
    /// with one of these literals (e.g. `references/`, `examples/`).
    #[serde(default)]
    target_prefixes: Vec<String>,
    /// `symlink`: only consider symlinks whose path starts with this
    /// literal (e.g. `plugins/`).
    #[serde(default)]
    path_prefix: Option<String>,
}

impl ReferenceResolvesConfig {
    fn parse(rule: &ResolvedRule, hint: &RuleHint) -> Result<Self, HintError> {
        let Some(raw) = hint.config.as_ref() else {
            return Ok(Self::default());
        };
        serde_json::from_value(raw.clone()).map_err(|_ignored| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::ReferenceResolves,
            reason: "invalid reference-resolves hint config JSON",
        })
    }
}

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, candidates: &[PathBuf], model: &WorkspaceModel,
    next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let cfg = ReferenceResolvesConfig::parse(rule, hint)?;
    match hint.value.trim() {
        SOURCE_MARKDOWN_LINK => Ok(markdown_links(rule, candidates, model, &cfg, next_id)),
        SOURCE_SYMLINK => Ok(symlinks(rule, model, &cfg, next_id)),
        _ => Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::ReferenceResolves,
            reason: "only `markdown-link` and `symlink` are supported in v1",
        }),
    }
}

fn markdown_links(
    rule: &ResolvedRule, candidates: &[PathBuf], model: &WorkspaceModel,
    cfg: &ReferenceResolvesConfig, next_id: &mut u64,
) -> Vec<Diagnostic> {
    let candidate_set = super::candidate_set(candidates);

    let mut out: Vec<Diagnostic> = Vec::new();
    for link in &model.markdown_links {
        if !candidate_set.contains(&link.from_path) {
            continue;
        }
        if link.image != cfg.image {
            continue;
        }
        if link.resolves != Some(false) {
            continue;
        }
        let path_part = link.to_raw.split(['#', '?']).next().unwrap_or(&link.to_raw);
        if let Some(suffix) = cfg.target_suffix.as_deref()
            && !path_part.ends_with(suffix)
        {
            continue;
        }
        if !cfg.target_prefixes.is_empty()
            && !cfg.target_prefixes.iter().any(|prefix| path_part.starts_with(prefix))
        {
            continue;
        }
        let location = FindingLocation {
            path: link.from_path.clone(),
            line: Some(link.line),
            column: None,
            end_line: None,
            end_column: None,
        };
        let evidence = FindingEvidence::Snippet {
            value: link.to_raw.clone(),
        };
        let title = format!("{}: unresolved reference `{}`", rule.title, link.to_raw);
        out.push(make_finding(rule, *next_id, title, Some(location), evidence));
        *next_id += 1;
    }
    out
}

fn symlinks(
    rule: &ResolvedRule, model: &WorkspaceModel, cfg: &ReferenceResolvesConfig, next_id: &mut u64,
) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();
    for symlink in &model.symlinks {
        if !symlink.broken {
            continue;
        }
        if let Some(prefix) = cfg.path_prefix.as_deref()
            && !symlink.path.starts_with(prefix)
        {
            continue;
        }
        let location = FindingLocation {
            path: symlink.path.clone(),
            line: Some(1),
            column: None,
            end_line: None,
            end_column: None,
        };
        let evidence = FindingEvidence::Snippet {
            value: symlink.target.clone(),
        };
        let title = format!("{}: broken symlink `{}`", rule.title, symlink.target);
        out.push(make_finding(rule, *next_id, title, Some(location), evidence));
        *next_id += 1;
    }
    out
}

#[cfg(test)]
mod unit {
    use serde_json::json;

    use super::*;
    use crate::lint::eval::testkit::{candidates, empty_model, hint, hint_with_config, rule};
    use crate::lint::{MarkdownLink, Symlink};

    fn link(from: &str, to: &str, resolves: Option<bool>, image: bool) -> MarkdownLink {
        MarkdownLink {
            from_path: from.to_string(),
            to_raw: to.to_string(),
            line: 3,
            resolves,
            image,
        }
    }

    fn symlink(path: &str, target: &str, broken: bool) -> Symlink {
        Symlink {
            path: path.to_string(),
            target: target.to_string(),
            broken,
            resolved_target: None,
        }
    }

    #[test]
    fn only_failed_resolutions_fire() {
        let mut model = empty_model();
        model.markdown_links = vec![
            link("docs/a.md", "missing.md", Some(false), false),
            link("docs/a.md", "present.md", Some(true), false),
            link("docs/a.md", "https://example.com", None, false),
        ];
        let cands = candidates(&["docs/a.md"]);
        let hint = hint(HintKind::ReferenceResolves, "markdown-link");
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("missing.md"), "{}", out[0].title);
    }

    #[test]
    fn image_and_target_filters_narrow() {
        let mut model = empty_model();
        model.markdown_links = vec![
            link("docs/a.md", "img/missing.svg", Some(false), true),
            link("docs/a.md", "img/missing.png", Some(false), true),
            link("docs/a.md", "plain-missing.svg", Some(false), false),
        ];
        let cands = candidates(&["docs/a.md"]);
        let cfg = json!({ "image": true, "target-suffix": ".svg", "target-prefixes": ["img/"] });
        let hint = hint_with_config(HintKind::ReferenceResolves, "markdown-link", Some(cfg));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("img/missing.svg"), "{}", out[0].title);
    }

    #[test]
    fn broken_symlinks_scoped_by_prefix() {
        let mut model = empty_model();
        model.symlinks = vec![
            symlink("plugins/spec/refs/x.md", "../gone.md", true),
            symlink("plugins/spec/refs/y.md", "../there.md", false),
            symlink("docs/z.md", "../gone-too.md", true),
        ];
        let cfg = json!({ "path-prefix": "plugins/" });
        let hint = hint_with_config(HintKind::ReferenceResolves, "symlink", Some(cfg));
        let out = evaluate(&rule(), &hint, &[], &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("../gone.md"), "{}", out[0].title);
    }

    #[test]
    fn unknown_source_is_unsupported() {
        let model = empty_model();
        let hint = hint(HintKind::ReferenceResolves, "no-such-source");
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }

    #[test]
    fn invalid_config_is_unsupported() {
        let model = empty_model();
        let hint = hint_with_config(
            HintKind::ReferenceResolves,
            "markdown-link",
            Some(json!({ "bogus": 1 })),
        );
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }
}
