use serde_json::json;

use super::*;
use crate::{ExtensionPermissions, ExtensionSource, PackageRequest};

fn project_scope() -> ExtensionScope {
    ExtensionScope::Project {
        project_name: "demo".to_string(),
    }
}

fn valid_tool(name: &str) -> Extension {
    Extension {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        source: ExtensionSource::HttpsUri("https://example.com/tool.wasm".to_string()),
        sha256: Some(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        ),
        permissions: ExtensionPermissions {
            read: vec!["$PROJECT_DIR/contracts".to_string()],
            write: vec!["$PROJECT_DIR/generated".to_string()],
        },
    }
}

fn fail_rule_ids(results: &[Diagnostic]) -> Vec<&str> {
    results.iter().filter_map(|d| d.rule_id.as_deref()).collect()
}

// `validate_structure` flags one deterministic violation per failing rule
// and emits nothing for a structurally valid tool. The maximal multi-rule
// failure, the package rule set, the project-scope capability-dir read, the
// template capability-dir source, and the three valid shapes (https,
// project-root write, template) collapse into one matrix.
#[test]
fn validate_structure_matrix() {
    let scope = project_scope();
    let contains = |tool: &Extension, expected: &[&str]| {
        let results = tool.validate_structure(&scope);
        let ids = fail_rule_ids(&results);
        for rule in expected {
            assert!(ids.contains(rule), "expected {rule} in {ids:?}");
        }
    };
    let valid = |tool: &Extension| {
        let results = tool.validate_structure(&scope);
        assert!(results.is_empty(), "{results:?}");
    };

    // A maximally-broken tool flags every chunk-one rule at once.
    contains(
        &Extension {
            name: "BadName".to_string(),
            version: "not-semver".to_string(),
            source: ExtensionSource::HttpsUri("oci://registry/tool.wasm".to_string()),
            sha256: Some("ABC".to_string()),
            permissions: ExtensionPermissions {
                read: vec![
                    "relative/../*.txt".to_string(),
                    "$CAPABILITY_DIR/templates".to_string(),
                ],
                write: vec!["$PROJECT_DIR/.specify/project.yaml".to_string()],
            },
        },
        &[
            RULE_NAME_FORMAT,
            RULE_VERSION_SEMVER,
            RULE_SOURCE_SUPPORTED,
            RULE_SHA256_FORMAT,
            RULE_PERMISSION_PATH_FORM,
            RULE_LIFECYCLE_WRITE_DENIED,
            RULE_CAPABILITY_DIR_SCOPE,
        ],
    );

    // A non-`specify` package with a leading-v version flags the package set.
    contains(
        &Extension {
            name: "contract".to_string(),
            version: "v1".to_string(),
            source: ExtensionSource::Package(PackageRequest {
                namespace: "other".to_string(),
                name: "contract".to_string(),
                version: "v1".to_string(),
            }),
            sha256: None,
            permissions: ExtensionPermissions::default(),
        },
        &[RULE_VERSION_SEMVER, RULE_PACKAGE_NAMESPACE, RULE_PACKAGE_VERSION],
    );

    // A project-scope read of `$CAPABILITY_DIR` is out of scope.
    let mut cap_read = valid_tool("contract");
    cap_read.permissions.read.push("$CAPABILITY_DIR/templates".to_string());
    contains(&cap_read, &[RULE_CAPABILITY_DIR_SCOPE]);

    // A project-scope template *source* referencing `$CAPABILITY_DIR`.
    contains(
        &Extension {
            name: "demo-tool".to_string(),
            version: "0.3.0".to_string(),
            source: ExtensionSource::TemplatePath("$CAPABILITY_DIR/bin/demo-tool.wasm".to_string()),
            sha256: None,
            permissions: ExtensionPermissions::default(),
        },
        &[RULE_SOURCE_CAPABILITY_DIR_SCOPE],
    );

    // Valid shapes emit nothing: the canonical https tool, a project-root
    // write, and a template source with a `$PROJECT_DIR` read.
    valid(&valid_tool("contract"));
    let mut root_write = valid_tool("contract");
    root_write.permissions.write = vec!["$PROJECT_DIR".to_string()];
    valid(&root_write);
    valid(&Extension {
        name: "demo-tool".to_string(),
        version: "0.3.0".to_string(),
        source: ExtensionSource::TemplatePath(
            "$PROJECT_DIR/../cli/target/demo-tool.wasm".to_string(),
        ),
        sha256: None,
        permissions: ExtensionPermissions {
            read: vec!["$PROJECT_DIR".to_string()],
            write: Vec::new(),
        },
    });
}

