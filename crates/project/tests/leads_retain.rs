//! Lead-catalog revision retention: first reference copies exact bytes.

use artifacts::leads::{Lead, Leads};
use project::config::Layout;
use project::plan::retain_leads;
use project::snapshot::SnapshotId;

#[test]
fn retain_on_reference() {
    let root = tempfile::tempdir().expect("tempdir");
    let layout = Layout::new(root.path());
    std::fs::create_dir_all(layout.change_root()).expect("change home");

    let catalog = Leads::from_leads(vec![Lead::new("orders-api", "code", "Orders API")]);
    catalog.write_atomic(&layout.leads_path()).expect("write");
    let digest = retain_leads(layout).expect("retain");
    let dest = layout.leads_revision_path(&digest);
    assert!(dest.is_file(), "{}", dest.display());
    let current = std::fs::read(layout.leads_path()).expect("current");
    let retained = std::fs::read(&dest).expect("retained");
    assert_eq!(retained, current);

    let again = retain_leads(layout).expect("idempotent");
    assert_eq!(again, digest);
    assert_eq!(std::fs::read(&dest).expect("still"), retained);
}

#[test]
fn edit_keeps_prior() {
    let root = tempfile::tempdir().expect("tempdir");
    let layout = Layout::new(root.path());
    std::fs::create_dir_all(layout.change_root()).expect("change home");

    let first = Leads::from_leads(vec![Lead::new("orders-api", "code", "Orders API")]);
    first.write_atomic(&layout.leads_path()).expect("write");
    let first_digest = retain_leads(layout).expect("retain");
    let first_bytes = std::fs::read(layout.leads_revision_path(&first_digest)).expect("first");

    let mut edited = first;
    edited.lead_mut("orders-api").expect("lead").synopsis = "Orders API with pagination".into();
    edited.write_atomic(&layout.leads_path()).expect("rewrite");
    let second_digest = SnapshotId::from_digest(&edited.digest_hex().expect("hex"));
    assert_ne!(first_digest, second_digest);

    let second = retain_leads(layout).expect("retain edited");
    assert_eq!(second, second_digest);
    assert_eq!(
        std::fs::read(layout.leads_revision_path(&first_digest)).expect("prior"),
        first_bytes,
        "referenced revision is immutable"
    );
    assert_ne!(
        std::fs::read(layout.leads_path()).expect("current"),
        first_bytes,
        "current view is a new revision"
    );
}
