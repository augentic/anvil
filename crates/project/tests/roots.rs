//! In-place vs detached change-home detection.

use std::path::Path;

use project::config::Roots;

fn product_tree() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let emery = tmp.path().join(".emery");
    std::fs::create_dir(&emery).expect("mkdir .emery");
    std::fs::write(emery.join("project.yaml"), "name: fixture\n").expect("project.yaml");
    tmp
}

#[test]
fn cwd_detached() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let roots = Roots::resolve(tmp.path(), None);
    assert!(roots.is_detached());
    assert_eq!(roots.mount_root(), tmp.path());
    assert_eq!(roots.change_root(), tmp.path());
    assert_eq!(roots.product_root(), None);
    assert!(tmp.path().read_dir().expect("read").next().is_none(), "no subdirectory created");
}

#[test]
fn change_dir_override() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path().join("home");
    std::fs::create_dir(&home).expect("mkdir home");
    let roots = Roots::resolve(tmp.path(), Some(Path::new("home")));
    assert!(roots.is_detached());
    assert_eq!(roots.change_root(), home);
}

#[test]
fn in_place_ancestor_wins() {
    let product = product_tree();
    let nested = product.path().join("src/nested");
    std::fs::create_dir_all(&nested).expect("mkdir nested");
    let roots = Roots::resolve(&nested, None);
    assert!(!roots.is_detached());
    assert_eq!(roots.mount_root(), product.path());
    assert_eq!(roots.change_root(), product.path().join(".emery/change"));
    assert_eq!(roots.product_root(), Some(product.path()));
}

#[test]
fn change_dir_in_product() {
    let product = product_tree();
    let home = product.path().join("elsewhere");
    std::fs::create_dir(&home).expect("mkdir elsewhere");
    let roots = Roots::resolve(product.path(), Some(&home));
    assert!(roots.is_detached());
    assert_eq!(roots.change_root(), home);
}

#[test]
fn change_dir_at_product() {
    let product = product_tree();
    let roots = Roots::resolve(product.path(), Some(product.path()));
    assert!(roots.is_detached());
    assert_eq!(roots.mount_root(), product.path());
}

#[test]
fn definition_joins_mount() {
    let product = product_tree();
    let roots = Roots::resolve(product.path(), None);
    assert_eq!(
        roots.definition_root(Path::new(".emery/system/")),
        product.path().join(".emery/system")
    );
}
