//! Baseline-aware synthesis projection and merge guard tests.

use std::collections::BTreeMap;
use std::fs;

use specify_error::Error;
use specify_model::evidence::{AuthorityClass, ClaimKind};
use specify_workflow_lib::merge::{ArtifactClass, MergeOperation, MergeStrategy, merge, slice};
use specify_workflow_lib::slice::model::{ModelClaim, ModelRequirement, SliceModel};
use specify_workflow_lib::slice::synthesis::authority::Agreement;
use specify_workflow_lib::slice::synthesis::baseline::BaselineIndex;
use specify_workflow_lib::slice::{
    LifecycleStatus, ProjectionHeader, SliceMetadata, project, render_spec_files,
};
use tempfile::TempDir;

const TASK_BASELINE: &str = r"### Requirement: List tasks

ID: REQ-001
Sources: [intent]
Status: agreed

The user can view their tasks.

#### Scenario: View list

- **WHEN** the user opens the app
- **THEN** tasks are listed

### Requirement: Create task

ID: REQ-002
Sources: [intent]
Status: agreed

The user can add a task.

#### Scenario: Add task

- **WHEN** the user submits a title
- **THEN** a task is created

### Requirement: Complete task

ID: REQ-003
Sources: [intent]
Status: agreed

The user can mark a task done.

#### Scenario: Complete

- **WHEN** the user taps complete
- **THEN** the task is marked done
";

fn baseline_index(tmp: &TempDir) -> BaselineIndex {
    let specs = tmp.path().join(".specify/specs/task");
    fs::create_dir_all(&specs).expect("mkdir specs");
    fs::write(specs.join("spec.md"), TASK_BASELINE).expect("write baseline");
    BaselineIndex::build(&tmp.path().join(".specify/specs")).expect("build index")
}

fn empty_header() -> ProjectionHeader {
    ProjectionHeader {
        version: 1,
        slice: "task-list".into(),
        project: None,
    }
}

fn claim(source: &str, id: &str) -> ModelClaim {
    ModelClaim {
        source: source.into(),
        id: id.into(),
        kind: ClaimKind::Requirement,
        winner: None,
    }
}

fn requirement(title: &str, domain: &str, statement: &str) -> ModelRequirement {
    ModelRequirement {
        id: None,
        title: title.into(),
        status: None,
        agreement: Some(Agreement::Agreed),
        domain: Some(domain.into()),
        baseline_id: None,
        sources: Vec::new(),
        claims: vec![claim("intent", "add-task-list")],
        statement: statement.into(),
        scenarios: Vec::new(),
        notes: None,
    }
}

fn project_model(model: SliceModel, baseline_index: &BaselineIndex) -> SliceModel {
    let authority = BTreeMap::from([("intent".into(), AuthorityClass::Intent)]);
    let evidence =
        BTreeMap::from([(("intent".into(), "add-task-list".into()), ClaimKind::Requirement)]);
    project(model, empty_header(), &authority, &BTreeMap::new(), &evidence, baseline_index)
        .expect("project")
}

#[test]
fn flat_delta_rejected() {
    let baseline = TASK_BASELINE;
    let delta =
        "### Requirement: Filter tasks\n\nID: REQ-004\n\n#### Scenario: Filter\n\n- filter\n\n";
    let merged = merge(Some(baseline), delta).expect("merge succeeds at engine layer");
    assert!(merged.operations.is_empty(), "engine alone would no-op");

    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    let slice_dir = root.join(".specify/slices/task-list");
    fs::create_dir_all(slice_dir.join("specs/task")).expect("mkdir slice spec");
    fs::create_dir_all(root.join(".specify/specs/task")).expect("mkdir baseline");
    fs::write(root.join(".specify/specs/task/spec.md"), baseline).expect("baseline");
    fs::write(slice_dir.join("specs/task/spec.md"), delta).expect("delta");
    fs::write(slice_dir.join("proposal.md"), "# p\n").expect("proposal");

    let metadata = SliceMetadata {
        target: "omnia".into(),
        status: LifecycleStatus::Built,
        created_at: None,
        defined_at: None,
        completed_at: None,
        merged_at: None,
        dropped_at: None,
        drop_reason: None,
        touched_specs: Vec::new(),
        outcome: None,
    };
    metadata.save(&slice_dir).expect("metadata");

    let classes = vec![ArtifactClass {
        name: "specs".into(),
        staged_dir: slice_dir.join("specs"),
        baseline_dir: root.join(".specify/specs"),
        strategy: MergeStrategy::ThreeWayMerge,
    }];

    let err = slice::preview(&slice_dir, &classes).expect_err("preview must reject flat delta");
    match err {
        Error::Diag { code, .. } => assert_eq!(code, "merge-delta-headers-required"),
        other => panic!("expected merge-delta-headers-required, got {other:?}"),
    }
}

