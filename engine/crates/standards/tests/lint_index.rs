//! Integration tests for the `lint::index::build` workspace indexer and
//! the `WorkspaceModel` it produces (project + framework profiles,
//! scenario discovery, and DTO round-trip).

mod framework_indexer {
    //! Integration test for the `scan_profile: framework` indexer.
    //!
    //! Drives `lint::index::build` against the checked-in
    //! `fixtures/lint/framework_minimal/` tree, minting one relative
    //! symlink at test time (`agent-teams.md` → `docs/reference/review-team-protocol.md`)
    //! because committed relative symlinks survive `git` poorly across
    //! operating systems.
    //!
    //! Two invariants the framework profile owes:
    //!
    //! 1. The produced [`WorkspaceModel`] validates against the embedded
    //!    [`WORKSPACE_MODEL_JSON_SCHEMA`] under the framework profile —
    //!    every new framework-only entity family round-trips through the
    //!    schema.
    //! 2. Every framework extractor (`skill`, `adapter`, `marketplace`,
    //!    `brief`) emits at least one fact against the
    //!    minimal fixture, and the followed `agent-teams.md` symlink
    //!    surfaces both endpoints plus a SHA-256 of the resolved target's
    //!    bytes per the standards-layer contract §F1.

    use std::fs;
    use std::path::PathBuf;

    use serde_json::Value;
    use specify_schema::{ValidationStatus, WORKSPACE_MODEL_JSON_SCHEMA, validate_value};
    use specify_standards::lint::ScanProfile;
    use specify_standards::lint::index::build;
    use tempfile::TempDir;

    const FIXTURE_NAME: &str = "framework_minimal";

    fn crate_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn fixture_src() -> PathBuf {
        crate_root().join("tests/fixtures/lint").join(FIXTURE_NAME)
    }

    /// Stage the fixture into a tempdir and add the followable
    /// `agent-teams.md` symlink at runtime.
    fn stage_fixture() -> TempDir {
        let tempdir = tempfile::tempdir().expect("tempdir");
        crate::common::copy_dir(&fixture_src(), tempdir.path());

        // `agent-teams.md` symlink in `adapters/targets/omnia/prose/references/`
        // pointing at the canonical `docs/reference/review-team-protocol.md`.
        // The `references/` parent isn't checked in (git doesn't track empty
        // dirs); create it before placing the link so the cross-platform
        // symlink calls below have a valid parent.
        let link_dir = tempdir.path().join("adapters/targets/omnia/prose/references");
        fs::create_dir_all(&link_dir).expect("create link parent");
        let link_path = link_dir.join("agent-teams.md");
        let link_target = "../../../../../docs/reference/review-team-protocol.md";
        #[cfg(unix)]
        std::os::unix::fs::symlink(link_target, &link_path).expect("create unix symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(link_target, &link_path)
            .expect("create windows symlink");

        tempdir
    }

    fn assert_schema_valid(value: &Value) {
        let summaries = validate_value(
            value,
            WORKSPACE_MODEL_JSON_SCHEMA,
            "workspace-model",
            "framework-indexer fixture",
        );
        let failures: Vec<_> =
            summaries.iter().filter(|s| matches!(s.status, ValidationStatus::Fail)).collect();
        assert!(failures.is_empty(), "WorkspaceModel must validate; got {failures:#?}");
    }

    #[test]
    fn extractors_emit_facts() {
        let tempdir = stage_fixture();
        let model = build(tempdir.path(), ScanProfile::Framework, &[], &[]).expect("build ok");
        let value = serde_json::to_value(&model).expect("serialise");
        assert_schema_valid(&value);

        assert_eq!(model.scan_profile, ScanProfile::Framework);

        assert!(!model.skills.is_empty(), "skill extractor must emit at least one fact");
        let skill = &model.skills[0];
        assert_eq!(skill.name, "specify-init");
        assert_eq!(skill.plugin, "spec");
        assert!(skill.body_line_count.unwrap_or(0) >= 1);

        assert!(
            model.adapter_manifests.len() >= 2,
            "adapter extractor must emit one fact per `adapter.yaml` (sources + targets)"
        );
        let names: Vec<&str> = model.adapter_manifests.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"intent"));
        assert!(names.contains(&"omnia"));

