//! Cross-brief rules that span multiple artifacts.

use crate::validate::{Classification, CrossContext, CrossRule, RuleOutcome, primitives};

fn cross_proposal_domains_have_specs(ctx: &CrossContext<'_>) -> RuleOutcome {
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

fn cross_design_references_valid(ctx: &CrossContext<'_>) -> RuleOutcome {
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

fn cross_composition_maps_to_consistent(ctx: &CrossContext<'_>) -> RuleOutcome {
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
        classification: Classification::Structural,
        check: cross_proposal_domains_have_specs,
    },
    CrossRule {
        id: "cross.design-references-valid",
        description: "Every requirement id referenced in design.md exists in specs",
        classification: Classification::Structural,
        check: cross_design_references_valid,
    },
    CrossRule {
        id: "cross.composition-maps-to-consistent",
        description: "composition.yaml maps_to values are well-formed",
        classification: Classification::Structural,
        check: cross_composition_maps_to_consistent,
    },
];

/// Return the registered cross-brief rules.
#[must_use]
pub const fn cross_rules() -> &'static [CrossRule] {
    CROSS_RULES
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use crate::validate::CrossContext;

    /// A slice dir plus a `specs/` sibling, both inside one tempdir.
    fn fixture() -> (TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let specs = dir.path().join("specs");
        fs::create_dir_all(&specs).expect("mkdir specs");
        (dir, specs)
    }

    fn ctx<'a>(slice_dir: &'a Path, specs_dir: &'a Path) -> CrossContext<'a> {
        CrossContext { slice_dir, specs_dir }
    }

    mod proposal_domains_have_specs {
        use super::{ctx, fixture};
        use crate::validate::RuleOutcome;
        use crate::validate::registry::cross::cross_proposal_domains_have_specs;

        /// Absent proposal is not this rule's concern — it passes.
        #[test]
        fn passes_when_no_proposal() {
            let (dir, specs) = fixture();
            assert_eq!(
                cross_proposal_domains_have_specs(&ctx(dir.path(), &specs)),
                RuleOutcome::Pass
            );
        }

        #[test]
        fn passes_when_domain_has_spec() {
            let (dir, specs) = fixture();
            std::fs::create_dir_all(specs.join("login")).expect("mkdir");
            std::fs::write(specs.join("login").join("spec.md"), "# Login\n").expect("write spec");
            std::fs::write(dir.path().join("proposal.md"), "## Domains\n\n- login\n")
                .expect("write proposal");
            assert_eq!(
                cross_proposal_domains_have_specs(&ctx(dir.path(), &specs)),
                RuleOutcome::Pass
            );
        }

        #[test]
        fn fails_when_domain_missing_spec() {
            let (dir, specs) = fixture();
            std::fs::write(dir.path().join("proposal.md"), "## Domains\n\n- ghost\n")
                .expect("write proposal");
            assert!(matches!(
                cross_proposal_domains_have_specs(&ctx(dir.path(), &specs)),
                RuleOutcome::Fail { detail } if detail.contains("no matching spec")
            ));
        }
    }

    mod design_references_valid {
        use super::{ctx, fixture};
        use crate::validate::RuleOutcome;
        use crate::validate::registry::cross::cross_design_references_valid;

        #[test]
        fn passes_when_no_design() {
            let (dir, specs) = fixture();
            assert_eq!(cross_design_references_valid(&ctx(dir.path(), &specs)), RuleOutcome::Pass);
        }

        #[test]
        fn passes_when_reference_backed() {
            let (dir, specs) = fixture();
            std::fs::create_dir_all(specs.join("a")).expect("mkdir");
            std::fs::write(specs.join("a").join("spec.md"), "ID: REQ-001\n").expect("write spec");
            std::fs::write(dir.path().join("design.md"), "See REQ-001.\n").expect("write design");
            assert_eq!(cross_design_references_valid(&ctx(dir.path(), &specs)), RuleOutcome::Pass);
        }

        #[test]
        fn fails_when_reference_dangling() {
            let (dir, specs) = fixture();
            std::fs::write(dir.path().join("design.md"), "See REQ-404.\n").expect("write design");
            assert!(matches!(
                cross_design_references_valid(&ctx(dir.path(), &specs)),
                RuleOutcome::Fail { detail } if detail.contains("not present in the baseline")
            ));
        }
    }

    mod composition_maps_to {
        use super::{ctx, fixture};
        use crate::validate::RuleOutcome;
        use crate::validate::registry::cross::cross_composition_maps_to_consistent;

        fn write_comp(dir: &std::path::Path, body: &str) {
            std::fs::write(dir.join("composition.yaml"), body).expect("write composition");
        }

        #[test]
        fn passes_when_no_composition() {
            let (dir, specs) = fixture();
            assert_eq!(
                cross_composition_maps_to_consistent(&ctx(dir.path(), &specs)),
                RuleOutcome::Pass
            );
        }

        #[test]
        fn passes_on_non_empty_string_maps_to() {
            let (dir, specs) = fixture();
            write_comp(dir.path(), "screens:\n  home:\n    maps_to: REQ-001\n");
            assert_eq!(
                cross_composition_maps_to_consistent(&ctx(dir.path(), &specs)),
                RuleOutcome::Pass
            );
        }

        #[test]
        fn fails_on_empty_maps_to() {
            let (dir, specs) = fixture();
            write_comp(dir.path(), "screens:\n  home:\n    maps_to: \"\"\n");
            assert!(matches!(
                cross_composition_maps_to_consistent(&ctx(dir.path(), &specs)),
                RuleOutcome::Fail { detail } if detail.contains("empty `maps_to`")
            ));
        }

        #[test]
        fn fails_on_non_string_maps_to() {
            let (dir, specs) = fixture();
            write_comp(dir.path(), "screens:\n  home:\n    maps_to: 7\n");
            assert!(matches!(
                cross_composition_maps_to_consistent(&ctx(dir.path(), &specs)),
                RuleOutcome::Fail { detail } if detail.contains("non-string `maps_to`")
            ));
        }

        /// The delta branch validates `added`/`modified` slug maps the
        /// same way as the flat `screens` branch.
        #[test]
        fn fails_on_empty_maps_to_in_delta() {
            let (dir, specs) = fixture();
            write_comp(dir.path(), "delta:\n  added:\n    home:\n      maps_to: \"\"\n");
            assert!(matches!(
                cross_composition_maps_to_consistent(&ctx(dir.path(), &specs)),
                RuleOutcome::Fail { detail } if detail.contains("empty `maps_to`")
            ));
        }
    }
}
