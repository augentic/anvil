//! Post-merge coherence checks over a merged baseline spec.

use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;
use specify_diagnostics::{Artifact, Diagnostic};
use specify_model::spec::{REQ_ID_PATTERN, REQ_ID_PREFIX, SCENARIO_HEADING, parse_baseline};

fn fail(rule_id: &str, rule: &str, detail: String) -> Diagnostic {
    Diagnostic::violation(rule_id, rule, detail, Artifact::Specs, None)
}

fn req_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(REQ_ID_PATTERN).expect("REQ_ID_PATTERN must be a valid regex"))
}

const RULE_NO_DUPLICATE_IDS: &str = "merge.no-duplicate-ids";
const RULE_NO_DUPLICATE_IDS_DESC: &str = "baseline has no duplicate requirement IDs";

const RULE_NO_DUPLICATE_NAMES: &str = "merge.no-duplicate-names";
const RULE_NO_DUPLICATE_NAMES_DESC: &str = "baseline has no duplicate requirement names";

const RULE_REQ_HAS_ID: &str = "merge.requirement-has-id";
const RULE_REQ_HAS_ID_DESC: &str = "every requirement block carries an `ID:` line";

const RULE_ID_MATCHES_PATTERN: &str = "merge.id-matches-pattern";
const RULE_ID_MATCHES_PATTERN_DESC: &str = "requirement IDs match the canonical REQ-NNN pattern";

const RULE_REQ_HAS_SCENARIO: &str = "merge.requirement-has-scenario";
const RULE_REQ_HAS_SCENARIO_DESC: &str = "every requirement declares at least one scenario";

/// Post-merge coherence validation.
///
/// Returns one [`Diagnostic`] `violation` per breach (always
/// deterministic, `important`); an empty vec means the baseline passes
/// every coherence rule. Design-reference checking is not part of this
/// pass — the `cross.design-references-valid` rule in
/// `specify-model`'s `validate` registry owns it.
///
/// # Panics
///
/// Panics if `REQ_ID_PATTERN` is not a valid regex (compile-time
/// constant — should never happen).
#[must_use]
pub fn validate_baseline(baseline: &str) -> Vec<Diagnostic> {
    let parsed = parse_baseline(baseline);
    let blocks = parsed.requirements;
    let mut results: Vec<Diagnostic> = Vec::new();

    // (a) Duplicate IDs.
    let mut seen_ids: HashSet<&str> = HashSet::new();
    for block in &blocks {
        if block.id.is_empty() {
            continue;
        }
        if seen_ids.contains(block.id.as_str()) {
            results.push(fail(
                RULE_NO_DUPLICATE_IDS,
                RULE_NO_DUPLICATE_IDS_DESC,
                format!("Duplicate ID: {}", block.id),
            ));
        }
        seen_ids.insert(block.id.as_str());
    }

    // (b) Duplicate names.
    let mut seen_names: HashSet<&str> = HashSet::new();
    for block in &blocks {
        if seen_names.contains(block.name.as_str()) {
            results.push(fail(
                RULE_NO_DUPLICATE_NAMES,
                RULE_NO_DUPLICATE_NAMES_DESC,
                format!("Duplicate requirement name: {}", block.name),
            ));
        }
        seen_names.insert(block.name.as_str());
    }

    // (c) Heading structure — ID present, ID matches pattern, scenario present.
    let id_pattern = req_id_re();
    // The scenario check strips the trailing colon so the substring match
    // still fires when body text contains `"#### Scenario"` without one.
    let scenario_needle = SCENARIO_HEADING.trim_end_matches(':');
    for block in &blocks {
        if block.id.is_empty() {
            results.push(fail(
                RULE_REQ_HAS_ID,
                RULE_REQ_HAS_ID_DESC,
                format!("Requirement '{}' has no {} line", block.name, REQ_ID_PREFIX),
            ));
        } else if !id_pattern.is_match(&block.id) {
            results.push(fail(
                RULE_ID_MATCHES_PATTERN,
                RULE_ID_MATCHES_PATTERN_DESC,
                format!(
                    "Requirement '{}' has invalid ID '{}' (expected pattern: {})",
                    block.name, block.id, REQ_ID_PATTERN
                ),
            ));
        }
        if !block.body.contains(scenario_needle) {
            results.push(fail(
                RULE_REQ_HAS_SCENARIO,
                RULE_REQ_HAS_SCENARIO_DESC,
                format!(
                    "Requirement '{}' ({}) has no {} section",
                    block.name, block.id, SCENARIO_HEADING
                ),
            ));
        }
    }

    results
}

#[cfg(test)]
mod tests;