        assert!(
            model.briefs.len() >= 2,
            "brief extractor must emit one fact per `briefs/*.md` (sources + targets)"
        );
        assert!(model.briefs.iter().any(|b| b.operation == "survey"));
        assert!(model.briefs.iter().any(|b| b.operation == "shape"));
        let survey_brief =
            model.briefs.iter().find(|b| b.operation == "survey").expect("survey brief");
        assert_eq!(
            survey_brief.sections,
            vec!["Inputs".to_string(), "Output contract".to_string()]
        );
    }

    #[test]
    fn symlink_records_endpoint_and_sha256() {
        let tempdir = stage_fixture();
        let model = build(tempdir.path(), ScanProfile::Framework, &[], &[]).expect("build ok");

        let symlink = model
            .symlinks
            .iter()
            .find(|s| s.path == "adapters/targets/omnia/prose/references/agent-teams.md")
            .expect("symlink fact recorded");
        assert!(!symlink.broken);
        assert_eq!(
            symlink.resolved_target.as_deref(),
            Some("docs/reference/review-team-protocol.md"),
            "framework symlink fact records the canonical endpoint"
        );
    }

    #[test]
    fn walk_byte_stable() {
        let tempdir = stage_fixture();
        let first = build(tempdir.path(), ScanProfile::Framework, &[], &[]).expect("first build");
        let second = build(tempdir.path(), ScanProfile::Framework, &[], &[]).expect("second build");
        let first_json = serde_json::to_string_pretty(&first).expect("first serialise");
        let second_json = serde_json::to_string_pretty(&second).expect("second serialise");
        assert_eq!(
            first_json, second_json,
            "two framework indexer runs must produce byte-identical JSON"
        );
    }
}

mod index_scenario {
    //! Integration test for the dedicated `scenario` discovery pass.
    //!
    //! Proves the scoped fact family: a staged `evals/scenarios/*.md`
    //! file is discovered into `model.scenarios` (with `id` / `fields`
    //! projected), and is kept OUT of `model.files` so no other rule's
    //! candidate set changes.

    use std::fs;
    use std::path::Path;

    use specify_standards::lint::ScanProfile;
    use specify_standards::lint::index::build;

    fn write_scenario(project: &Path, name: &str, id: &str) {
        let content = format!(
            "---\nid: {id}\nowner: spec\nkind: skill\nentrypoint: /spec:refine\nstages: [refine, build]\nisolation: fresh-project\nexpected-artifacts: [spec.md]\n---\n\nScenario ID: `{id}`\n"
        );
        let path = project.join(format!("evals/scenarios/{name}"));
        fs::create_dir_all(path.parent().expect("parent")).expect("scenario dir");
        fs::write(&path, content).expect("write scenario");
    }

    #[test]
    fn discovers_scenario_into_dedicated_family() {
        let tmp = tempfile::tempdir().expect("tmp");
        write_scenario(tmp.path(), "refine.md", "refine-happy-path");

        let model = build(tmp.path(), ScanProfile::Framework, &[], &[]).expect("framework build");

        let scenario = model
            .scenarios
            .iter()
            .find(|s| s.path == "evals/scenarios/refine.md")
            .expect("staged scenario appears in model.scenarios");
        assert_eq!(scenario.id.as_deref(), Some("refine-happy-path"));
        assert_eq!(scenario.stages, vec!["refine".to_string(), "build".to_string()]);
        assert_eq!(scenario.expected_artifacts, vec!["spec.md".to_string()]);
        assert_eq!(scenario.body_id.as_deref(), Some("refine-happy-path"));
        assert_eq!(scenario.fields.get("owner").and_then(|v| v.as_str()), Some("spec"));
    }

