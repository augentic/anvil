//! Output-home integration: the generation-pointer commit contract
//! (ADR-0001 Option C, ADR-0009 §2) at the crate's public surface.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use engine::home::{Diff, Home, SpecSet};

fn set(spec: &str) -> SpecSet {
    SpecSet {
        bindings: "sources: []\n".to_string(),
        receipts: "receipts: []\n".to_string(),
        spec: spec.to_string(),
        design: "# Design\n".to_string(),
    }
}

/// Every file under `dir` by relative path and bytes.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, tree: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("read dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(root, &path, tree);
            } else {
                let relative = path.strip_prefix(root).expect("under root").to_path_buf();
                tree.insert(relative, fs::read(&path).expect("read file"));
            }
        }
    }
    let mut tree = BTreeMap::new();
    walk(dir, dir, &mut tree);
    tree
}

#[test]
fn commit_behind_pointer() {
    let project = tempfile::tempdir().expect("tempdir");
    let home = Home::new(project.path());

    let committed = home.commit(&set("# Spec\n")).expect("commit");

    assert_eq!(fs::read_to_string(committed.dir.join("spec.md")).expect("spec"), "# Spec\n");
    assert_eq!(fs::read_to_string(committed.dir.join("design.md")).expect("design"), "# Design\n");
    let pointer = fs::read_to_string(project.path().join(".emery/spec/current")).expect("pointer");
    assert_eq!(pointer.trim(), committed.id, "the pointer names the committed generation");
    let current = home.current().expect("current").expect("committed");
    assert_eq!(current, committed);
}

#[test]
fn rerun_is_byte_stable() {
    let project = tempfile::tempdir().expect("tempdir");
    let home = Home::new(project.path());

    home.commit(&set("# Spec\n")).expect("first commit");
    let before = snapshot(project.path());
    home.commit(&set("# Spec\n")).expect("re-run commit");

    assert_eq!(before, snapshot(project.path()), "an identical re-run must be byte-stable");
}

#[test]
fn swap_prunes_superseded() {
    let project = tempfile::tempdir().expect("tempdir");
    let home = Home::new(project.path());

    let first = home.commit(&set("# Spec v1\n")).expect("first commit");
    let second = home.commit(&set("# Spec v2\n")).expect("second commit");

    assert_ne!(first.id, second.id);
    assert!(!first.dir.exists(), "the superseded generation is pruned");
    assert_eq!(home.current().expect("current").expect("committed").id, second.id);
}

#[test]
fn crash_litter_pruned() {
    let project = tempfile::tempdir().expect("tempdir");
    let home = Home::new(project.path());
    let committed = home.commit(&set("# Spec\n")).expect("commit");

    // A crash between generation write and pointer swap leaves a
    // partial generation directory and a stray temp file; the pointer
    // still names the previous set.
    let partial = project.path().join(".emery/spec/generations/deadbeef");
    fs::create_dir_all(&partial).expect("partial dir");
    fs::write(partial.join("spec.md"), "half-written").expect("partial file");
    fs::write(project.path().join(".emery/spec/.tmpXYZ"), "temp litter").expect("temp litter");

    let current = home.current().expect("current").expect("committed");
    assert_eq!(current.id, committed.id, "readers trust only what the pointer names");

    home.commit(&set("# Spec\n")).expect("re-run commit");
    assert!(!partial.exists(), "crash litter is pruned on the next commit");
    assert!(!project.path().join(".emery/spec/.tmpXYZ").exists());
}

/// One parseable requirement block for the diff kernel's spec fixtures.
fn block(id: u32, name: &str, body: &str) -> String {
    format!(
        "### Requirement: {name}\n\nID: REQ-{id:03}\nSources: [mock-docs]\nStatus: agreed\n\n{body}\n\n"
    )
}

#[test]
fn outgoing_reads_current() {
    let project = tempfile::tempdir().expect("tempdir");
    let home = Home::new(project.path());
    assert!(home.outgoing().is_none(), "no generation, no outgoing set");

    let committed = home.commit(&set("# Spec\n")).expect("commit");
    let (from, outgoing) = home.outgoing().expect("outgoing after commit");
    assert_eq!(from, committed.id, "the outgoing id is the pointer's");
    assert_eq!(outgoing, set("# Spec\n"), "the outgoing set reads back verbatim");
}

/// ADR-0010: the re-mine diff names changed artifacts and `spec.md`
/// sections by heading subject — immune to positional `REQ-NNN`
/// shifts when blocks are inserted or removed.
#[test]
fn remine_diff_sections() {
    let old_spec = format!(
        "# Specification\n\n{}{}{}",
        block(1, "login.flow", "Users sign in with email and password."),
        block(2, "legacy.export", "Exports ship nightly."),
        block(3, "session.timeout", "Sessions expire after 30 minutes of inactivity."),
    );
    let new_spec = format!(
        "# Specification\n\n{}{}{}",
        block(1, "access.audit", "Access is audited."),
        block(2, "login.flow", "Users sign in with email and password."),
        block(3, "session.timeout", "Sessions expire after 45 minutes of inactivity."),
    );
    let outgoing = SpecSet {
        receipts: "receipts: [old]\n".to_string(),
        ..set(&old_spec)
    };
    let incoming = SpecSet {
        receipts: "receipts: [new]\n".to_string(),
        ..set(&new_spec)
    };

    let diff = Diff::between("cafe".to_string(), &outgoing, &incoming);

    assert!(!diff.is_empty());
    assert_eq!(diff.from, "cafe");
    assert_eq!(diff.artifacts, ["receipts.yaml", "spec.md"], "changed artifacts in set order");
    assert_eq!(diff.added, ["access.audit"]);
    assert_eq!(diff.removed, ["legacy.export"]);
    assert_eq!(
        diff.changed,
        ["session.timeout"],
        "`login.flow` shifted from REQ-001 to REQ-002 unchanged — positional ids never count"
    );
}

#[test]
fn remine_diff_empty() {
    let identical =
        set(&format!("# Specification\n\n{}", block(1, "login.flow", "Users sign in.")));
    let diff = Diff::between("cafe".to_string(), &identical, &identical);
    assert!(diff.is_empty(), "a byte-stable re-run is an explicit empty diff: {diff:?}");
}

#[test]
fn dangling_pointer_fails() {
    let project = tempfile::tempdir().expect("tempdir");
    let home = Home::new(project.path());
    let spec_root = project.path().join(".emery/spec");
    fs::create_dir_all(&spec_root).expect("spec root");
    fs::write(spec_root.join("current"), "0123456789abcdef\n").expect("pointer");

    let err = home.current().expect_err("a dangling pointer is corruption, not an empty result");
    assert!(err.to_string().contains("spec-home-corrupt"), "typed failure: {err}");
}
