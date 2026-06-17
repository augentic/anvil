//! `kind: path-pattern` evaluator per the executable hint-kind contract.
//!
//! Path-pattern hints are filters, not finders. They narrow the
//! candidate file set the later hint kinds (`schema`, `regex`,
//! `tool`) consume; they emit zero findings on their own. Glob
//! semantics follow [`globset`] with `literal_separator` on — `*`
//! never crosses `/`, `**` does, and `{a,b}` brace alternation lets
//! one hint carry what used to take a fan-out of near-identical
//! patterns. The pattern matches a file's project-relative path
//! verbatim — no separator translation, no per-OS munging, since
//! [`crate::lint::File::path`] is already forward-slash relative per
//! `WorkspaceModel` stability.
//!
//! Include patterns match paths into the candidate set. Exclusion
//! patterns use a leading `!` on `value`; the
//! the eval umbrella unions includes then subtracts excludes.

use std::path::PathBuf;

use globset::GlobBuilder;

use super::HintError;
use crate::lint::WorkspaceModel;
use crate::rules::{HintKind, ResolvedRule, RuleHint};

/// Whether `value` is an exclusion glob (`!` prefix).
#[must_use]
pub(crate) fn is_exclusion(hint: &RuleHint) -> bool {
    hint.value.starts_with('!')
}

/// Glob text after stripping a leading `!`, if present.
fn glob_text(value: &str) -> &str {
    value.strip_prefix('!').unwrap_or(value)
}

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, model: &WorkspaceModel,
) -> Result<Vec<PathBuf>, HintError> {
    let glob = glob_text(&hint.value);
    let matcher = GlobBuilder::new(glob)
        .literal_separator(true)
        .build()
        .map_err(|_silenced| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::PathPattern,
            reason: "invalid glob pattern",
        })?
        .compile_matcher();
    let mut hits: Vec<PathBuf> = model
        .files
        .iter()
        .filter(|f| matcher.is_match(&f.path))
        .map(|f| PathBuf::from(&f.path))
        .collect();
    hits.sort();
    Ok(hits)
}

#[cfg(test)]
mod unit {
    use super::*;
    use crate::lint::eval::testkit::{hint, model_with_paths, rule};
    use crate::rules::HintKind;

    fn path_hint(value: &str) -> RuleHint {
        hint(HintKind::PathPattern, value)
    }

    #[test]
    fn exclusion_prefix_matches_paths() {
        let model = model_with_paths(&["docs/a.md", "docs/explanation/decision-log.md"]);
        let hint = path_hint("!docs/explanation/decision-log.md");
        let matched = evaluate(&rule(), &hint, &model).expect("evaluate");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].to_string_lossy(), "docs/explanation/decision-log.md");
    }

    #[test]
    fn include_pattern_unchanged() {
        let model = model_with_paths(&["a.rs", "b.md"]);
        let hint = path_hint("*.rs");
        let matched = evaluate(&rule(), &hint, &model).expect("evaluate");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].to_string_lossy(), "a.rs");
    }

    #[test]
    fn invalid_glob_is_unsupported() {
        let model = model_with_paths(&["a.rs"]);
        let hint = path_hint("[");
        evaluate(&rule(), &hint, &model).unwrap_err();
    }

    #[test]
    fn star_does_not_cross_separators() {
        let model = model_with_paths(&["docs/a.md", "docs/sub/b.md"]);
        let hint = path_hint("docs/*.md");
        let matched = evaluate(&rule(), &hint, &model).expect("evaluate");
        assert_eq!(matched.len(), 1, "`*` must not cross `/`; got {matched:?}");
        assert_eq!(matched[0].to_string_lossy(), "docs/a.md");
    }

    #[test]
    fn brace_alternation_unions_patterns() {
        let model = model_with_paths(&[
            "docs/a.md",
            "plugins/spec/skills/refine/SKILL.md",
            "src/lib.rs",
            "AGENTS.md",
        ]);
        let hint = path_hint("{docs/**/*.md,plugins/**/*.md,**/AGENTS.md}");
        let matched = evaluate(&rule(), &hint, &model).expect("evaluate");
        let paths: Vec<String> = matched.iter().map(|p| p.to_string_lossy().into_owned()).collect();
        assert_eq!(paths, ["AGENTS.md", "docs/a.md", "plugins/spec/skills/refine/SKILL.md"]);
    }
}
