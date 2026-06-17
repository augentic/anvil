//! Contracts-brief rules.

use crate::validate::{BriefContext, Classification, Rule, RuleOutcome, primitives};

fn contracts_schemas_dir_has_files(ctx: &BriefContext<'_>) -> RuleOutcome {
    let schemas_dir = ctx.slice_dir.join("contracts").join("schemas");
    if !schemas_dir.is_dir() {
        return RuleOutcome::Fail {
            detail: "contracts/schemas/ directory not found in slice".to_string(),
        };
    }

    let has_yaml = std::fs::read_dir(&schemas_dir).ok().is_some_and(|entries| {
        entries
            .filter_map(Result::ok)
            .any(|e| matches!(e.path().extension().and_then(|x| x.to_str()), Some("yaml" | "yml")))
    });

    if has_yaml {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: "contracts/schemas/ exists but contains no .yaml files".to_string(),
        }
    }
}

fn contracts_refs_resolve(ctx: &BriefContext<'_>) -> RuleOutcome {
    let contracts_dir = ctx.slice_dir.join("contracts");
    let mut failures: Vec<String> = Vec::new();

    for subdir in &["http", "messages"] {
        let dir = contracts_dir.join(subdir);
        if !dir.is_dir() {
            continue;
        }

        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !matches!(path.extension().and_then(|x| x.to_str()), Some("yaml" | "yml")) {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            for line in content.lines() {
                if let Some(ref_value) = primitives::extract_ref(line) {
                    let resolved = dir.join(ref_value);
                    if !resolved.is_file() {
                        failures.push(format!(
                            "{}: $ref '{}' does not resolve",
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            ref_value
                        ));
                    }
                }
            }
        }
    }

    if failures.is_empty() {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: failures.join("; "),
        }
    }
}

fn contracts_schema_metadata(ctx: &BriefContext<'_>) -> RuleOutcome {
    let schemas_dir = ctx.slice_dir.join("contracts").join("schemas");
    if !schemas_dir.is_dir() {
        return RuleOutcome::Pass;
    }

    let mut failures: Vec<String> = Vec::new();

    let Ok(entries) = std::fs::read_dir(&schemas_dir) else {
        return RuleOutcome::Pass;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !matches!(path.extension().and_then(|x| x.to_str()), Some("yaml" | "yml")) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        let Ok(doc) = serde_saphyr::from_str::<serde_json::Value>(&content) else {
            continue;
        };
        if doc.get("$id").is_none() {
            failures.push(format!("{filename}: missing $id"));
        }
        if doc.get("title").and_then(|v| v.as_str()).is_none_or(str::is_empty) {
            failures.push(format!("{filename}: missing or empty title"));
        }
        if doc.get("description").and_then(|v| v.as_str()).is_none_or(str::is_empty) {
            failures.push(format!("{filename}: missing or empty description"));
        }
    }

    if failures.is_empty() {
        RuleOutcome::Pass
    } else {
        RuleOutcome::Fail {
            detail: failures.join("; "),
        }
    }
}

