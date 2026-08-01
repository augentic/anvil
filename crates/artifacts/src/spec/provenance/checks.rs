//! Per-requirement provenance validation over a [`ParsedSpec`]: the
//! `ID:` / `Sources:` / `Status:` rules and the heading-tag coherence
//! gate, cross-checked against the slice's plan-level source keys.

use std::collections::BTreeSet;

use super::{Finding, ParsedSpec, Requirement, RequirementStatus};

/// Validate parsed requirements against the slice's plan-level source
/// keys. Pass an empty `source_keys` set to skip the cross-validation
/// (structural rules still run).
#[must_use]
pub fn validate(parsed: &ParsedSpec, source_keys: &BTreeSet<String>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for req in &parsed.requirements {
        check_id(req, &mut findings);
        check_sources(req, source_keys, &mut findings);
        check_status(req, &mut findings);
    }
    findings
}

fn check_id(req: &Requirement, out: &mut Vec<Finding>) {
    if req.id.is_empty() {
        out.push(Finding {
            rule_id: "spec.requirement-id-missing",
            rule: "Every requirement carries an `ID:` line",
            detail: format!("requirement {} has no `ID:` line", req.id_or_name()),
            span: req.span,
        });
    } else if !is_valid_req_id(&req.id) {
        out.push(Finding {
            rule_id: "spec.requirement-id-malformed",
            rule: "Requirement `ID:` matches `REQ-NNN` (three ASCII digits)",
            detail: format!("requirement {} has malformed id `{}`", req.id_or_name(), req.id),
            span: req.span,
        });
    }
}

fn check_sources(req: &Requirement, source_keys: &BTreeSet<String>, out: &mut Vec<Finding>) {
    if req.sources_line_absent {
        out.push(Finding {
            rule_id: "spec.requirement-sources-missing",
            rule: "Every requirement carries a `Sources:` line",
            detail: format!("requirement {} has no `Sources:` line", req.id_or_name()),
            span: req.span,
        });
        return;
    }
    if req.sources.is_empty() {
        // Contract: `Sources: []` is legal exactly when `Status: unknown`
        // — an evidence-less requirement (e.g. a reconciliation-inserted
        // bootstrap slice) has no contributing source to cite.
        if req.status != Some(RequirementStatus::Unknown) {
            out.push(Finding {
                rule_id: "spec.requirement-sources-empty",
                rule: "`Sources:` lists at least one key (empty only with `Status: unknown`)",
                detail: format!(
                    "requirement {} has an empty `Sources:` line but is not `Status: unknown`",
                    req.id_or_name()
                ),
                span: req.span,
            });
        }
        return;
    }
    for key in &req.sources {
        if !is_valid_source_key(key) {
            out.push(Finding {
                rule_id: "spec.requirement-source-malformed",
                rule: "Each `Sources:` key is kebab-case (`[a-z][a-z0-9-]*`)",
                detail: format!(
                    "requirement {} has malformed source key `{key}`",
                    req.id_or_name()
                ),
                span: req.span,
            });
            continue;
        }
        if !source_keys.is_empty() && !source_keys.contains(key) {
            out.push(Finding {
                rule_id: "spec.requirement-source-undefined",
                rule: "Each `Sources:` key resolves to a slice-level plan binding",
                detail: format!(
                    "requirement {} references source key `{key}`, which is not declared on the slice's plan entry",
                    req.id_or_name()
                ),
                span: req.span,
            });
        }
    }
}

fn check_status(req: &Requirement, out: &mut Vec<Finding>) {
    match (req.status, req.status_raw.as_deref()) {
        (None, None) => out.push(Finding {
            rule_id: "spec.requirement-status-missing",
            rule: "Every requirement carries a `Status:` line",
            detail: format!("requirement {} has no `Status:` line", req.id_or_name()),
            span: req.span,
        }),
        (None, Some(raw)) => out.push(Finding {
            rule_id: "spec.requirement-status-unknown-value",
            rule: "`Status:` is one of `agreed | unknown | conflict | divergence`",
            detail: format!("requirement {} has unrecognised `Status: {raw}`", req.id_or_name()),
            span: req.span,
        }),
        (Some(status), _) => {
            if let Some(tag) = req.tag {
                if status != tag.expected_status() {
                    out.push(Finding {
                        rule_id: "spec.requirement-tag-status-mismatch",
                        rule: "Heading tag agrees with `Status:` value",
                        detail: format!(
                            "requirement {} carries heading tag `[{tag}]` but `Status: {status}`",
                            req.id_or_name(),
                        ),
                        span: req.span,
                    });
                }
            } else if status != RequirementStatus::Agreed {
                out.push(Finding {
                    rule_id: "spec.requirement-tag-status-mismatch",
                    rule: "Heading tag agrees with `Status:` value",
                    detail: format!(
                        "requirement {} has `Status: {status}` but no `[{status}]` heading tag",
                        req.id_or_name(),
                    ),
                    span: req.span,
                });
            }
        }
    }
}

fn is_valid_req_id(id: &str) -> bool {
    id.strip_prefix("REQ-")
        .is_some_and(|tail| tail.len() == 3 && tail.bytes().all(|b| b.is_ascii_digit()))
}

fn is_valid_source_key(s: &str) -> bool {
    let mut bytes = s.bytes();
    let Some(first) = bytes.next() else { return false };
    if !first.is_ascii_lowercase() {
        return false;
    }
    let mut prev_dash = false;
    for b in bytes {
        if b == b'-' {
            if prev_dash {
                return false;
            }
            prev_dash = true;
        } else if b.is_ascii_lowercase() || b.is_ascii_digit() {
            prev_dash = false;
        } else {
            return false;
        }
    }
    !prev_dash
}
