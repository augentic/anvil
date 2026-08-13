//! Change-home vs durable-product-state boundary on [`Layout`].

use project::config::Layout;

#[test]
fn change_home_boundary() {
    let root = std::path::Path::new("/tmp/emery-layout");
    let layout = Layout::new(root);
    let emery = root.join(".emery");
    let change = emery.join("change");

    assert_eq!(layout.emery_dir(), emery);
    assert_eq!(layout.change_root(), change);

    assert_eq!(layout.config_path(), emery.join("project.yaml"));
    assert_eq!(layout.specs_dir(), emery.join("specs"));
    assert_eq!(layout.decisions_dir(), emery.join("decisions"));
    assert_eq!(layout.guest_lock_path(), change.join("guest.lock"));

    assert_eq!(layout.plan_path(), change.join("plan.yaml"));
    assert_eq!(layout.change_brief_path(), change.join("change.md"));
    assert_eq!(layout.discovery_path(), change.join("discovery.md"));
    assert_eq!(layout.slices_dir(), change.join("slices"));
    assert_eq!(layout.slice_dir("demo"), change.join("slices/demo"));
    assert_eq!(layout.events_dir(), change.join("events"));
    assert_eq!(layout.targets_dir(), change.join("targets"));
    assert_eq!(layout.archive_dir(), change.join("archive"));

    assert!(layout.change_root().starts_with(&emery));
    assert!(!layout.config_path().starts_with(&change));
    assert!(!layout.specs_dir().starts_with(&change));
    assert!(!layout.decisions_dir().starts_with(&change));
    assert!(layout.guest_lock_path().starts_with(&change));
}

#[test]
fn project_dir_from_slice() {
    let root = std::path::Path::new("/tmp/emery-layout");
    let slice = Layout::new(root).slice_dir("demo");
    assert_eq!(Layout::project_dir_from_slice(&slice).as_deref(), Some(root));
    assert_eq!(Layout::project_dir_from_slice(root), None);
    assert_eq!(Layout::project_dir_from_slice(&root.join(".emery/slices/demo")), None);
}