#[test]
fn validate_rejects_duplicate_names() {
    let manifest = ExtensionManifest {
        tools: vec![valid_tool("contract"), valid_tool("contract")],
    };
    let results = manifest.validate_structure(&project_scope());
    assert!(fail_rule_ids(&results).contains(&RULE_NAME_UNIQUE));
}

// The embedded EXTENSION_JSON_SCHEMA is the first gate every `tools:` block
// passes; it must reject malformed shapes (bad name / version / source /
// sha256 / permissions / duplicates / unknown keys and the unsupported scalar
// shorthand) and accept project-root writes, the object package form, and
// template sources. Compile once, drive accept and reject case lists.
#[test]
fn schema_validation_matrix() {
    let schema: serde_json::Value =
        serde_json::from_str(EXTENSION_JSON_SCHEMA).expect("schema parses");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");

    let rejected = [
        json!({ "tools": [{ "name": "Bad", "version": "1.0.0", "source": "/tmp/a.wasm" }] }),
        json!({ "tools": [{ "name": "bad", "version": "one", "source": "/tmp/a.wasm" }] }),
        json!({ "tools": [{ "name": "bad", "version": "1.0.0", "source": "relative.wasm" }] }),
        json!({ "tools": [{ "name": "bad", "version": "1.0.0", "source": "oci://x" }] }),
        json!({ "tools": ["other:bad@1.0.0"] }),
        json!({ "tools": ["specify:bad@v1.0.0"] }),
        json!({ "tools": ["specify:bad@latest"] }),
        json!({ "tools": [{ "name": "bad", "version": "1.0.0", "source": "/tmp/a.wasm", "sha256": "ABC" }] }),
        json!({ "tools": [{ "name": "bad", "version": "1.0.0", "source": "/tmp/a.wasm", "permissions": { "read": ["$PROJECT_DIR/../x"] } }] }),
        json!({ "tools": [{ "name": "bad", "version": "1.0.0", "source": "/tmp/a.wasm", "permissions": { "write": ["$PROJECT_DIR/.specify/project.yaml"] } }] }),
        json!({ "tools": [
            { "name": "bad", "version": "1.0.0", "source": "/tmp/a.wasm" },
            { "name": "bad", "version": "1.0.0", "source": "/tmp/a.wasm" }
        ] }),
        json!({ "tools": [{ "name": "bad", "version": "1.0.0", "source": "/tmp/a.wasm", "permissions": { "read": [], "exec": [] } }] }),
        // The scalar first-party shorthand does not validate.
        json!({ "tools": ["specify:contract@1.2.3"] }),
    ];
    for case in &rejected {
        assert!(!validator.is_valid(case), "schema should reject: {case}");
    }

    let accepted = [
        // Project-root writes.
        json!({ "tools": [{ "name": "root-writer", "version": "1.0.0", "source": "/tmp/a.wasm", "permissions": { "write": ["$PROJECT_DIR"] } }] }),
        // Object form with a package source.
        json!({ "tools": [{ "name": "contract", "version": "1.2.3", "source": "specify:contract@1.2.3" }] }),
        // Template sources ($PROJECT_DIR and $CAPABILITY_DIR).
        json!({ "tools": [{ "name": "demo-tool", "version": "0.3.0", "source": "$PROJECT_DIR/../cli/target/demo-tool.wasm" }] }),
        json!({ "tools": [{ "name": "demo-tool", "version": "0.3.0", "source": "$PROJECT_DIR/tools/demo-tool.wasm" }] }),
        json!({ "tools": [{ "name": "demo-tool", "version": "0.3.0", "source": "$CAPABILITY_DIR/bin/demo-tool.wasm" }] }),
    ];
    for case in &accepted {
        assert!(validator.is_valid(case), "schema should accept: {case}");
    }
}

// `targets_lifecycle_state` is the textual guard that keeps tools out
// of `.specify` state. It must match `.specify` as a path *component* —
// a sibling like `.specify-data` is legitimate and must pass — while
// still catching nested and backslash-separated targets. A prefix-only
// implementation would wrongly flag `.specify-data`.
#[test]
fn lifecycle_state_boundary() {
    assert!(targets_lifecycle_state("$PROJECT_DIR/.specify"));
    assert!(targets_lifecycle_state("$PROJECT_DIR/.specify/project.yaml"));
    assert!(targets_lifecycle_state(r"$PROJECT_DIR\.specify\slices"));
    assert!(!targets_lifecycle_state("$PROJECT_DIR/.specify-data"));
    assert!(!targets_lifecycle_state("$PROJECT_DIR/generated"));
    assert!(!targets_lifecycle_state("$PROJECT_DIR/.specification"));
}