    #[test]
    fn scenario_file_is_kept_out_of_files() {
        let tmp = tempfile::tempdir().expect("tmp");
        write_scenario(tmp.path(), "refine.md", "refine-happy-path");

        let model = build(tmp.path(), ScanProfile::Framework, &[], &[]).expect("framework build");

        assert!(
            !model.files.iter().any(|f| f.path == "evals/scenarios/refine.md"),
            "eval scenario files must not enter model.files (zero blast radius)"
        );
    }
}

mod indexer_project {
    //! Integration test for the `WorkspaceModel` file scan project indexer.
    //!
    //! Drives `lint::index::build` against the checked-in
    //! `fixtures/lint/minimal/` tree, augmenting it at runtime with the
    //! few entries that cannot be committed cleanly (a `.gitignore`-ignored
    //! sibling, an `always-ignore`-globbed `target/` directory, and a
    //! relative symlink — committed symlinks are fragile across operating
    //! systems). The fixture's `README.md` documents the rationale.
    //!
    //! Two invariants are asserted:
    //!
    //! 1. The produced [`WorkspaceModel`] validates against the embedded
    //!    [`WORKSPACE_MODEL_JSON_SCHEMA`] and matches the checked-in
    //!    golden once the tempdir prefix is normalised to `<TEMPDIR>`.
    //! 2. Two consecutive invocations produce byte-identical pretty-printed
    //!    JSON envelopes — the §"Stability" guarantee from `WorkspaceModel` stability.
    //!
    //! Regenerate the golden with
    //! `REGENERATE_GOLDENS=1 cargo nextest run -p specify-standards --test lint_index`
    //! after a deliberate model change; see [`docs/standards/testing.md`].

    use std::fs;
    use std::path::{Path, PathBuf};

    use serde_json::Value;
    use specify_schema::{ValidationStatus, WORKSPACE_MODEL_JSON_SCHEMA, validate_value};
    use specify_standards::lint::ScanProfile;
    use specify_standards::lint::index::build;
    use tempfile::TempDir;

    const FIXTURE_NAME: &str = "minimal";

    fn crate_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn fixture_src() -> PathBuf {
        crate_root().join("tests/fixtures/lint").join(FIXTURE_NAME)
    }

    fn golden_path() -> PathBuf {
        crate_root()
            .join("tests/fixtures/lint")
            .join(format!("{FIXTURE_NAME}_workspace_model.json"))
    }

