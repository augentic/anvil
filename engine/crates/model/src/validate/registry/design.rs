//! Design-brief rules.

use crate::validate::{BriefContext, Classification, Rule, RuleOutcome, primitives};

fn design_references_valid_ids(ctx: &BriefContext<'_>) -> RuleOutcome {
    if primitives::design_references_exist(ctx.content, ctx.specs_dir) {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: "design.md references requirement IDs not present in any baseline spec"
                .to_string(),
        }
    }
}

pub(super) const DESIGN_RULES: &[Rule] = &[Rule {
    id: "design.references-valid-ids",
    description: "References only requirement ids present in specs",
    classification: Classification::Structural,
    check: Some(design_references_valid_ids),
}];

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::design_references_valid_ids;
    use crate::validate::{BriefContext, RuleOutcome};

    fn ctx<'a>(content: &'a str, specs_dir: &'a Path) -> BriefContext<'a> {
        BriefContext {
            id: "design",
            content,
            parsed_spec: None,
            tasks: None,
            slice_dir: Path::new("."),
            specs_dir,
        }
    }

    #[test]
    fn passes_when_ids_backed_by_specs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let specs = dir.path().join("specs");
        std::fs::create_dir_all(specs.join("a")).expect("mkdir");
        std::fs::write(specs.join("a").join("spec.md"), "### Requirement\nID: REQ-001\n")
            .expect("write spec");
        assert_eq!(
            design_references_valid_ids(&ctx("See REQ-001 here.", &specs)),
            RuleOutcome::Pass
        );
    }

    #[test]
    fn fails_when_id_missing_from_specs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let specs = dir.path().join("specs");
        std::fs::create_dir_all(specs.join("a")).expect("mkdir");
        std::fs::write(specs.join("a").join("spec.md"), "### Requirement\nID: REQ-001\n")
            .expect("write spec");
        assert!(matches!(
            design_references_valid_ids(&ctx("See REQ-999 here.", &specs)),
            RuleOutcome::Fail { .. }
        ));
    }

    #[test]
    fn passes_when_no_ids_are_referenced() {
        let dir = tempfile::tempdir().expect("tempdir");
        let specs = dir.path().join("specs");
        std::fs::create_dir_all(&specs).expect("mkdir");
        assert_eq!(
            design_references_valid_ids(&ctx("No requirement references here.", &specs)),
            RuleOutcome::Pass
        );
    }
}
