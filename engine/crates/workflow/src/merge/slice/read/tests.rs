use tempfile::TempDir;

use super::*;

/// A valid minimal spec with no delta headers — `merge` keeps it
/// verbatim as a freshly created baseline.
const NEW_SPEC: &str = "### Requirement: User can log in\n\nID: REQ-001\n\n#### Scenario: ok\n\n- GIVEN a user\n- WHEN they log in\n- THEN it works\n";

fn write_file(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().expect("path has a parent")).expect("mkdir");
    fs::write(path, body).expect("write fixture");
}

fn three_way_class(staged: &Path, baseline: &Path) -> ArtifactClass {
    ArtifactClass {
        name: "specs".to_string(),
        staged_dir: staged.to_path_buf(),
        baseline_dir: baseline.to_path_buf(),
        strategy: MergeStrategy::ThreeWayMerge,
    }
}

#[test]
fn lists_spec_deltas() {
    let dir = TempDir::new().expect("tempdir");
    let staged = dir.path().join("staged");
    let baseline = dir.path().join("baseline");
    write_file(&staged.join("login/spec.md"), NEW_SPEC);
    write_file(&staged.join("logout/spec.md"), NEW_SPEC);
    write_file(&staged.join("notes.txt"), "loose file, not a spec dir");
    fs::create_dir_all(staged.join("no-spec")).expect("mkdir empty dir");

    let class = three_way_class(&staged, &baseline);
    let specs = list_delta_specs(&class).expect("list");

    let names: Vec<&str> = specs.iter().map(|s| s.spec_name.as_str()).collect();
    assert_eq!(names, vec!["login", "logout"]);
    assert_eq!(specs[0].baseline_path, baseline.join("login").join("spec.md"));
}

#[test]
fn plan_creates_new_baseline() {
    let dir = TempDir::new().expect("tempdir");
    let staged = dir.path().join("staged");
    let baseline = dir.path().join("baseline");
    write_file(&staged.join("login/spec.md"), NEW_SPEC);

    let class = three_way_class(&staged, &baseline);
    let plan = plan_three_way(dir.path(), std::slice::from_ref(&class)).expect("plan");

    assert_eq!(plan.len(), 1);
    let entry = &plan[0];
    assert_eq!(entry.name, "login");
    assert_eq!(entry.class_name, "specs");
    assert_eq!(entry.baseline_path, baseline.join("login").join("spec.md"));
    assert_eq!(entry.result.output, NEW_SPEC);
}

#[test]
fn preview_opaque_marks_actions() {
    let dir = TempDir::new().expect("tempdir");
    let staged = dir.path().join("staged");
    let baseline = dir.path().join("baseline");
    write_file(&staged.join("schemas/user.yaml"), "fresh: true\n");
    write_file(&staged.join("schemas/order.yaml"), "updated: true\n");
    write_file(&baseline.join("schemas/order.yaml"), "old\n");

    let class = ArtifactClass {
        name: "contracts".to_string(),
        staged_dir: staged,
        baseline_dir: baseline,
        strategy: MergeStrategy::OpaqueReplace,
    };
    let entries = preview_opaque(std::slice::from_ref(&class)).expect("preview");

    let observed: Vec<(&str, OpaqueAction)> =
        entries.iter().map(|e| (e.relative_path.as_str(), e.action)).collect();
    assert_eq!(
        observed,
        vec![
            ("schemas/order.yaml", OpaqueAction::Replaced),
            ("schemas/user.yaml", OpaqueAction::Added),
        ]
    );
}

/// read → parse/merge → write round-trip over the `case-04-modified`
/// merge fixture: a delta with a `## MODIFIED Requirements` block laid
/// out as a slice tree, planned, committed to a baseline, and read back
/// byte-for-byte against the captured expected merge.
#[test]
fn round_trip_modified_baseline() {
    const BASELINE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/merge/case-04-modified/baseline.md"
    ));
    const DELTA: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/merge/case-04-modified/delta.md"
    ));
    const EXPECTED: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/merge/case-04-modified/expected-merged.md"
    ));

    let dir = TempDir::new().expect("tempdir");
    let staged = dir.path().join("staged");
    let baseline = dir.path().join("baseline");
    write_file(&staged.join("login/spec.md"), DELTA);
    write_file(&baseline.join("login/spec.md"), BASELINE);

    let class = three_way_class(&staged, &baseline);
    let plan = plan_three_way(dir.path(), std::slice::from_ref(&class)).expect("plan");
    crate::merge::slice::write::write_three_way_baselines(&plan).expect("write");

    let merged = fs::read_to_string(baseline.join("login").join("spec.md")).expect("read");
    assert_eq!(merged, EXPECTED);
}