    fn stage_fixture() -> TempDir {
        let tempdir = tempfile::tempdir().expect("tempdir");
        crate::common::copy_dir(&fixture_src(), tempdir.path());

        // `.gitignore` + ignored.md cannot be committed cleanly inside
        // the fixture (the .gitignore would cause git to skip the
        // sibling), so they are minted at test time.
        fs::write(tempdir.path().join(".gitignore"), "ignored.md\n").expect("write .gitignore");
        fs::write(tempdir.path().join("ignored.md"), "# Should be ignored\n")
            .expect("write ignored.md");

        // `target/**` must be filtered out by the always-ignore globs.
        fs::create_dir_all(tempdir.path().join("target")).expect("create target dir");
        fs::write(tempdir.path().join("target/build.rs"), "// excluded\n")
            .expect("write target file");

        // Relative symlink that exercises the symlink-fact recorder.
        #[cfg(unix)]
        std::os::unix::fs::symlink("doc.md", tempdir.path().join("link.md"))
            .expect("create unix symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_file("doc.md", tempdir.path().join("link.md"))
            .expect("create windows symlink");

        tempdir
    }

    fn normalise(value: Value, tempdir: &Path) -> Value {
        match value {
            Value::String(s) => {
                let prefix = tempdir.to_string_lossy().into_owned();
                if s == prefix { Value::String("<TEMPDIR>".into()) } else { Value::String(s) }
            }
            Value::Array(items) => {
                Value::Array(items.into_iter().map(|v| normalise(v, tempdir)).collect())
            }
            Value::Object(map) => {
                Value::Object(map.into_iter().map(|(k, v)| (k, normalise(v, tempdir))).collect())
            }
            other => other,
        }
    }

    fn assert_schema_valid(value: &Value) {
        let summaries = validate_value(
            value,
            WORKSPACE_MODEL_JSON_SCHEMA,
            "workspace-model",
            "consumer-indexer fixture",
        );
        let failures: Vec<_> =
            summaries.iter().filter(|s| matches!(s.status, ValidationStatus::Fail)).collect();
        assert!(failures.is_empty(), "WorkspaceModel must validate; got {failures:#?}");
    }

    #[test]
    fn minimal_fixture_matches_golden() {
        let tempdir = stage_fixture();
        let model = build(tempdir.path(), ScanProfile::Project, &[], &[]).expect("build ok");
        let value = serde_json::to_value(&model).expect("serialise");
        assert_schema_valid(&value);

        let normalised = normalise(value, tempdir.path());
        let pretty = serde_json::to_string_pretty(&normalised).expect("pretty");

        let path = golden_path();
        if std::env::var_os("REGENERATE_GOLDENS").is_some() {
            fs::write(&path, format!("{pretty}\n")).expect("write golden");
            return;
        }

        let expected = fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!(
                "missing golden {}: {err}; regenerate with \
             REGENERATE_GOLDENS=1 cargo nextest run -p specify-standards --test lint_index",
                path.display()
            )
        });
        let expected_value: Value = serde_json::from_str(&expected).expect("parse golden");
        assert_eq!(
            normalised, expected_value,
            "WorkspaceModel diverged from golden. Actual:\n{pretty}"
        );
    }

    #[test]
    fn byte_stable_across_runs() {
        let tempdir = stage_fixture();
        let first = build(tempdir.path(), ScanProfile::Project, &[], &[]).expect("first build");
        let second = build(tempdir.path(), ScanProfile::Project, &[], &[]).expect("second build");
        let first_json = serde_json::to_string_pretty(&first).expect("first serialise");
        let second_json = serde_json::to_string_pretty(&second).expect("second serialise");
        assert_eq!(first_json, second_json, "two indexer runs must produce byte-identical JSON");
    }

    #[test]
    fn framework_scan_profile_now_accepted() {
        // `scan_profile: framework` is active; the consumer
        // fixture has no framework-shaped files so the framework walk
        // yields an essentially empty model, but it must no longer
        // surface `IndexError::UnsupportedScanProfile`.
        let tempdir = stage_fixture();
        let model = build(tempdir.path(), ScanProfile::Framework, &[], &[]).expect("framework ok");
        assert_eq!(model.scan_profile, ScanProfile::Framework);
    }
}

mod model_round_trip {
    //! Round-trip + schema validation for `WorkspaceModel`
    //! DTOs.
    //!
    //! Validates two invariants the indexer (S6) relies on:
    //!
    //! 1. The empty envelope serialises to a wire shape that satisfies
    //!    `specify_schema::WORKSPACE_MODEL_JSON_SCHEMA` and round-trips
    //!    back to the same Rust value.
    //! 2. Populating one record per entity family also serialises into
    //!    a schema-valid envelope and round-trips back — catches
    //!    per-entity `rename_all` regressions the empty fixture would
    //!    miss.

    use serde_json::{Map, Value, json};
    use specify_schema::{ValidationStatus, WORKSPACE_MODEL_JSON_SCHEMA, validate_value};
    use specify_standards::lint::{
        AdapterAxis, AdapterDir, AdapterManifest, Brief, BriefScope, File, FileKind, Frontmatter,
        IgnoreDirective, MarkdownLink, MarkdownSection, ScanProfile, Scenario, Skill, Symlink,
        WorkspaceModel, WorkspaceModelVersion,
    };

