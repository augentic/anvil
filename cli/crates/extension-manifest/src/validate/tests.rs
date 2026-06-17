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

#[test]
fn validate_reports_chunk_one_rules() {
    let tool = Extension {
        name: "BadName".to_string(),
        version: "not-semver".to_string(),
        source: ExtensionSource::HttpsUri("oci://registry/tool.wasm".to_string()),
        sha256: Some("ABC".to_string()),
        permissions: ExtensionPermissions {
            read: vec!["relative/../*.txt".to_string(), "$CAPABILITY_DIR/templates".to_string()],
            write: vec!["$PROJECT_DIR/.specify/project.yaml".to_string()],
        },
    };

    let results = tool.validate_structure(&project_scope());
    let ids = fail_rule_ids(&results);
    assert!(ids.contains(&RULE_NAME_FORMAT));
    assert!(ids.contains(&RULE_VERSION_SEMVER));
    assert!(ids.contains(&RULE_SOURCE_SUPPORTED));
    assert!(ids.contains(&RULE_SHA256_FORMAT));
    assert!(ids.contains(&RULE_PERMISSION_PATH_FORM));
    assert!(ids.contains(&RULE_LIFECYCLE_WRITE_DENIED));
    assert!(ids.contains(&RULE_CAPABILITY_DIR_SCOPE));
}

#[test]
fn package_validation_reports_rules() {
    let tool = Extension {
        name: "contract".to_string(),
        version: "v1".to_string(),
        source: ExtensionSource::Package(PackageRequest {
            namespace: "other".to_string(),
            name: "contract".to_string(),
            version: "v1".to_string(),
        }),
        sha256: None,
        permissions: ExtensionPermissions::default(),
    };

    let results = tool.validate_structure(&project_scope());
    let ids = fail_rule_ids(&results);
    assert!(ids.contains(&RULE_VERSION_SEMVER));
    assert!(ids.contains(&RULE_PACKAGE_NAMESPACE));
    assert!(ids.contains(&RULE_PACKAGE_VERSION));
}

#[test]
fn scalar_package_form_no_longer_parses() {
    // The first-party scalar shorthand (`specify:<name>@<ver>`) and its
    // embedded permissions catalog are retired (RFC-48 D10): a tool must
    // be a full object with its own `source` and `permissions`.
    let parsed =
        serde_saphyr::from_str::<ExtensionManifest>("tools:\n  - \"specify:contract@1.2.3\"\n");
    assert!(parsed.is_err(), "the scalar first-party form must no longer deserialize");
}

#[test]
fn root_write_valid_for_root_outputs() {
    let mut tool = valid_tool("contract");
    tool.permissions.write = vec!["$PROJECT_DIR".to_string()];

    let results = tool.validate_structure(&project_scope());
    assert!(results.is_empty(), "{results:?}");
}

#[test]
fn validate_rejects_duplicate_names() {
    let manifest = ExtensionManifest {
        tools: vec![valid_tool("contract"), valid_tool("contract")],
    };
    let results = manifest.validate_structure(&project_scope());
    assert!(fail_rule_ids(&results).contains(&RULE_NAME_UNIQUE));
}

#[test]
fn project_scope_rejects_capability_dir() {
    let mut tool = valid_tool("contract");
    tool.permissions.read.push("$CAPABILITY_DIR/templates".to_string());

    let results = tool.validate_structure(&project_scope());
    assert!(fail_rule_ids(&results).contains(&RULE_CAPABILITY_DIR_SCOPE));
}

#[test]
fn valid_tool_passes_structure_validation() {
    let results = valid_tool("contract").validate_structure(&project_scope());
    assert!(results.is_empty(), "{results:?}");
}

#[test]
fn schema_rejects_invalid_shapes() {
    let schema: serde_json::Value =
        serde_json::from_str(EXTENSION_JSON_SCHEMA).expect("schema parses");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    let cases = [
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
    ];

    for case in cases {
        assert!(!validator.is_valid(&case), "schema should reject invalid case: {case}");
    }
}

#[test]
fn schema_accepts_root_write() {
    let schema: serde_json::Value =
        serde_json::from_str(EXTENSION_JSON_SCHEMA).expect("schema parses");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    let case = json!({
        "tools": [{
            "name": "root-writer",
            "version": "1.0.0",
            "source": "/tmp/a.wasm",
            "permissions": { "write": ["$PROJECT_DIR"] }
        }]
    });

    assert!(validator.is_valid(&case), "schema should allow project-root writes: {case}");
}

#[test]
fn schema_accepts_object_rejects_scalar() {
    let schema: serde_json::Value =
        serde_json::from_str(EXTENSION_JSON_SCHEMA).expect("schema parses");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    // The object form with a package `source:` is still valid.
    let object = json!({ "tools": [{ "name": "contract", "version": "1.2.3", "source": "specify:contract@1.2.3" }] });
    assert!(validator.is_valid(&object), "schema should accept object package case: {object}");
    // The retired scalar first-party shorthand no longer validates.
    let scalar = json!({ "tools": ["specify:contract@1.2.3"] });
    assert!(!validator.is_valid(&scalar), "schema must reject the retired scalar form: {scalar}");
}

#[test]
fn schema_accepts_template_sources() {
    let schema: serde_json::Value =
        serde_json::from_str(EXTENSION_JSON_SCHEMA).expect("schema parses");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    for case in [
        json!({ "tools": [{ "name": "vectis", "version": "0.3.0", "source": "$PROJECT_DIR/../cli/target/vectis.wasm" }] }),
        json!({ "tools": [{ "name": "vectis", "version": "0.3.0", "source": "$PROJECT_DIR/tools/vectis.wasm" }] }),
        json!({ "tools": [{ "name": "vectis", "version": "0.3.0", "source": "$CAPABILITY_DIR/bin/vectis.wasm" }] }),
    ] {
        assert!(validator.is_valid(&case), "schema should accept template source: {case}");
    }
}

#[test]
fn template_source_passes_validation() {
    let tool = Extension {
        name: "vectis".to_string(),
        version: "0.3.0".to_string(),
        source: ExtensionSource::TemplatePath("$PROJECT_DIR/../cli/target/vectis.wasm".to_string()),
        sha256: None,
        permissions: ExtensionPermissions {
            read: vec!["$PROJECT_DIR".to_string()],
            write: Vec::new(),
        },
    };
    let results = tool.validate_structure(&project_scope());
    assert!(results.is_empty(), "{results:?}");
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

#[test]
fn template_capability_dir_rejected() {
    let tool = Extension {
        name: "vectis".to_string(),
        version: "0.3.0".to_string(),
        source: ExtensionSource::TemplatePath("$CAPABILITY_DIR/bin/vectis.wasm".to_string()),
        sha256: None,
        permissions: ExtensionPermissions::default(),
    };
    let results = tool.validate_structure(&project_scope());
    assert!(fail_rule_ids(&results).contains(&RULE_SOURCE_CAPABILITY_DIR_SCOPE));
}
