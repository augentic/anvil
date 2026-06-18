//! Applicability and deprecation filters (CH-13).
//!
//! Implements rule applicability, path-glob semantics, and
//! §"Overlay precedence" / §"Resolved Decisions". The CH-12 resolver
//! discovers every rule visible to the export envelope; this module
//! narrows that pool down to the rules that actually apply to the
//! caller's slice context, using the inclusive AND-across-dimensions
//! semantics the rules contract defines.
//!
//! # Precedence
//!
//! Three filters run in a fixed order:
//!
//! 1. **Origin (`core`)** — entries with [`super::super::Origin::Core`]
//!    are dropped unless [`ResolveInputs::include_core`] is set
//!    (consumer-export filtering). The check runs
//!    **first** so `CORE-*` rules never reach the deprecation or
//!    applicability passes on a default consumer export — even if the
//!    rule's `applicability` block would have accepted them.
//! 2. **Deprecation** — entries whose [`Rule::deprecated`] is
//!    populated are dropped unless [`ResolveInputs::include_deprecated`]
//!    is set. Deprecation runs before applicability so an
//!    `--include-deprecated` rule still walks through applicability
//!    normally (§"Overlay precedence": "Export includes every rule that
//!    passes applicability filtering" — `include_deprecated` does not
//!    bypass applicability).
//! 3. **Applicability** — a rule with no `applicability` block always
//!    passes ("A rule with no `applicability` block applies wherever
//!    its root applies"). A rule with an `applicability` block must
//!    pass **every populated dimension** (AND semantics).
//!
//! # Populated dimension + missing caller input
//!
//! Rule applicability closes the awkward edge case: if the rule
//! populates a dimension but the caller supplied no matching input, the
//! rule is **excluded** unless [`ResolveInputs::include_unmatched`] is
//! set. This module applies that rule per dimension.
//!
//! # Per-dimension matching
//!
//! - **`adapters`** — caller input is always populated
//!   ([`ResolveInputs::target_adapter`] is required), so the
//!   missing-input branch is unreachable. The rule matches when its
//!   adapter list contains either the target adapter or any bound
//!   source adapter. The rule's adapter ref may carry an optional
//!   `@v<major>` suffix; for v1 this module strips the suffix before
//!   comparison and matches by bare name.
//! - **`languages`** — case-sensitive bare-name match against
//!   [`ResolveInputs::languages`]. Missing caller input (empty slice)
//!   excludes the rule unless `include_unmatched`.
//! - **`artifacts`** — v1 has no `--artifact-kind` input on
//!   [`ResolveInputs`]. Per the closed
//!   populated-dimension-without-caller-input rule, any rule that
//!   populates `applicability.artifacts` is **excluded unless
//!   `include_unmatched` is set**. This is an honest reflection of v1
//!   capability; a future contract may add an explicit `--artifact-kind`
//!   input, after which `artifact_dimension_matches` would consult it
//!   the same way `language_dimension_matches` consults `languages`.
//! - **`paths`** — patterns interpreted by the `glob` crate
//!   (`glob = "0.3"`). Per the rules contract §"Path glob semantics
//!   (`applicability.paths`)": `*` matches a single path segment
//!   without crossing `/`, `**` matches across segments, and `/` is the
//!   only separator. Matching is case-sensitive. Caller paths are
//!   normalised to forward-slash separators on Windows before being
//!   tested. Missing caller input (empty [`ResolveInputs::artifact_paths`])
//!   excludes the rule unless `include_unmatched`.
//!
//! Malformed glob patterns are treated as **non-matching** rather than
//! aborting the resolver. `specify lint framework` catches
//! authoring bugs in patterns; surfacing a hard error here over a
//! single first-party typo would be more disruptive than dropping the
//! rule from the export. A future authoring-check could elevate this to
//! a hard error.
//!
//! # Composition
//!
//! Call [`super::resolve`] first, then pass its `Vec<ResolvedRuleEntry>`
//! to [`filter`] — e.g. `filter(super::resolve(inputs)?, inputs)`.
//! [`build_resolved_rules`] in the sibling `sort` module is the
//! conventional export entry point.

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

/// Apply origin + deprecation + applicability filters to a CH-12 result.
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
/// Rules resolved from `adapters/shared/rules/core/`
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
/// v1 [`ResolveInputs`] has no `--artifact-kind` input, so any rule
/// that populates `applicability.artifacts` lacks caller input by
/// definition. Per the rules contract §"Applicability" the populated-without-input
/// case excludes the rule unless `include_unmatched` is set. See the
/// module docs.
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
/// treated as non-matching — see the module docs.
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
