//! Binding-list loading at the crate's public surface: argv entries
//! and the operator-owned `sources.toml` carrier.

use emery_engine::sources::{BindingContent, SourceBinding, bindings};

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}

fn write_sources(body: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("sources.toml");
    std::fs::write(&path, body).expect("write sources.toml");
    let path = path.to_str().expect("utf-8 path").to_string();
    (dir, path)
}

fn from_file(body: &str) -> Result<Vec<SourceBinding>, emery_error::Error> {
    let (_dir, path) = write_sources(body);
    bindings(&[], &[], Some(&path))
}

#[test]
fn argv_bindings() {
    let bound = bindings(&strings(&["./adapters/docs.wasm"]), &strings(&["intent=Ship it."]), None)
        .expect("argv bindings load");
    assert_eq!(
        bound,
        [
            SourceBinding {
                key: "docs".to_string(),
                adapter: "./adapters/docs.wasm".to_string(),
                content: BindingContent::Workspace(".".to_string()),
            },
            SourceBinding {
                key: "intent".to_string(),
                adapter: "intent".to_string(),
                content: BindingContent::Value("Ship it.".to_string()),
            },
        ]
    );
}

#[test]
fn no_sources_refused() {
    let err = bindings(&[], &[], None).expect_err("an empty binding list is refused");
    assert!(err.to_string().contains("specify-source-required"), "{err}");
}

#[test]
fn duplicate_key_refused() {
    let err = bindings(&strings(&["docs", "docs"]), &[], None)
        .expect_err("the same key must not bind twice");
    assert!(err.to_string().contains("specify-source-duplicate"), "{err}");
}

#[test]
fn malformed_value_refused() {
    let err =
        bindings(&[], &strings(&["no-equals"]), None).expect_err("`--value` needs `<adapter>=`");
    assert!(err.to_string().contains("--value"), "{err}");
}

#[test]
fn mixing_argv_and_file_refused() {
    for (adapters, values) in
        [(strings(&["docs"]), Vec::new()), (Vec::new(), strings(&["intent=text"]))]
    {
        let err = bindings(&adapters, &values, Some("sources.toml"))
            .expect_err("`--sources` carries the whole binding list");
        assert!(
            matches!(
                err,
                emery_error::Error::Argument {
                    flag: "--sources",
                    ..
                }
            ),
            "{err}"
        );
    }
}

#[test]
fn file_bindings() {
    let (_dir, path) = write_sources(
        r#"
[sources.docs]
adapter = "emery:documentation@1.2.0"

[sources.api-surface]
adapter = "typescript"
path = "packages/api/src"

[sources.intent]
adapter = "intent@1.0.0"
value = "Ship a location-independent spec generator."

[sources.custom]
adapter = "./adapters/custom.wasm"
"#,
    );
    let bound = bindings(&[], &[], Some(&path)).expect("file bindings load");

    // Table keys are the binding keys, in sorted order.
    let keys: Vec<&str> = bound.iter().map(|binding| binding.key.as_str()).collect();
    assert_eq!(keys, ["api-surface", "custom", "docs", "intent"]);
    assert_eq!(bound[0].content, BindingContent::Workspace(anchored(&path, "packages/api/src")));
    assert_eq!(
        bound[1].adapter,
        anchored(&path, "adapters/custom.wasm"),
        "a local component resolves relative to the file"
    );
    assert_eq!(bound[2].adapter, "emery:documentation@1.2.0");
    assert_eq!(
        bound[2].content,
        BindingContent::Workspace(".".to_string()),
        "omitted location is the workspace lend at `.`"
    );
    assert_eq!(
        bound[3].content,
        BindingContent::Value("Ship a location-independent spec generator.".to_string())
    );
}

// File-relative entries anchor at the file's own directory, so the
// file works from any invocation directory.
fn anchored(sources_path: &str, relative: &str) -> String {
    let dir = std::path::Path::new(sources_path).parent().expect("parent");
    dir.join(relative).display().to_string()
}

#[test]
fn two_location_keys_refused() {
    let err = from_file(
        "[sources.docs]\nadapter = \"documentation\"\npath = \"docs\"\nvalue = \"text\"\n",
    )
    .expect_err("exactly one location key is allowed");
    assert!(
        matches!(
            err,
            emery_error::Error::Argument {
                flag: "--sources",
                ..
            }
        ),
        "{err}"
    );
    assert!(err.to_string().contains("more than one"), "{err}");
}

#[test]
fn remote_locations_reserved() {
    for location in [
        "git = \"https://github.com/acme/api@v2.3.0\"",
        "url = \"https://example.com/spec/openapi.yaml\"",
    ] {
        let err =
            from_file(&format!("[sources.upstream]\nadapter = \"documentation\"\n{location}\n"))
                .expect_err("remote reads are reserved, not implemented");
        assert!(err.to_string().contains("source-remote-unsupported"), "{err}");
    }
}

#[test]
fn cargo_source_id_form_refused() {
    let err = from_file(
        "[sources.upstream]\nadapter = \"documentation\"\ngit = \"git+https://github.com/acme/api#deadbeef\"\n",
    )
    .expect_err("Cargo's machine-written source-id form is the wrong precedent");
    assert!(
        matches!(
            err,
            emery_error::Error::Argument {
                flag: "--sources",
                ..
            }
        ),
        "{err}"
    );
}

#[test]
fn relative_paths_normalise() {
    let (_dir, path) =
        write_sources("[sources.docs]\nadapter = \"documentation\"\npath = \"nested/../docs\"\n");
    let bound = bindings(&[], &[], Some(&path)).expect("bindings load");
    assert_eq!(
        bound[0].content,
        BindingContent::Workspace(anchored(&path, "docs")),
        "`..` folds lexically against the file's directory"
    );
}

#[test]
fn malformed_toml_refused() {
    let err = from_file("not toml [").expect_err("parse failures are typed");
    assert!(err.to_string().contains("sources-toml-malformed"), "{err}");
}

#[test]
fn unknown_key_refused() {
    let err = from_file("[sources.docs]\nadapter = \"documentation\"\nbranch = \"main\"\n")
        .expect_err("unknown keys fail closed");
    assert!(err.to_string().contains("sources-toml-malformed"), "{err}");
}

#[test]
fn empty_file_refused() {
    let err = from_file("").expect_err("a sources file with no entries is refused");
    assert!(err.to_string().contains("specify-source-required"), "{err}");
}

#[test]
fn missing_file_refused() {
    let err = bindings(&[], &[], Some("/nonexistent/sources.toml"))
        .expect_err("an unreadable file is a typed filesystem error");
    assert!(err.to_string().contains("filesystem-read"), "{err}");
}
