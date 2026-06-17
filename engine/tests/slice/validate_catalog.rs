//! Component catalog contract — `slice validate` catalog drift gate.

use crate::support::*;

/// Evidence with a `component:` directive on a claim.
const EVIDENCE_WITH_COMPONENT: &str = "authority: behaviour
lead: my-slice
claims:
  - kind: region
    id: task-list-footer
    component: tab-bar
    statement: \"Bottom tab bar with three tabs.\"
";

/// Evidence with `notes.candidate_component` (informational hint,
/// not a hard `component:` directive).
const EVIDENCE_WITH_CANDIDATE_COMPONENT: &str = "authority: behaviour
lead: my-slice
claims:
  - kind: region
    id: task-list-header
    notes:
      candidate_component: hero-banner
    statement: \"Hero banner at top of screen.\"
";

/// A minimal catalog YAML with one confirmed and one rejected entry.
const CATALOG_YAML: &str = "version: 1
components:
  tab-bar:
    status: confirmed
    description: \"Bottom navigation across the primary app sections.\"
  hero-banner:
    status: rejected
    description: \"Not a real shared component.\"
";

/// Plan that declares a `ui-screens` source for the `my-slice` entry.
const PLAN_WITH_UI_SCREENS: &str = "\
name: component-catalog
lifecycle: pending
sources:
  ui-screens:
    adapter: screenshots
    path: ./screens
slices:
  - name: my-slice
    status: pending
    sources:
      - { source: ui-screens, lead: my-slice }
";

/// Stage a slice with Evidence containing `component:` directives
/// and optionally a component catalog.
fn stage_slice_with_catalog(evidence: &str, catalog: Option<&str>, plan: Option<&str>) -> Project {
    let project = Project::init().with_schemas();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "my-slice"])
        .assert()
        .success();
    let slice_dir = project.slices_dir().join("my-slice");
    let evidence_dir = slice_dir.join("evidence");
    fs::create_dir_all(&evidence_dir).expect("mkdir evidence");
    fs::write(evidence_dir.join("ui-screens.yaml"), evidence).expect("write evidence");

    if let Some(cat) = catalog {
        let catalog_dir = project.root().join(".specify/design-system");
        fs::create_dir_all(&catalog_dir).expect("mkdir design-system");
        fs::write(catalog_dir.join("components.yaml"), cat).expect("write catalog");
    }

    if let Some(yaml) = plan {
        project.seed_plan(yaml);
    }
    project
}

#[test]
fn skips_catalog_drift_no_catalog() {
    let project =
        stage_slice_with_catalog(EVIDENCE_WITH_COMPONENT, None, Some(PLAN_WITH_UI_SCREENS));
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert();
    assert_no_finding(assert.get_output(), "slice-catalog-drift");
}

#[test]
fn validate_passes_when_slug_confirmed() {
    let project = stage_slice_with_catalog(
        EVIDENCE_WITH_COMPONENT,
        Some(CATALOG_YAML),
        Some(PLAN_WITH_UI_SCREENS),
    );
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert();
    assert_no_finding(assert.get_output(), "slice-catalog-drift");
}

#[test]
fn validate_detects_missing_catalog_entry() {
    let catalog_without_tab_bar = "version: 1\ncomponents:\n  card-row:\n    status: confirmed\n";
    let project = stage_slice_with_catalog(
        EVIDENCE_WITH_COMPONENT,
        Some(catalog_without_tab_bar),
        Some(PLAN_WITH_UI_SCREENS),
    );
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let detail = find_finding_impact(assert.get_output(), "slice-catalog-drift");
    assert!(
        detail.contains("tab-bar") && detail.contains("no entry exists"),
        "drift detail should name the missing slug, got: {detail}"
    );
}

#[test]
fn validate_detects_rejected_catalog_entry() {
    let catalog_with_rejected = "version: 1\ncomponents:\n  tab-bar:\n    status: rejected\n";
    let project = stage_slice_with_catalog(
        EVIDENCE_WITH_COMPONENT,
        Some(catalog_with_rejected),
        Some(PLAN_WITH_UI_SCREENS),
    );
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let detail = find_finding_impact(assert.get_output(), "slice-catalog-drift");
    assert!(
        detail.contains("tab-bar") && detail.contains("rejected"),
        "drift detail should describe the rejected status, got: {detail}"
    );
}

#[test]
fn validate_ignores_candidate_notes() {
    let project = stage_slice_with_catalog(
        EVIDENCE_WITH_CANDIDATE_COMPONENT,
        Some(CATALOG_YAML),
        Some(PLAN_WITH_UI_SCREENS),
    );
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert();
    assert_no_finding(assert.get_output(), "slice-catalog-drift");
}

#[test]
fn validate_passes_with_empty_catalog() {
    let empty_catalog = "version: 1\ncomponents: {}\n";
    let evidence_no_component = "authority: behaviour
lead: my-slice
claims:
  - kind: region
    id: task-list-body
    statement: \"Main task list body.\"
";
    let project = stage_slice_with_catalog(
        evidence_no_component,
        Some(empty_catalog),
        Some(PLAN_WITH_UI_SCREENS),
    );
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert();
    assert_no_finding(assert.get_output(), "slice-catalog-drift");
}
