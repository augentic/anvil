//! Stable export ordering and `ResolvedRules` assembly.
//!
//! Sorts the filtered pool by the contract's closed four-tuple —
//! non-deprecated before deprecated, then severity, then origin, then
//! `rule-id` lexical — and lifts each [`ResolvedRuleEntry`] into a
//! wire-shaped [`ResolvedRule`] inside a [`ResolvedRules`] envelope.
//! [`crate::rules`] declares the `Severity` and `Origin` variants in
//! contract sort order, so the derived [`Ord`] is the comparator;
//! tests in `rules.rs` pin that declaration order. The stable sort
//! preserves the resolver's lexical intra-directory ordering on ties,
//! and paths are carried verbatim (forward-slash on every host) so
//! golden bytes match across platforms.

use super::{ResolveError, ResolveInputs, ResolvedRuleEntry, filter};
use crate::rules::{ResolvedRule, ResolvedRules, Rule};

/// Sort `entries` in place by the closed rules-export four-tuple.
///
/// See the module docs for the ordering rationale. [`slice::sort_by`]
/// is stable, so ties on the four-tuple preserve the resolver's
/// lexical intra-directory ordering.
pub fn sort_resolved(entries: &mut [ResolvedRuleEntry]) {
    entries.sort_by(|a, b| {
        let key_a = (a.rule.deprecated.is_some(), a.rule.severity, a.origin, a.rule.id.as_str());
        let key_b = (b.rule.deprecated.is_some(), b.rule.severity, b.origin, b.rule.id.as_str());
        key_a.cmp(&key_b)
    });
}

/// Compose [`super::resolve`], [`super::filter`], and [`sort_resolved`]
/// to assemble the [`ResolvedRules`] wire envelope.
///
/// This is the top-level entry point the `specify rules export` CLI
/// calls. The returned envelope is fully ordered and ready for
/// serialisation against `resolved.schema.json`.
///
/// # Errors
///
/// Returns the same [`ResolveError`] variants as the underlying
/// [`mod@super::super::resolve`] call; sort + lift are infallible.
pub fn build_resolved_rules(inputs: &ResolveInputs<'_>) -> Result<ResolvedRules, ResolveError> {
    let mut entries = filter(super::resolve(inputs)?, inputs);
    sort_resolved(&mut entries);
    let rules = entries.into_iter().map(entry_into_resolved_rule).collect();
    Ok(ResolvedRules {
        version: 1,
        target_adapter: inputs.target_adapter.to_string(),
        source_adapters: inputs.source_adapters.to_vec(),
        rules,
    })
}

/// Lift a [`ResolvedRuleEntry`] into a wire-shaped [`ResolvedRule`].
///
/// Consumes the entry so `rule.body` and the other owned strings move
/// into the result without cloning. The `rule_id` wire field comes
/// from [`Rule::id`]; the rename is documented on
/// [`ResolvedRule::rule_id`].
fn entry_into_resolved_rule(entry: ResolvedRuleEntry) -> ResolvedRule {
    let ResolvedRuleEntry {
        rule,
        origin,
        path_root,
        path,
    } = entry;
    let Rule {
        id,
        title,
        severity,
        trigger,
        applicability,
        references,
        deprecated,
        body,
    } = rule;
    ResolvedRule {
        rule_id: id,
        title,
        severity,
        trigger,
        applicability,
        references,
        origin,
        path_root,
        path,
        body,
        deprecated,
    }
}

#[cfg(test)]
mod tests;
