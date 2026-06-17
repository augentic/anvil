//! In-process `prose` framework checker (Road B `kind: tool`).
//!
//! Covers CORE-024 (`prose.numeric-cap-exceeded`): the documented skill
//! description / body caps must stay in sync across the canonical
//! skill schema and the `docs/standards/skill-authoring.md` prose. The
//! cap *values* arrive in the rule's forwarded `config:`.
//!
//! Unlike the retired WASI tool — which substring-scanned its embedded
//! schema copy for the cap digits — the schema side here parses the
//! canonical [`specify_schema::SKILL_JSON_SCHEMA`] and compares
//! `properties.description.maxLength` numerically.

use std::path::Path;

use serde_json::Value as JsonValue;

use super::support::{ToolFinding, parsed_config};

const RULE_NUMERIC_CAP_EXCEEDED: &str = "CORE-024";

/// Standards document that must carry both numeric caps in prose.
const STANDARDS_REL: &str = "docs/standards/skill-authoring.md";

const IMPACT: &str = "A documented numeric cap has drifted from its canonical source, so authors read a stale limit.";
const REMEDIATION: &str = "Restore the cap value in the drifted source so the schema and standards doc agree with the rule's policy.";

/// Run the numeric-cap drift check with the forwarded
/// `{description-cap, body-cap}` policy.
pub fn run(project_dir: &Path, args: &[String]) -> Vec<ToolFinding> {
    let config = parsed_config(args);
    let Some((description_cap, body_cap)) = caps(config.as_ref()) else {
        // No policy supplied: nothing to compare against. Emit a clean
        // report rather than inventing a cap.
        return Vec::new();
    };
    check_numeric_caps(project_dir, description_cap, body_cap)
}

/// Read CORE-024's `{description-cap, body-cap}` policy out of the
/// forwarded config; `None` when either is absent.
fn caps(config: Option<&JsonValue>) -> Option<(u64, u64)> {
    let config = config?;
    let description = config.get("description-cap").and_then(JsonValue::as_u64)?;
    let body = config.get("body-cap").and_then(JsonValue::as_u64)?;
    Some((description, body))
}

/// CORE-024: the `description_cap` must match the canonical skill
/// schema's `description.maxLength`, and both caps must appear in the
/// standards document prose.
fn check_numeric_caps(project_dir: &Path, description_cap: u64, body_cap: u64) -> Vec<ToolFinding> {
    let standards_path = project_dir.join(STANDARDS_REL);
    // The standards document is a plugins-repo authoring artifact; an
    // adapters-only framework root (RFC-48 H1) legitimately omits it, so
    // an absent doc is a skip — distinct from a present-but-unreadable
    // doc, which still flags below.
    if !standards_path.exists() {
        return Vec::new();
    }

    let mut findings = Vec::new();

    match schema_description_max_length() {
        Some(max_length) if max_length == description_cap => {}
        Some(max_length) => findings.push(ToolFinding {
            rule_id: RULE_NUMERIC_CAP_EXCEEDED,
            path: None,
            message: format!(
                "Skill description cap drift in skill.schema.json: description.maxLength is {max_length}, expected {description_cap}"
            ),
            impact: IMPACT,
            remediation: REMEDIATION,
        }),
        None => findings.push(ToolFinding {
            rule_id: RULE_NUMERIC_CAP_EXCEEDED,
            path: None,
            message: "Skill description cap source missing: skill.schema.json declares no description.maxLength".to_string(),
            impact: IMPACT,
            remediation: REMEDIATION,
        }),
    }

    let description = description_cap.to_string();
    let body = body_cap.to_string();
    match std::fs::read_to_string(&standards_path) {
        Ok(content) => {
            if !content.contains(&description) {
                findings.push(standards_finding(format!(
                    "Skill description cap drift in {STANDARDS_REL}; expected {description}"
                )));
            }
            if !content.contains(&body) {
                findings.push(standards_finding(format!(
                    "Skill body cap drift in {STANDARDS_REL}; expected {body}"
                )));
            }
        }
        Err(_) => findings
            .push(standards_finding(format!("Skill numeric cap source missing: {STANDARDS_REL}"))),
    }

    findings
}

/// The canonical skill schema's `properties.description.maxLength`,
/// parsed (not substring-scanned) from the embedded constant.
fn schema_description_max_length() -> Option<u64> {
    let schema: JsonValue = serde_json::from_str(specify_schema::SKILL_JSON_SCHEMA).ok()?;
    schema.get("properties")?.get("description")?.get("maxLength").and_then(JsonValue::as_u64)
}

fn standards_finding(message: String) -> ToolFinding {
    ToolFinding {
        rule_id: RULE_NUMERIC_CAP_EXCEEDED,
        path: Some(STANDARDS_REL.to_string()),
        message,
        impact: IMPACT,
        remediation: REMEDIATION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_standards(root: &Path, body: &str) {
        let path = root.join(STANDARDS_REL);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, body).expect("write");
    }

    #[test]
    fn clean_tree_is_silent() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_standards(dir.path(), "Description cap: 512 characters. Body cap: 200 lines.\n");
        assert!(check_numeric_caps(dir.path(), 512, 200).is_empty());
    }

    #[test]
    fn flags_body_cap_drift() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_standards(dir.path(), "Description cap: 512 characters.\n");
        let findings = check_numeric_caps(dir.path(), 512, 200);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_NUMERIC_CAP_EXCEEDED);
        assert!(findings[0].message.contains("body cap drift"));
    }

    #[test]
    fn flags_schema_max_length_drift() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_standards(dir.path(), "Description cap: 400 characters. Body cap: 200 lines.\n");
        let findings = check_numeric_caps(dir.path(), 400, 200);
        assert!(
            findings.iter().any(|f| f.message.contains("description.maxLength is 512")),
            "expected parsed maxLength drift finding, got {findings:?}"
        );
    }

    #[test]
    fn absent_standards_doc_is_silent() {
        let dir = tempfile::tempdir().expect("tempdir");
        // An adapters-only framework root (RFC-48 H1) carries no
        // standards doc: absent is a skip, not a missing-source finding.
        assert!(check_numeric_caps(dir.path(), 512, 200).is_empty());
    }
}