    fn assert_schema_valid(value: &Value) {
        let summaries = validate_value(
            value,
            WORKSPACE_MODEL_JSON_SCHEMA,
            "workspace-model",
            "WorkspaceModel round-trip fixture",
        );
        let failures: Vec<_> =
            summaries.iter().filter(|s| matches!(s.status, ValidationStatus::Fail)).collect();
        assert!(failures.is_empty(), "WorkspaceModel must validate; got {failures:?}");
    }

    #[test]
    fn empty_model_round_trips() {
        let model = WorkspaceModel {
            version: WorkspaceModelVersion,
            project_dir: ".".into(),
            scan_profile: ScanProfile::Project,
            artifact_paths: vec![],
            languages: vec![],
            files: vec![],
            frontmatter: vec![],
            markdown_sections: vec![],
            markdown_links: vec![],
            symlinks: vec![],
            skills: vec![],
            adapter_manifests: vec![],
            ignore_directives: vec![],
            briefs: vec![],
            fenced_blocks: vec![],
            scenarios: vec![],
            adapter_dirs: vec![],
        };

        let value = serde_json::to_value(&model).expect("serialise empty model");

        assert_eq!(value.get("version"), Some(&Value::from(1)));
        assert_eq!(value.get("project_dir").and_then(Value::as_str), Some("."));
        assert_eq!(value.get("scan_profile").and_then(Value::as_str), Some("project"));
        for required_array in [
            "artifact_paths",
            "languages",
            "files",
            "frontmatter",
            "markdown_sections",
            "markdown_links",
            "symlinks",
            "skills",
            "adapter_manifests",
            "ignore_directives",
        ] {
            assert!(
                value.get(required_array).and_then(Value::as_array).is_some_and(Vec::is_empty),
                "envelope must always serialise `{required_array}` as an empty array"
            );
        }

        assert_schema_valid(&value);

        let parsed: WorkspaceModel =
            serde_json::from_value(value).expect("round-trip empty model from JSON");
        assert_eq!(model, parsed);
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "Single fixture covers every entity family's rename_all wiring; splitting hides the contract."
    )]
    fn populated_model_round_trips() {
        let mut frontmatter_fields: Map<String, Value> = Map::new();
        frontmatter_fields.insert("title".into(), json!("Refine"));
        frontmatter_fields.insert("description".into(), json!("Refine a Specify slice"));

        let mut scenario_fields: Map<String, Value> = Map::new();
        scenario_fields.insert("id".into(), json!("refine-happy-path"));
        scenario_fields.insert("stages".into(), json!(["refine", "build"]));

        let model = WorkspaceModel {
            version: WorkspaceModelVersion,
            project_dir: ".".into(),
            scan_profile: ScanProfile::Project,
            artifact_paths: vec!["src/lib.rs".into()],
            languages: vec!["rust".into()],
            files: vec![File {
                path: "src/lib.rs".into(),
                kind: FileKind::Text,
                language: Some("rust".into()),
                sha256: Some("0".repeat(64)),
            }],
            frontmatter: vec![Frontmatter {
                path: "plugins/spec/skills/refine/SKILL.md".into(),
                schema_id: Some("skill".into()),
                fields: frontmatter_fields,
            }],
            markdown_sections: vec![MarkdownSection {
                path: "README.md".into(),
                level: 2,
                title: "Overview".into(),
                line_start: 5,
                line_end: 12,
                body_line_count: 6,
            }],
            markdown_links: vec![MarkdownLink {
                from_path: "README.md".into(),
                to_raw: "./docs/index.md".into(),
                line: 7,
                resolves: Some(true),
                image: false,
            }],
            symlinks: vec![Symlink {
                path: "adapters/targets/omnia/prose/references/agent-teams.md".into(),
                target: "../../../shared/agent-teams.md".into(),
                broken: false,
                resolved_target: Some("docs/reference/review-team-protocol.md".into()),
            }],
            skills: vec![Skill {
                name: "refine".into(),
                path: "plugins/spec/skills/refine/SKILL.md".into(),
                plugin: "spec".into(),
                frontmatter_ref: "plugins/spec/skills/refine/SKILL.md".into(),
                body_line_count: Some(42),
            }],
            adapter_manifests: vec![AdapterManifest {
                axis: AdapterAxis::Targets,
                name: "omnia".into(),
                path: "adapters/targets/omnia/adapter.yaml".into(),
                version: Some("1".into()),
                brief_keys: vec!["build".into(), "merge".into(), "shape".into()],
            }],
            ignore_directives: vec![IgnoreDirective {
                path: "src/lib.rs".into(),
                line: 12,
                rule_id: "UNI-014".into(),
                rationale: Some("documented rationale that is long enough".into()),
                target_line: 13,
                raw: "// specify-ignore: UNI-014 — documented rationale that is long enough".into(),
            }],
            briefs: vec![Brief {
                path: "adapters/sources/intent/prose/briefs/survey.md".into(),
                axis: AdapterAxis::Sources,
                adapter: "intent".into(),
                operation: "survey".into(),
                scope: BriefScope::Parent,
                sections: vec!["Inputs".into(), "Output contract".into()],
                body_line_count: 24,
            }],
            fenced_blocks: vec![],
            scenarios: vec![Scenario {
                path: "evals/scenarios/refine-happy-path.md".into(),
                id: Some("refine-happy-path".into()),
                stages: vec!["refine".into(), "build".into()],
                expected_artifacts: vec!["spec.md".into()],
                body_id: Some("refine-happy-path".into()),
                fields: scenario_fields,
            }],
            adapter_dirs: vec![AdapterDir {
                path: "adapters/targets/omnia".into(),
                axis: AdapterAxis::Targets,
                name: "omnia".into(),
            }],
        };

        let value = serde_json::to_value(&model).expect("serialise populated model");

        // Per-entity rename-all spot checks — these catch regressions
        // where a `rename_all = "kebab-case"` annotation is dropped or
        // miswired on a single entity struct.
        let section =
            value.pointer("/markdown_sections/0").expect("populated markdown_sections has index 0");
        assert!(section.get("line-start").is_some(), "markdown_sections.line-start missing");
        assert!(
            section.get("line_start").is_none(),
            "snake_case must not leak from markdownSection"
        );

        let link =
            value.pointer("/markdown_links/0").expect("populated markdown_links has index 0");
        assert!(link.get("from-path").is_some(), "markdown_links.from-path missing");
        assert!(link.get("from_path").is_none());

        let skill = value.pointer("/skills/0").expect("populated skills has index 0");
        assert!(skill.get("frontmatter-ref").is_some(), "skill.frontmatter-ref missing");
        assert!(skill.get("frontmatter_ref").is_none());

        let directive =
            value.pointer("/ignore_directives/0").expect("populated ignore_directives has index 0");
        assert!(directive.get("rule-id").is_some(), "ignore_directives.rule-id missing");
        assert!(directive.get("rule_id").is_none());
        assert!(directive.get("target-line").is_some(), "ignore_directives.target-line missing");
        assert!(directive.get("target_line").is_none());

        let fm = value.pointer("/frontmatter/0").expect("populated frontmatter has index 0");
        assert!(fm.get("schema-id").is_some(), "frontmatter.schema-id missing");
        assert!(fm.get("schema_id").is_none());

        let scenario = value.pointer("/scenarios/0").expect("populated scenarios has index 0");
        assert!(
            scenario.get("expected-artifacts").is_some(),
            "scenario.expected-artifacts missing"
        );
        assert!(scenario.get("expected_artifacts").is_none());
        assert!(scenario.get("body-id").is_some(), "scenario.body-id missing");
        assert!(scenario.get("body_id").is_none());

        assert_schema_valid(&value);

        let parsed: WorkspaceModel =
            serde_json::from_value(value).expect("round-trip populated model from JSON");
        assert_eq!(model, parsed);
    }
}
