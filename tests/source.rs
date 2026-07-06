//! Integration tests for the `specify source` subcommand tree
//! (`resolve` — `survey` and `extract` are guest-routed; see
//! `tests/guest.rs`). Shared helpers live in [`common`].

mod resolve {
    //! Integration tests for `specify source resolve`.
    //!
    //! Mirrors the source-adapter loader exposed by
    //! `crates/workflow/src/adapter/`. The CLI verb is a thin
    //! `SourceAdapter::resolve(adapter_ref, project_dir)` wrapper; the
    //! cases below pin the wire shape skill bodies and downstream
    //! callers rely on.

    use std::fs;

    use crate::common::{
        Project, expected_cache_dir, fixture_source_component, parse_stderr, parse_stdout,
        specify_cmd,
    };

    /// Stage the echo source guest as `<project-cache>/components/<name>.wasm`
    /// so the bare-name probe resolves it (the same mirror slot `specify
    /// init <component>` populates).
    fn stage_cached_source(project: &Project, name: &str) {
        let dir = expected_cache_dir(project.root()).join("components");
        fs::create_dir_all(&dir).expect("create component cache dir");
        fs::copy(fixture_source_component(name), dir.join(format!("{name}.wasm")))
            .expect("stage cached source component");
    }

    #[test]
    fn resolve_cached_returns_component() {
        let project = Project::init();
        stage_cached_source(&project, "typescript");

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "source", "resolve", "typescript"])
            .arg("--project-dir")
            .arg(project.root())
            .assert()
            .success();

        let actual = parse_stdout(&assert.get_output().stdout, project.root());
        assert_eq!(actual["axis"], "sources");
        assert_eq!(actual["name"], "typescript");
        assert_eq!(actual["location"], "dev");
        let operations = actual["operations"].as_array().expect("operations array");
        let ops: Vec<&str> = operations.iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(ops, vec!["extract", "survey"]);
        let resolved = actual["resolved-path"].as_str().expect("resolved-path str");
        assert!(
            resolved.ends_with("components/typescript.wasm"),
            "resolved-path {resolved} must be the cached component file"
        );
        assert!(
            actual.get("briefs-dir").is_none(),
            "briefs-dir left the envelope with the compiled-in briefs cutover"
        );
    }

    #[test]
    fn resolve_missing_emits_not_found() {
        let project = Project::init();

        let assert = specify_cmd()
            .current_dir(project.root())
            .args(["--format", "json", "source", "resolve", "no-such-source"])
            .arg("--project-dir")
            .arg(project.root())
            .assert()
            .failure();
        let stderr = parse_stderr(&assert.get_output().stderr, project.root());
        assert_eq!(stderr["error"], "adapter-not-found");
        assert_eq!(stderr["exit-code"], 1);
    }
}
