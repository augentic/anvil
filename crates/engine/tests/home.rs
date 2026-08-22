//! Output-home integration: the generation-pointer commit contract at
//! the crate's public surface — semantics over the scripted in-memory
//! store, and the byte-for-byte on-disk layout over the filesystem
//! [`Disk`] backing.

#[path = "support/storage.rs"]
mod storage;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use emery_engine::home::{Diff, Home, SpecSet};
use emery_engine::storage::Disk;
use storage::Memory;

fn set(spec: &str) -> SpecSet {
    SpecSet {
        spec: spec.to_string(),
        design: "# Design\n".to_string(),
    }
}

fn generation_object(id: &str, name: &str) -> String {
    format!("generations/{id}/{name}")
}

#[tokio::test]
async fn commit_behind_pointer() {
    let store = Memory::default();
    let home = Home::new(&store);

    let committed = home.commit(&set("# Spec\n")).await.expect("commit");

    let spec = store.object("spec", &generation_object(&committed.id, "spec.md")).expect("spec");
    assert_eq!(spec, b"# Spec\n");
    let design =
        store.object("spec", &generation_object(&committed.id, "design.md")).expect("design");
    assert_eq!(design, b"# Design\n");
    let pointer = store.state("spec/current").expect("pointer");
    assert_eq!(
        String::from_utf8_lossy(&pointer).trim(),
        committed.id,
        "the pointer names the committed generation"
    );
    let current = home.current().await.expect("current").expect("committed");
    assert_eq!(current, committed);
}

#[tokio::test]
async fn rerun_is_byte_stable() {
    let store = Memory::default();
    let home = Home::new(&store);

    home.commit(&set("# Spec\n")).await.expect("first commit");
    let before = store.snapshot();
    home.commit(&set("# Spec\n")).await.expect("re-run commit");

    assert_eq!(before, store.snapshot(), "an identical re-run must be byte-stable");
}

#[tokio::test]
async fn swap_prunes_superseded() {
    let store = Memory::default();
    let home = Home::new(&store);

    let first = home.commit(&set("# Spec v1\n")).await.expect("first commit");
    let second = home.commit(&set("# Spec v2\n")).await.expect("second commit");

    assert_ne!(first.id, second.id);
    assert!(
        store.object("spec", &generation_object(&first.id, "spec.md")).is_none(),
        "the superseded generation is pruned"
    );
    assert_eq!(home.current().await.expect("current").expect("committed").id, second.id);
}

#[tokio::test]
async fn crash_litter_pruned() {
    let store = Memory::default();
    let home = Home::new(&store);
    let committed = home.commit(&set("# Spec\n")).await.expect("commit");

    // A crash between generation writes and pointer swap leaves a
    // partial generation; the pointer still names the previous set.
    let partial = generation_object("deadbeef", "spec.md");
    store.insert_object("spec", &partial, b"half-written");

    let current = home.current().await.expect("current").expect("committed");
    assert_eq!(current.id, committed.id, "readers trust only what the pointer names");

    home.commit(&set("# Spec\n")).await.expect("re-run commit");
    assert!(store.object("spec", &partial).is_none(), "crash litter is pruned on the next commit");
}

// One parseable requirement block for the diff kernel's spec fixtures.
fn block(id: u32, name: &str, body: &str) -> String {
    format!(
        "### Requirement: {name}\n\nID: REQ-{id:03}\nSources: [mock-docs]\nStatus: agreed\n\n{body}\n\n"
    )
}

#[tokio::test]
async fn outgoing_reads_current() {
    let store = Memory::default();
    let home = Home::new(&store);
    assert!(home.outgoing().await.is_none(), "no generation, no outgoing set");

    let committed = home.commit(&set("# Spec\n")).await.expect("commit");
    let (from, outgoing) = home.outgoing().await.expect("outgoing after commit");
    assert_eq!(from, committed.id, "the outgoing id is the pointer's");
    assert_eq!(outgoing, set("# Spec\n"), "the outgoing set reads back verbatim");
}

// The re-mine diff names changed artifacts and `spec.md` sections
// by heading subject — immune to positional `REQ-NNN` shifts when
// blocks are inserted or removed.
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
        design: "# Design v1\n".to_string(),
        ..set(&old_spec)
    };
    let incoming = SpecSet {
        design: "# Design v2\n".to_string(),
        ..set(&new_spec)
    };

    let diff = Diff::between("cafe".to_string(), &outgoing, &incoming);

    assert!(!diff.is_empty());
    assert_eq!(diff.from, "cafe");
    assert_eq!(diff.artifacts, ["spec.md", "design.md"], "changed artifacts in set order");
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

#[tokio::test]
async fn dangling_pointer_fails() {
    let store = Memory::default();
    let home = Home::new(&store);
    store.insert_state("spec/current", b"0123456789abcdef\n");

    let err =
        home.current().await.expect_err("a dangling pointer is corruption, not an empty result");
    assert!(err.to_string().contains("spec-home-corrupt"), "typed failure: {err}");
}

// Every file under `dir` by relative path and bytes.
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

// The "no observable change" claim, tested: the filesystem backing
// reproduces the pre-seam `.emery/spec/` tree byte-for-byte —
// pointer, generation documents, swap-prune, and crash-litter prune.
#[tokio::test]
async fn disk_layout_byte_for_byte() {
    let project = tempfile::tempdir().expect("tempdir");
    let disk = Disk::rooted(project.path());
    let home = Home::new(&disk);

    let committed = home.commit(&set("# Spec\n")).await.expect("commit");
    let generation = project.path().join(".emery/spec/generations").join(&committed.id);
    assert_eq!(fs::read_to_string(generation.join("spec.md")).expect("spec"), "# Spec\n");
    assert_eq!(fs::read_to_string(generation.join("design.md")).expect("design"), "# Design\n");
    let pointer = fs::read_to_string(project.path().join(".emery/spec/current")).expect("pointer");
    assert_eq!(pointer, format!("{}\n", committed.id), "the pointer file is `<id>\\n`");

    // An identical re-run leaves the tree byte-stable.
    let before = snapshot(project.path());
    home.commit(&set("# Spec\n")).await.expect("re-run commit");
    assert_eq!(before, snapshot(project.path()), "an identical re-run must be byte-stable");

    // Crash litter — a partial generation directory and a stray temp
    // file at the spec root — is pruned on the next commit.
    let partial = project.path().join(".emery/spec/generations/deadbeef");
    fs::create_dir_all(&partial).expect("partial dir");
    fs::write(partial.join("spec.md"), "half-written").expect("partial file");
    fs::write(project.path().join(".emery/spec/.tmpXYZ"), "temp litter").expect("temp litter");

    let second = home.commit(&set("# Spec v2\n")).await.expect("second commit");
    assert!(!partial.exists(), "crash litter is pruned");
    assert!(!project.path().join(".emery/spec/.tmpXYZ").exists());
    assert!(!generation.exists(), "the superseded generation is pruned");
    assert_eq!(
        snapshot(project.path()).into_keys().collect::<Vec<_>>(),
        [
            PathBuf::from(".emery/spec/current"),
            PathBuf::from(format!(".emery/spec/generations/{}/design.md", second.id)),
            PathBuf::from(format!(".emery/spec/generations/{}/spec.md", second.id)),
        ],
        "exactly the pointer and the one named generation survive"
    );
}
