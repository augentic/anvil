use std::path::Path;

use super::*;

fn workspace_clone_dir(suffix: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    // Platform root carries `.specify/project.yaml`, the slot marker.
    std::fs::create_dir_all(tmp.path().join(".specify")).unwrap();
    std::fs::write(tmp.path().join(".specify").join("project.yaml"), "workspace: true\n").unwrap();
    let slot = tmp.path().join("workspace").join(suffix);
    std::fs::create_dir_all(slot.join(".specify")).unwrap();
    std::fs::write(slot.join(".specify").join("project.yaml"), "name: stub\n").unwrap();
    tmp
}

#[test]
fn workspace_clone_path() {
    let tmp = workspace_clone_dir("traffic");
    let path = tmp.path().join("workspace").join("traffic");
    assert!(is_clone_eligible(&path));
}

#[test]
fn rejects_normal_project_root() {
    let path = Path::new("/home/user/project/");
    assert!(!is_clone_eligible(path));
}

#[test]
fn rejects_bare_specify_dir() {
    let path = Path::new("/home/user/project/.specify/");
    assert!(!is_clone_eligible(path));
}

#[test]
fn deeply_nested_workspace_clone() {
    let tmp = workspace_clone_dir("mobile");
    let path = tmp.path().join("workspace").join("mobile").join("sub").join("dir");
    std::fs::create_dir_all(path.join(".specify")).unwrap();
    std::fs::write(path.join(".specify").join("project.yaml"), "name: stub\n").unwrap();
    assert!(is_clone_eligible(&path));
}