#[test]
fn modified_additive_ids() {
    let tmp = TempDir::new().expect("tempdir");
    let baseline_index = baseline_index(&tmp);
    let model = SliceModel {
        version: None,
        slice: None,
        project: None,
        requirements: vec![
            requirement("Filter tasks", "task", "The user can filter tasks."),
            requirement("Sort tasks", "task", "The user can sort tasks."),
        ],
        tasks: Vec::new(),
    };
    let projected = project_model(model, &baseline_index);
    let ids: Vec<String> = projected.requirements.iter().map(|r| r.id.clone().unwrap()).collect();
    assert_eq!(ids, vec!["REQ-004", "REQ-005"]);

    let rendered = render_spec_files(&projected, &baseline_index);
    let spec = rendered.into_iter().find(|s| s.domain == "task").expect("task spec");
    assert!(spec.content.contains("## ADDED Requirements"));
    assert!(spec.content.contains("ID: REQ-004"));
    assert!(spec.content.contains("ID: REQ-005"));
    assert!(!spec.content.contains("## MODIFIED Requirements"));
}

#[test]
fn baseline_id_modified_section() {
    let tmp = TempDir::new().expect("tempdir");
    let baseline_index = baseline_index(&tmp);
    let mut model = SliceModel {
        version: None,
        slice: None,
        project: None,
        requirements: vec![requirement(
            "List tasks",
            "task",
            "The user can view and search their tasks.",
        )],
        tasks: Vec::new(),
    };
    model.requirements[0].baseline_id = Some("REQ-002".into());

    let projected = project_model(model, &baseline_index);
    assert_eq!(projected.requirements[0].id.as_deref(), Some("REQ-002"));

    let rendered = render_spec_files(&projected, &baseline_index);
    let spec = rendered.into_iter().find(|s| s.domain == "task").expect("task spec");
    assert!(spec.content.contains("## MODIFIED Requirements"));
    assert!(spec.content.contains("ID: REQ-002"));
    assert!(!spec.content.contains("## ADDED Requirements"));
}

#[test]
fn additive_merge_into_baseline() {
    let tmp = TempDir::new().expect("tempdir");
    let baseline_index = baseline_index(&tmp);
    let model = SliceModel {
        version: None,
        slice: None,
        project: None,
        requirements: vec![requirement("Filter tasks", "task", "The user can filter tasks.")],
        tasks: Vec::new(),
    };
    let projected = project_model(model, &baseline_index);
    let rendered = render_spec_files(&projected, &baseline_index);
    let delta = rendered.into_iter().find(|s| s.domain == "task").expect("task spec").content;

    let merged = merge(Some(TASK_BASELINE), &delta).expect("merge applies ADDED section");
    assert_eq!(merged.operations.len(), 1);
    assert!(matches!(
        &merged.operations[0],
        MergeOperation::Added { id, name } if id == "REQ-004" && name == "Filter tasks"
    ));
    assert!(merged.output.contains("ID: REQ-004"));
    assert!(merged.output.contains("ID: REQ-001"));
}

#[test]
fn mixed_domain_ids_no_collision() {
    let tmp = TempDir::new().expect("tempdir");
    let baseline_index = baseline_index(&tmp);
    let model = SliceModel {
        version: None,
        slice: None,
        project: None,
        requirements: vec![
            requirement("Greenfield one", "auth", "First new domain requirement."),
            requirement("Filter tasks", "task", "Additive in modified domain."),
            requirement("Greenfield two", "auth", "Second new domain requirement."),
        ],
        tasks: Vec::new(),
    };
    let projected = project_model(model, &baseline_index);
    let ids: Vec<String> = projected.requirements.iter().map(|r| r.id.clone().unwrap()).collect();
    // Baseline task domain occupies REQ-001..REQ-003; slice-global allocator skips them.
    assert_eq!(ids, vec!["REQ-004", "REQ-005", "REQ-006"]);
}

#[test]
fn baseline_id_rejected_on_new_domain() {
    let model = SliceModel {
        version: None,
        slice: None,
        project: None,
        requirements: vec![ModelRequirement {
            baseline_id: Some("REQ-001".into()),
            ..requirement("Only requirement", "auth", "No baseline for this domain.")
        }],
        tasks: Vec::new(),
    };
    let err = project(
        model,
        empty_header(),
        &BTreeMap::from([("intent".into(), AuthorityClass::Intent)]),
        &BTreeMap::new(),
        &BTreeMap::from([(("intent".into(), "add-task-list".into()), ClaimKind::Requirement)]),
        &BaselineIndex::default(),
    )
    .expect_err("baseline-id requires a modified domain");
    assert!(err.to_string().contains("slice-model-baseline-id-orphan"), "unexpected error: {err}");
}
