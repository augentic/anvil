//! Parity gate for the committed judgment-answer schema goldens under
//! `crates/project/answers/`: each committed document must byte-match
//! the current generation from the Rust wire types. Regenerate with
//! `REGENERATE_GOLDENS=1`.

use std::path::PathBuf;

use serde_json::Value;

fn assert_golden(file: &str, schema: &Value) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("answers").join(file);
    let actual = project::answers::render(schema);
    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        std::fs::create_dir_all(path.parent().expect("answers dir")).expect("create answers dir");
        std::fs::write(&path, &actual).expect("regenerate golden");
    }
    let expected = std::fs::read_to_string(&path).expect("read golden");
    assert_eq!(actual, expected, "golden mismatch: {}", path.display());
}

#[test]
fn leads_golden() {
    assert_golden("leads.schema.json", &project::answers::leads());
}

#[test]
fn evidence_golden() {
    assert_golden("evidence.schema.json", &project::answers::evidence());
}

#[test]
fn report_golden() {
    assert_golden("report.schema.json", &project::answers::report());
}

#[test]
fn phase_report_golden() {
    assert_golden("phase-report.schema.json", &project::answers::phase_report());
}

#[test]
fn proposal_golden() {
    assert_golden("proposal.schema.json", &project::answers::proposal());
}

#[test]
fn partition_golden() {
    assert_golden("partition.schema.json", &project::answers::partition());
}

#[test]
fn boundary_review_golden() {
    assert_golden("boundary-review.schema.json", &project::answers::boundary_review());
}

/// The semantic constraints patched onto the generated shapes: kebab
/// grammars on lead ids and topic slugs, the dotted-kebab claim-id
/// grammar, and the conditional id requirement on `requirement` /
/// `criterion` / `example` claims.
mod patched_constraints {
    use serde_json::Value;

    const KEBAB: &str = "^[a-z0-9]+(-[a-z0-9]+)*$";
    const DOTTED_KEBAB: &str = "^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$";

    fn pattern_at(schema: &Value, pointer: &str) -> String {
        schema
            .pointer(pointer)
            .and_then(|property| property.get("pattern"))
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("pattern at {pointer}"))
            .to_string()
    }

    #[test]
    fn lead_id_kebab() {
        let schema = project::answers::leads();
        assert_eq!(pattern_at(&schema, "/$defs/Lead/properties/lead"), KEBAB);
    }

    #[test]
    fn topic_slug_kebab() {
        let schema = project::answers::leads();
        assert_eq!(pattern_at(&schema, "/$defs/Lead/properties/topics/items"), KEBAB);
    }

    #[test]
    fn claim_id_dotted_kebab() {
        let schema = project::answers::evidence();
        assert_eq!(pattern_at(&schema, "/$defs/Claim/properties/id"), DOTTED_KEBAB);
    }

    #[test]
    fn claim_id_conditional() {
        let schema = project::answers::evidence();
        let condition = schema.pointer("/$defs/Claim/if").expect("if clause on Claim");
        assert_eq!(
            condition.pointer("/properties/kind/enum").expect("kind enum"),
            &serde_json::json!(["requirement", "criterion", "example"])
        );
        let consequence = schema.pointer("/$defs/Claim/then").expect("then clause on Claim");
        assert_eq!(consequence.get("required").expect("required"), &serde_json::json!(["id"]));
        assert_eq!(
            consequence.pointer("/properties/id/type").expect("id type"),
            &serde_json::json!("string")
        );
    }
}
