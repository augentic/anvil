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
    assert_eq!(layout.leads_path(), change.join("leads.md"));
    assert_eq!(layout.leads_dir(), change.join("leads"));
    assert_eq!(layout.decomposition_path(), change.join("decomposition.yaml"));
    assert_eq!(layout.decompositions_dir(), change.join("decompositions"));
    assert_eq!(layout.proposals_dir(), change.join("planning/proposals"));
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

#[test]
fn detached_change_home() {
    let root = std::path::Path::new("/tmp/emery-detached");
    let layout = Layout::detached(root);
    assert!(layout.is_detached());
    assert_eq!(layout.project_dir(), root);
    assert_eq!(layout.change_root(), root);
    assert_eq!(layout.plan_path(), root.join("plan.yaml"));
    assert_eq!(layout.slices_dir(), root.join("slices"));
    assert_eq!(layout.specs_dir(), root.join(".emery/specs"));
}
