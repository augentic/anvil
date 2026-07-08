//! The project-scope `tools[]` declaration shape on `project.yaml`
//! (`workflow_lib::config::tools`): wire round-trips, the
//! `parse_wire` source classifier, and the permissive
//! `PackageRequest::parse` split points.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use workflow_lib::config::tools::{
    Extension, ExtensionPermissions, ExtensionSource, PackageRequest,
};

/// The `tools:` mapping as it appears on `project.yaml`.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Doc {
    tools: Vec<Extension>,
}

#[test]
fn round_trips_all_sources() {
    let doc = Doc {
        tools: vec![
            Extension {
                name: "local-tool".to_string(),
                version: "1.0.0".to_string(),
                source: ExtensionSource::LocalPath(PathBuf::from("/opt/specify/local.wasm")),
                sha256: None,
                permissions: ExtensionPermissions::default(),
            },
            Extension {
                name: "file-tool".to_string(),
                version: "1.0.1".to_string(),
                source: ExtensionSource::FileUri("file:///opt/specify/file.wasm".to_string()),
                sha256: None,
                permissions: ExtensionPermissions {
                    read: vec!["$PROJECT_DIR/contracts".to_string()],
                    write: Vec::new(),
                },
            },
            Extension {
                name: "https-tool".to_string(),
                version: "1.0.2".to_string(),
                source: ExtensionSource::HttpsUri(
                    "https://example.com/specify/https.wasm".to_string(),
                ),
                sha256: Some(
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
                ),
                permissions: ExtensionPermissions {
                    read: Vec::new(),
                    write: vec!["$PROJECT_DIR/generated".to_string()],
                },
            },
        ],
    };

    let yaml = serde_saphyr::to_string(&doc).expect("serialize tools");
    assert!(yaml.contains("source: /opt/specify/local.wasm"));
    assert!(yaml.contains("source: file:///opt/specify/file.wasm"));
    assert!(yaml.contains("source: https://example.com/specify/https.wasm"));

    let parsed: Doc = serde_saphyr::from_str(&yaml).expect("parse tools");
    assert_eq!(parsed, doc);
}

// serde rejects both a tool whose `source:` is an unsupported wire
// string (the `TryFrom` error propagates) and the unsupported top-level
// scalar shorthand (a tool is always a full object with its own
// `source` and `permissions`).
#[test]
fn serde_rejects() {
    let rejected = [
        "tools:\n  - name: bad\n    version: 1.0.0\n    source: relative.wasm\n",
        "tools:\n  - \"specify:contract@1.2.3\"\n",
        "tools:\n  - \"other:helper@latest\"\n",
    ];
    for yaml in rejected {
        assert!(serde_saphyr::from_str::<Doc>(yaml).is_err(), "must be rejected: {yaml}");
    }
}

// A package or template `source:` string inside the object form parses
// to its own variant and serializes back to the same wire string.
#[test]
fn package_and_template_sources_round_trip() {
    let package: Doc = serde_saphyr::from_str(
        "tools:\n  - name: contract\n    version: 1.2.3\n    source: \"specify:contract@1.2.3\"\n",
    )
    .expect("parse package source");
    let tool = &package.tools[0];
    assert_eq!(tool.name, "contract");
    assert!(matches!(
        &tool.source,
        ExtensionSource::Package(package)
            if package.namespace == "specify"
                && package.name == "contract"
                && package.version == "1.2.3"
    ));
    assert_eq!(tool.permissions, ExtensionPermissions::default());

    let template: Doc = serde_saphyr::from_str(
        "tools:\n  - name: demo-tool\n    version: 0.3.0\n    source: $PROJECT_DIR/tools/demo-tool.wasm\n",
    )
    .expect("parse template source");
    let tool = &template.tools[0];
    assert!(
        matches!(&tool.source, ExtensionSource::TemplatePath(t) if t == "$PROJECT_DIR/tools/demo-tool.wasm"),
    );
    let yaml = serde_saphyr::to_string(&template).expect("serialize template source");
    assert!(yaml.contains("source: $PROJECT_DIR/tools/demo-tool.wasm"), "{yaml}");
}

// `parse_wire` is the single classifier every wire string flows
// through; one drift in the prefix order (e.g. classifying a
// `$PROJECT_DIR` template as a package because it contains no `:`)
// would silently mis-route a source. Pin each arm, including the
// Windows-absolute branch that string-prefix checks alone miss and the
// template-variable boundary (`$PROJECT_DIRX` is not a template).
#[test]
fn parse_wire_classifies_each_scheme() {
    assert!(matches!(
        ExtensionSource::parse_wire("https://example.com/t.wasm"),
        Ok(ExtensionSource::HttpsUri(_))
    ));
    assert!(matches!(
        ExtensionSource::parse_wire("file:///opt/t.wasm"),
        Ok(ExtensionSource::FileUri(_))
    ));
    assert!(matches!(
        ExtensionSource::parse_wire("/opt/specify/t.wasm"),
        Ok(ExtensionSource::LocalPath(_))
    ));
    assert!(matches!(
        ExtensionSource::parse_wire(r"C:\tools\t.wasm"),
        Ok(ExtensionSource::LocalPath(_))
    ));
    assert!(matches!(
        ExtensionSource::parse_wire("C:/tools/t.wasm"),
        Ok(ExtensionSource::LocalPath(_))
    ));
    assert!(matches!(
        ExtensionSource::parse_wire("$PROJECT_DIR/tools/t.wasm"),
        Ok(ExtensionSource::TemplatePath(_))
    ));
    assert!(matches!(
        ExtensionSource::parse_wire("$CAPABILITY_DIR"),
        Ok(ExtensionSource::TemplatePath(_))
    ));
    assert!(matches!(
        ExtensionSource::parse_wire("$PROJECT_DIRX/t.wasm"),
        Err(_)
    ));
    assert!(matches!(
        ExtensionSource::parse_wire("specify:contract@1.0.0"),
        Ok(ExtensionSource::Package(_))
    ));
    ExtensionSource::parse_wire("relative/t.wasm").expect_err("relative path is unclassifiable");
}

// `PackageRequest::parse` is deliberately permissive; verify the split
// points so a refactor of the `@` / `:` handling cannot quietly swap
// which field captures a missing separator.
#[test]
fn package_request_parse_edges() {
    let full = PackageRequest::parse("specify:contract@1.2.3");
    assert_eq!(
        (full.namespace.as_str(), full.name.as_str(), full.version.as_str()),
        ("specify", "contract", "1.2.3")
    );

    let no_version = PackageRequest::parse("specify:contract");
    assert_eq!(
        (no_version.namespace.as_str(), no_version.name.as_str(), no_version.version.as_str()),
        ("specify", "contract", "")
    );

    let no_namespace = PackageRequest::parse("contract@1.2.3");
    assert_eq!(
        (no_namespace.namespace.as_str(), no_namespace.name.as_str(), no_namespace.version.as_str()),
        ("", "contract", "1.2.3")
    );

    // The version split happens before the namespace split, so a
    // second `@` stays inside the version segment.
    let extra_at = PackageRequest::parse("specify:contract@1@2");
    assert_eq!(extra_at.version, "1@2");
}
