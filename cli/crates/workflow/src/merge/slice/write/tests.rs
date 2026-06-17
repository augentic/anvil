use std::ffi::OsString;
use std::path::PathBuf;

use super::*;
use crate::merge::engine::MergeResult;

fn entry(baseline_path: PathBuf, output: &str) -> MergePreviewEntry {
    MergePreviewEntry {
        class_name: "specs".to_string(),
        name: "login".to_string(),
        baseline_path,
        result: MergeResult {
            output: output.to_string(),
            operations: Vec::new(),
        },
    }
}

#[test]
fn writes_baseline_atomically() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("specs").join("login").join("spec.md");
    write_three_way_baselines(&[entry(target.clone(), "merged body\n")]).expect("write");

    assert_eq!(fs::read_to_string(&target).expect("read"), "merged body\n");
    assert!(target.parent().expect("parent").is_dir(), "parent chain created");
}

#[test]
fn write_replaces_whole_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("spec.md");
    fs::write(&target, "OLD CONTENT THAT IS MUCH LONGER\n").expect("seed");

    write_three_way_baselines(&[entry(target.clone(), "new\n")]).expect("write");

    assert_eq!(fs::read_to_string(&target).expect("read"), "new\n");
    let siblings: Vec<OsString> = fs::read_dir(dir.path())
        .expect("read_dir")
        .map(|e| e.expect("entry").file_name())
        .collect();
    assert_eq!(siblings, vec![OsString::from("spec.md")], "atomic rename leaves no temp file");
}

#[test]
fn commit_opaque_copies_tree() {
    let dir = tempfile::tempdir().expect("tempdir");
    let staged = dir.path().join("staged");
    let baseline = dir.path().join("baseline");
    fs::create_dir_all(staged.join("schemas")).expect("mkdir");
    fs::write(staged.join("schemas").join("user.yaml"), "a\n").expect("seed nested");
    fs::write(staged.join("top.txt"), "b\n").expect("seed top");

    let class = ArtifactClass {
        name: "contracts".to_string(),
        staged_dir: staged,
        baseline_dir: baseline.clone(),
        strategy: MergeStrategy::OpaqueReplace,
    };
    let counts = commit_opaque(std::slice::from_ref(&class)).expect("commit");

    assert_eq!(counts.get("contracts"), Some(&2));
    assert_eq!(
        fs::read_to_string(baseline.join("schemas").join("user.yaml")).expect("read"),
        "a\n"
    );
    assert_eq!(fs::read_to_string(baseline.join("top.txt")).expect("read"), "b\n");
}

#[test]
fn summary_counts_classes() {
    let three_way = vec![entry(PathBuf::from("/x"), "")];
    let mut opaque: BTreeMap<String, usize> = BTreeMap::new();
    opaque.insert("contracts".to_string(), 3);

    assert_eq!(
        build_merge_summary(&three_way, &opaque),
        "Merged 3 contracts, 1 specs into baseline"
    );
}

#[test]
fn summary_empty_merge() {
    let empty: BTreeMap<String, usize> = BTreeMap::new();
    assert_eq!(build_merge_summary(&[], &empty), "Merged 0 entries into baseline");
}