pub(super) const CONTRACTS_RULES: &[Rule] = &[
    Rule {
        id: "contracts.schemas-dir-has-files",
        description: "contracts/schemas/ directory exists and contains at least one .yaml file",
        classification: Classification::Structural,
        check: Some(contracts_schemas_dir_has_files),
    },
    Rule {
        id: "contracts.refs-resolve",
        description: "$ref pointers in OpenAPI/AsyncAPI files resolve to existing schema files",
        classification: Classification::Structural,
        check: Some(contracts_refs_resolve),
    },
    Rule {
        id: "contracts.schema-metadata",
        description: "JSON Schema files have $id, title, and description fields",
        classification: Classification::Structural,
        check: Some(contracts_schema_metadata),
    },
];

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;

    fn brief_ctx<'a>(
        slice_dir: &'a Path, specs_dir: &'a Path, content: &'a str,
    ) -> BriefContext<'a> {
        BriefContext {
            id: "contracts",
            content,
            parsed_spec: None,
            tasks: None,
            slice_dir,
            specs_dir,
        }
    }

    #[test]
    fn schemas_dir_has_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let schemas = dir.path().join("contracts").join("schemas");
        fs::create_dir_all(&schemas).unwrap();
        fs::write(schemas.join("user.yaml"), "$id: urn:test\ntitle: U\ndescription: d\n").unwrap();

        let specs_dir = dir.path().join("specs");
        let ctx = brief_ctx(dir.path(), &specs_dir, "");
        assert_eq!(contracts_schemas_dir_has_files(&ctx), RuleOutcome::Pass);
    }

    #[test]
    fn schemas_dir_empty_fails() {
        let dir = tempfile::tempdir().unwrap();
        let schemas = dir.path().join("contracts").join("schemas");
        fs::create_dir_all(&schemas).unwrap();

        let specs_dir = dir.path().join("specs");
        let ctx = brief_ctx(dir.path(), &specs_dir, "");
        assert!(matches!(contracts_schemas_dir_has_files(&ctx), RuleOutcome::Fail { .. }));
    }

    #[test]
    fn schemas_dir_missing_fails() {
        let dir = tempfile::tempdir().unwrap();
        let specs_dir = dir.path().join("specs");
        let ctx = brief_ctx(dir.path(), &specs_dir, "");
        match contracts_schemas_dir_has_files(&ctx) {
            RuleOutcome::Fail { detail } => {
                assert!(detail.contains("not found"), "got: {detail}");
            }
            other @ RuleOutcome::Pass => panic!("expected Fail, got {other:?}"),
        }
    }

    #[test]
    fn refs_resolve_ok() {
        let dir = tempfile::tempdir().unwrap();
        let contracts = dir.path().join("contracts");
        let http = contracts.join("http");
        let schemas = contracts.join("schemas");
        fs::create_dir_all(&http).unwrap();
        fs::create_dir_all(&schemas).unwrap();
        fs::write(schemas.join("user.yaml"), "title: User\n").unwrap();
        fs::write(
            http.join("api.yaml"),
            "openapi: '3.1.0'\npaths:\n  /users:\n    get:\n      schema:\n        $ref: \"../schemas/user.yaml\"\n",
        ).unwrap();

        let specs_dir = dir.path().join("specs");
        let ctx = brief_ctx(dir.path(), &specs_dir, "");
        assert_eq!(contracts_refs_resolve(&ctx), RuleOutcome::Pass);
    }

    #[test]
    fn refs_resolve_broken() {
        let dir = tempfile::tempdir().unwrap();
        let contracts = dir.path().join("contracts");
        let http = contracts.join("http");
        fs::create_dir_all(&http).unwrap();
        fs::write(
            http.join("api.yaml"),
            "openapi: '3.1.0'\npaths:\n  /users:\n    get:\n      schema:\n        $ref: \"../schemas/nonexistent.yaml\"\n",
        ).unwrap();

        let specs_dir = dir.path().join("specs");
        let ctx = brief_ctx(dir.path(), &specs_dir, "");
        match contracts_refs_resolve(&ctx) {
            RuleOutcome::Fail { detail } => {
                assert!(detail.contains("nonexistent.yaml"), "got: {detail}");
            }
            other @ RuleOutcome::Pass => panic!("expected Fail, got {other:?}"),
        }
    }

    #[test]
    fn refs_resolve_no_http_dir() {
        let dir = tempfile::tempdir().unwrap();
        let specs_dir = dir.path().join("specs");
        let ctx = brief_ctx(dir.path(), &specs_dir, "");
        assert_eq!(contracts_refs_resolve(&ctx), RuleOutcome::Pass);
    }

    #[test]
    fn schema_metadata_complete() {
        let dir = tempfile::tempdir().unwrap();
        let schemas = dir.path().join("contracts").join("schemas");
        fs::create_dir_all(&schemas).unwrap();
        fs::write(
            schemas.join("user.yaml"),
            "$id: urn:specify:schemas/user\ntitle: User\ndescription: A user entity.\ntype: object\n",
        ).unwrap();

        let specs_dir = dir.path().join("specs");
        let ctx = brief_ctx(dir.path(), &specs_dir, "");
        assert_eq!(contracts_schema_metadata(&ctx), RuleOutcome::Pass);
    }

    #[test]
    fn schema_metadata_missing_id() {
        let dir = tempfile::tempdir().unwrap();
        let schemas = dir.path().join("contracts").join("schemas");
        fs::create_dir_all(&schemas).unwrap();
        fs::write(
            schemas.join("user.yaml"),
            "title: User\ndescription: A user entity.\ntype: object\n",
        )
        .unwrap();

        let specs_dir = dir.path().join("specs");
        let ctx = brief_ctx(dir.path(), &specs_dir, "");
        match contracts_schema_metadata(&ctx) {
            RuleOutcome::Fail { detail } => {
                assert!(detail.contains("missing $id"), "got: {detail}");
            }
            other @ RuleOutcome::Pass => panic!("expected Fail, got {other:?}"),
        }
    }

    #[test]
    fn schema_metadata_empty_title() {
        let dir = tempfile::tempdir().unwrap();
        let schemas = dir.path().join("contracts").join("schemas");
        fs::create_dir_all(&schemas).unwrap();
        fs::write(
            schemas.join("user.yaml"),
            "$id: urn:test\ntitle: \"\"\ndescription: A user.\ntype: object\n",
        )
        .unwrap();

        let specs_dir = dir.path().join("specs");
        let ctx = brief_ctx(dir.path(), &specs_dir, "");
        match contracts_schema_metadata(&ctx) {
            RuleOutcome::Fail { detail } => {
                assert!(detail.contains("missing or empty title"), "got: {detail}");
            }
            other @ RuleOutcome::Pass => panic!("expected Fail, got {other:?}"),
        }
    }

    #[test]
    fn schema_metadata_no_dir() {
        let dir = tempfile::tempdir().unwrap();
        let specs_dir = dir.path().join("specs");
        let ctx = brief_ctx(dir.path(), &specs_dir, "");
        assert_eq!(contracts_schema_metadata(&ctx), RuleOutcome::Pass);
    }
}
