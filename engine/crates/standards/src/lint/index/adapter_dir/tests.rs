use std::fs;
use std::path::Path;

use super::*;

fn mkdir(root: &Path, rel: &str) {
    fs::create_dir_all(root.join(rel)).expect("create adapter dir");
}

#[test]
fn lists_immediate_children_of_both_axes() {
    let tmp = tempfile::tempdir().expect("tmp");
    mkdir(tmp.path(), "adapters/sources/intent");
    mkdir(tmp.path(), "adapters/targets/omnia");
    mkdir(tmp.path(), "adapters/targets/orphan");

    let dirs = extract(tmp.path());
    let paths: Vec<&str> = dirs.iter().map(|d| d.path.as_str()).collect();
    assert_eq!(
        paths,
        vec!["adapters/sources/intent", "adapters/targets/omnia", "adapters/targets/orphan"],
    );
    assert_eq!(dirs[0].axis, AdapterAxis::Sources);
    assert_eq!(dirs[1].axis, AdapterAxis::Targets);
    assert_eq!(dirs[1].name, "omnia");
}

#[test]
fn skips_nested_and_non_axis_directories() {
    let tmp = tempfile::tempdir().expect("tmp");
    mkdir(tmp.path(), "adapters/sources/intent/briefs");
    mkdir(tmp.path(), "adapters/shared/rules");

    let dirs = extract(tmp.path());
    let paths: Vec<&str> = dirs.iter().map(|d| d.path.as_str()).collect();
    assert_eq!(paths, vec!["adapters/sources/intent"], "only immediate axis children are listed");
}

#[test]
fn absent_axis_yields_no_facts() {
    let tmp = tempfile::tempdir().expect("tmp");
    assert!(extract(tmp.path()).is_empty());
}
