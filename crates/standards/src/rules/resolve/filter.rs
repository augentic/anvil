//! Applicability and deprecation filters over the resolver's rule pool.
//!
//! [`filter`] runs three passes in fixed order: origin (`core` rules
//! drop unless `--include-core`), deprecation (drops unless
//! `--include-deprecated`), then applicability — a rule with no
//! `applicability` block always passes; one with a block must match
//! **every populated dimension** (AND semantics). A dimension the rule
//! populates but the caller supplied no input for excludes the rule
//! unless [`ResolveInputs::include_unmatched`] is set. Per-dimension
//! matching semantics live on the `*_dimension_matches` functions
//! below.
//!
//! Call [`super::resolve`] first, then pass its `Vec<ResolvedRuleEntry>`
//! to [`filter`]; `build_resolved_rules` in the sibling `sort` module
//! is the conventional export entry point.

use std::path::{Path, PathBuf};

use glob::{MatchOptions, Pattern};

use super::{ResolveInputs, ResolvedRuleEntry};
use crate::rules::{Origin, Rule};

/// Rule path-glob semantics (`applicability.paths`): case-sensitive,
/// `/` is the only separator, leading dots match literally.
const PATH_MATCH_OPTIONS: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_separator: true,
    require_literal_leading_dot: false,
};

/// Apply origin + deprecation + applicability filters to a
/// [`super::resolve`] result.
///
/// Origin (`core`) runs first; deprecation runs against those
/// survivors; applicability runs against the survivors of both. See
/// the module docs for the closed precedence rules and per-dimension
/// matching semantics.
#[must_use]
pub fn filter(
    entries: Vec<ResolvedRuleEntry>, inputs: &ResolveInputs<'_>,
) -> Vec<ResolvedRuleEntry> {
    entries
        .into_iter()
        .filter(|entry| keeps_core(entry.origin, inputs.include_core))
        .filter(|entry| keeps_deprecated(&entry.rule, inputs.include_deprecated))
        .filter(|entry| applicability_matches(&entry.rule, inputs))
        .collect()
}

/// `true` when the entry survives the consumer-export `core` filter.
///
/// Rules resolved from `codex/rules/core/`
/// (i.e. [`Origin::Core`]) are excluded from the export by default; the
/// caller opts in via `--include-core`.
const fn keeps_core(origin: Origin, include_core: bool) -> bool {
    !matches!(origin, Origin::Core) || include_core
}

/// `true` when the rule survives the deprecation filter.
const fn keeps_deprecated(rule: &Rule, include_deprecated: bool) -> bool {
    rule.deprecated.is_none() || include_deprecated
}

/// `true` when every populated applicability dimension matches.
///
/// A rule with no [`Applicability`] block always passes per the rules contract.
fn applicability_matches(rule: &Rule, inputs: &ResolveInputs<'_>) -> bool {
    let Some(applicability) = rule.applicability.as_ref() else {
        return true;
    };
    adapter_dimension_matches(applicability.adapters.as_deref(), inputs)
        && language_dimension_matches(
            applicability.languages.as_deref(),
            inputs.languages,
            inputs.include_unmatched,
        )
        && artifact_dimension_matches(applicability.artifacts.as_deref(), inputs.include_unmatched)
        && paths_dimension_matches(
            applicability.paths.as_deref(),
            inputs.artifact_paths,
            inputs.include_unmatched,
        )
}

/// Adapter dimension match.
///
/// Returns `true` when the rule does not constrain adapters, or when
/// the rule's adapter list contains the target adapter or any bound
/// source adapter (after stripping the optional `@v<major>` suffix on
/// the rule side).
///
/// The caller's `target_adapter` is always populated on
/// [`ResolveInputs`], so the populated-dimension-without-caller-input
/// branch is unreachable here.
fn adapter_dimension_matches(rule_adapters: Option<&[String]>, inputs: &ResolveInputs<'_>) -> bool {
    let Some(rule_adapters) = rule_adapters else {
        return true;
    };
    rule_adapters.iter().any(|raw| {
        let bare = strip_version_suffix(raw);
        bare == inputs.target_adapter
            || inputs.source_adapters.iter().any(|src| src.as_str() == bare)
    })
}

/// Language dimension match.
///
/// Returns `true` when the rule does not constrain languages. When the
/// rule populates languages but the caller supplied none, the rule is
/// excluded unless `include_unmatched`. Otherwise the rule matches when
/// any caller language appears in the rule's list.
fn language_dimension_matches(
    rule_languages: Option<&[String]>, caller_languages: &[String], include_unmatched: bool,
) -> bool {
    let Some(rule_languages) = rule_languages else {
        return true;
    };
    if caller_languages.is_empty() {
        return include_unmatched;
    }
    caller_languages.iter().any(|lang| rule_languages.iter().any(|r| r == lang))
}

/// Artifact dimension match.
///
/// [`ResolveInputs`] has no `--artifact-kind` input, so any rule
/// that populates `applicability.artifacts` lacks caller input by
/// definition and is excluded unless `include_unmatched` is set (the
/// rules contract's populated-without-input rule).
const fn artifact_dimension_matches(
    rule_artifacts: Option<&[String]>, include_unmatched: bool,
) -> bool {
    if rule_artifacts.is_none() {
        return true;
    }
    include_unmatched
}

/// Path-globs dimension match.
///
/// Returns `true` when the rule does not constrain paths. When the
/// rule populates paths but the caller supplied none, the rule is
/// excluded unless `include_unmatched`. Otherwise the rule matches
/// when any caller path matches any compiled rule pattern via
/// [`Pattern::matches_path_with`]. Patterns that fail to compile are
/// treated as non-matching rather than aborting the resolver —
/// `specify lint framework` catches pattern authoring bugs.
fn paths_dimension_matches(
    rule_paths: Option<&[String]>, caller_paths: &[PathBuf], include_unmatched: bool,
) -> bool {
    let Some(rule_paths) = rule_paths else {
        return true;
    };
    if caller_paths.is_empty() {
        return include_unmatched;
    }
    let patterns: Vec<Pattern> =
        rule_paths.iter().filter_map(|pat| Pattern::new(pat).ok()).collect();
    if patterns.is_empty() {
        return false;
    }
    caller_paths
        .iter()
        .map(|p| normalise_path(p))
        .any(|candidate| patterns.iter().any(|p| p.matches_with(&candidate, PATH_MATCH_OPTIONS)))
}

/// Strip the optional `@v<major>` suffix from an adapter reference so
/// v1 matching compares bare names. `"omnia@1.0.0"` becomes `"omnia"`;
/// `"omnia"` is returned unchanged.
fn strip_version_suffix(adapter_ref: &str) -> &str {
    adapter_ref.split_once('@').map_or(adapter_ref, |(name, _)| name)
}

/// Normalise a caller path to a forward-slash string, dropping any
/// leading `./`. Matches the rule path-glob semantics, which fix
/// `/` as the only separator regardless of host OS.
fn normalise_path(path: &Path) -> String {
    let displayed = path.to_string_lossy();
    let forward = if cfg!(windows) { displayed.replace('\\', "/") } else { displayed.into_owned() };
    forward.strip_prefix("./").map_or_else(|| forward.clone(), str::to_string)
}

#[cfg(test)]
mod tests;
