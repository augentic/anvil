use std::fs;

use tempfile::TempDir;

use super::pre_adapter::synopsis_is_thin;
use super::spec_location::collect_spec_file_location_findings;
use super::{collect_spec_files, path_hint};

#[test]
fn synopsis_thin_below_word_floor() {
    // Three contentful words clear the char floor but trip the word
    // floor (which sits at four).
    assert!(synopsis_is_thin("validates everything carefully"));
}

#[test]
fn synopsis_thin_below_char_floor() {
    // Four short words clear the word floor but trip the char floor.
    assert!(synopsis_is_thin("a b c de"));
}

#[test]
fn synopsis_contentful_clears_both_floors() {
    assert!(!synopsis_is_thin("register a new user account and enforce a unique email"));
}

#[test]
fn synopsis_blank_is_thin() {
    assert!(synopsis_is_thin("   "));
}

#[test]
fn path_hint_relativises_under_slice() {
    let slice = TempDir::new().unwrap();
    let spec = slice.path().join("specs").join("auth").join("spec.md");
    assert_eq!(path_hint(&spec, slice.path()), "specs/auth/spec.md");
}

#[test]
fn path_hint_keeps_filename_outside() {
    let slice = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    let stray = other.path().join("spec.md");
    // A path outside the slice dir cannot be stripped; the hint still
    // names the file rather than dropping it.
    assert!(path_hint(&stray, slice.path()).ends_with("spec.md"));
}

#[test]
fn spec_files_walked_recursively_and_sorted() {
    let root = TempDir::new().unwrap();
    let domain_b = root.path().join("b");
    let domain_a = root.path().join("a");
    fs::create_dir_all(&domain_b).unwrap();
    fs::create_dir_all(&domain_a).unwrap();
    fs::write(domain_b.join("spec.md"), "b").unwrap();
    fs::write(domain_a.join("spec.md"), "a").unwrap();
    fs::write(root.path().join("notes.txt"), "ignored").unwrap();

    let found = collect_spec_files(root.path()).unwrap();
    assert_eq!(found.len(), 2);
    assert_eq!(found[0], domain_a.join("spec.md"));
    assert_eq!(found[1], domain_b.join("spec.md"));
}

#[test]
fn file_location_flags_root_no_canonical() {
    let slice = TempDir::new().unwrap();
    fs::write(slice.path().join("spec.md"), "# misplaced").unwrap();
    let findings = collect_spec_file_location_findings(slice.path());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id.as_deref(), Some("specs.file-location"));
}

#[test]
fn file_location_silent_with_canonical() {
    let slice = TempDir::new().unwrap();
    let domain = slice.path().join("specs").join("auth");
    fs::create_dir_all(&domain).unwrap();
    fs::write(domain.join("spec.md"), "# canonical").unwrap();
    // Even a stray root spec.md does not fire once canonical specs exist.
    fs::write(slice.path().join("spec.md"), "# stray").unwrap();
    assert!(collect_spec_file_location_findings(slice.path()).is_empty());
}

#[test]
fn file_location_silent_no_specs() {
    let slice = TempDir::new().unwrap();
    assert!(collect_spec_file_location_findings(slice.path()).is_empty());
}
