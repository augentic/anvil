//! Cross-brief rules that span multiple artifacts.

use crate::validate::{CrossContext, CrossRule, RuleOutcome, primitives};

fn proposal_domains_have_specs(ctx: &CrossContext<'_>) -> RuleOutcome {
    let proposal_path = ctx.slice_dir.join("proposal.md");
    if !proposal_path.is_file() {
        return RuleOutcome::Pass;
    }
    let proposal_text = match std::fs::read_to_string(&proposal_path) {
        Ok(t) => t,
        Err(err) => {
            return RuleOutcome::Fail {
                detail: format!("failed to read proposal `{}`: {err}", proposal_path.display()),
            };
        }
    };
    if primitives::proposal_deliverables_have_specs(&proposal_text, ctx.specs_dir) {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: "one or more domains listed in the proposal have no matching spec file"
                .to_string(),
        }
    }
}

fn design_references_valid(ctx: &CrossContext<'_>) -> RuleOutcome {
    let design_path = ctx.slice_dir.join("design.md");
    if !design_path.is_file() {
        return RuleOutcome::Pass;
    }
    let design_text = match std::fs::read_to_string(&design_path) {
        Ok(t) => t,
        Err(err) => {
            return RuleOutcome::Fail {
                detail: format!("failed to read design `{}`: {err}", design_path.display()),
            };
        }
    };
    if primitives::design_references_exist(&design_text, ctx.specs_dir) {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: "design.md references requirement IDs that are not present in the baseline"
                .to_string(),
        }
    }
}

fn composition_consistent(ctx: &CrossContext<'_>) -> RuleOutcome {
    let comp_path = ctx.slice_dir.join("composition.yaml");
    let Ok(comp_text) = std::fs::read_to_string(&comp_path) else {
        return RuleOutcome::Pass;
    };

    let doc: serde_json::Value = match serde_saphyr::from_str(&comp_text) {
        Ok(v) => v,
        Err(_) => return RuleOutcome::Pass,
    };

    let Some(screens) = doc.get("screens").and_then(|s| s.as_object()) else {
        if let Some(delta) = doc.get("delta").and_then(|d| d.as_object()) {
            let mut maps_to_issues: Vec<String> = Vec::new();
            for section_key in &["added", "modified"] {
                if let Some(section) = delta.get(*section_key).and_then(|s| s.as_object()) {
                    for (slug, screen) in section {
                        if let Some(maps_to) = screen.get("maps_to") {
                            if let Some(val) = maps_to.as_str() {
                                if val.is_empty() {
                                    maps_to_issues
                                        .push(format!("screen `{slug}` has empty `maps_to`"));
                                }
                            } else {
                                maps_to_issues
                                    .push(format!("screen `{slug}` has non-string `maps_to`"));
                            }
                        }
                    }
                }
            }
            if maps_to_issues.is_empty() {
                return RuleOutcome::Pass;
            }
            return RuleOutcome::Fail {
                detail: maps_to_issues.join("; "),
            };
        }
        return RuleOutcome::Pass;
    };

    let mut issues: Vec<String> = Vec::new();
    for (slug, screen) in screens {
        if let Some(maps_to) = screen.get("maps_to") {
            if let Some(val) = maps_to.as_str() {
                if val.is_empty() {
                    issues.push(format!("screen `{slug}` has empty `maps_to`"));
                }
            } else {
                issues.push(format!("screen `{slug}` has non-string `maps_to`"));
            }
        }
    }

    if issues.is_empty() {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: issues.join("; "),
        }
    }
}

const CROSS_RULES: &[CrossRule] = &[
    CrossRule {
        id: "cross.proposal-domains-have-specs",
        description: "Every domain listed in the proposal has a matching spec file",
        check: proposal_domains_have_specs,
    },
    CrossRule {
        id: "cross.design-references-valid",
        description: "Every requirement id referenced in design.md exists in specs",
        check: design_references_valid,
    },
    CrossRule {
        id: "cross.composition-maps-to-consistent",
        description: "composition.yaml maps_to values are well-formed",
        check: composition_consistent,
    },
];

/// Return the registered cross-brief rules.
#[must_use]
pub const fn cross_rules() -> &'static [CrossRule] {
    CROSS_RULES
}
