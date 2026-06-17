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
            path: "adapters/targets/omnia/references/agent-teams.md".into(),
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
            path: "adapters/sources/intent/briefs/survey.md".into(),
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
    assert!(section.get("line_start").is_none(), "snake_case must not leak from markdownSection");

    let link = value.pointer("/markdown_links/0").expect("populated markdown_links has index 0");
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
    assert!(scenario.get("expected-artifacts").is_some(), "scenario.expected-artifacts missing");
    assert!(scenario.get("expected_artifacts").is_none());
    assert!(scenario.get("body-id").is_some(), "scenario.body-id missing");
    assert!(scenario.get("body_id").is_none());

    assert_schema_valid(&value);

    let parsed: WorkspaceModel =
        serde_json::from_value(value).expect("round-trip populated model from JSON");
    assert_eq!(model, parsed);
}
