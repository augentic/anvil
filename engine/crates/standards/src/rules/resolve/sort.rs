//! Stable export ordering and `ResolvedRules` assembly (CH-14).
//!
//! Implements `ResolvedRules` export contract §"Ordering". CH-12 owns
//! discovery, CH-13 owns filtering; this module sorts the survivors
//! by the closed four-tuple and lifts each [`ResolvedRuleEntry`] into
//! a wire-shaped [`ResolvedRule`] inside a [`ResolvedRules`] envelope.
//!
//! # Sort tuple
//!
//! Per the rules contract §"Ordering" the closed sort key is:
//!
//! 1. **non-deprecated before deprecated** — `rule.deprecated.is_some()`
//!    maps to `false < true`, putting active rules ahead of historical
//!    citations.
//! 2. **severity** — `critical < important < suggestion < optional`.
//! 3. **origin** — `target < source < shared < core < unknown`.
//! 4. **`rule-id`** lexical.
//!
//! Both [`super::super::Severity`] and [`super::super::Origin`] are
//! declared with variants in the contract sort order in
//! [`crate::rules`], so the derived [`Ord`] picks up the
//! contract-defined comparator directly — no bespoke `_sort_key` helpers
//! needed. The `severity_ordering_matches_contract` and
//! `origin_ordering_matches_contract` tests in `crates/standards/src/rules.rs`
//! pin that declaration order so a future refactor cannot silently
//! shift the sort.
//!
//! [`slice::sort_by`] is a stable sort, so ties on the closed four-tuple
//! preserve CH-12's lexical intra-directory ordering. That keeps
//! same-id-prefix rules from the same overlay rung in the order
//! `list_rule_files` produced them.
//!
//! # Path stability
//!
//! [`ResolvedRule::path`] is carried verbatim from
//! [`ResolvedRuleEntry::path`] (already relative to
//! [`ResolvedRule::path_root`] per CH-12 §"Compute `path` relative to
//! `root`"). The resolver writes forward-slash paths on every host,
//! so golden bytes match across Linux, macOS, and Windows.

use super::{ResolveError, ResolveInputs, ResolvedRuleEntry, filter};
use crate::rules::{ResolvedRule, ResolvedRules, Rule};

/// Sort `entries` in place by the closed rules-export four-tuple.
///
/// See the module docs for the ordering rationale. [`slice::sort_by`]
/// is stable, so ties on the four-tuple preserve CH-12's lexical
/// intra-directory ordering.
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
/// This is the top-level entry point CH-17 (the `specify rules
/// export` CLI) will call. The returned envelope is fully ordered and
/// ready for serialisation against `resolved.schema.json`.
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
        lint_mode,
        applicability,
        rule_hints,
        references,
        deprecated,
        body,
    } = rule;
    ResolvedRule {
        rule_id: id,
        title,
        severity,
        trigger,
        lint_mode,
        applicability,
        rule_hints,
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
