use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use specify_error::Error;
use tempfile::tempdir;

use super::super::change;
use super::super::model::{Entry, Lifecycle, Plan, Status};

fn write_plan(dir: &Path, name: &str, changes: Vec<Entry>) -> PathBuf {
    let plan = Plan {
        name: name.into(),
        lifecycle: Lifecycle::Pending,
        sources: BTreeMap::new(),
        entries: changes,
    };
    let path = dir.join("plan.yaml");
    plan.save(&path).expect("save plan");
    path
}

fn write_plan_with_working_dir(
    dir: &Path, name: &str, changes: Vec<Entry>, files: &[(&str, &[u8])],
) -> PathBuf {
    let plan_path = write_plan(dir, name, changes);

    let specify = dir.join(".specify");
    std::fs::create_dir_all(&specify).expect("mkdir .specify");
    let plans_dir = specify.join("plans").join(name);
    std::fs::create_dir_all(&plans_dir).expect("mkdir plans dir");
    for (filename, bytes) in files {
        let target = plans_dir.join(filename);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).expect("mkdir nested working-dir path");
        }
        std::fs::write(&target, bytes).expect("seed working file");
    }

    plan_path
}

fn archive_test(
    plan_path: &Path, archive_dir: &Path, force: bool,
) -> Result<(PathBuf, Option<PathBuf>), Error> {
    let brief =
        plan_path.parent().map_or_else(|| PathBuf::from("change.md"), |p| p.join("change.md"));
    Plan::archive(plan_path, &brief, archive_dir, force, jiff::Timestamp::now())
}

fn today_yyyymmdd() -> String {
    jiff::Timestamp::now().strftime("%Y%m%d").to_string()
}

#[test]
fn archive_happy_path() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join("archive");
    let plan_path = write_plan(
        tmp.path(),
        "release-1",
        vec![change("a", Status::Done), change("b", Status::Done), change("c", Status::Done)],
    );
    let pre_bytes = std::fs::read(&plan_path).expect("read pre-archive");

    let (dest, plans_dir) = archive_test(&plan_path, &archive_dir, false).expect("archive ok");

    assert!(!plan_path.exists(), "original plan.yaml must be gone after archive");
    assert!(dest.exists(), "destination archive file must exist");
    let expected = archive_dir.join(format!("release-1-{}.yaml", today_yyyymmdd()));
    assert_eq!(dest, expected);
    assert!(plans_dir.is_none(), "no working dir means archived_plans_dir must be None");

    let post_bytes = std::fs::read(&dest).expect("read post-archive");
    assert_eq!(
        pre_bytes, post_bytes,
        "archived file must be byte-identical to the pre-archive plan"
    );
}

#[test]
fn archive_creates_dir() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join("does").join("not").join("exist").join("yet");
    assert!(!archive_dir.exists());
    let plan_path = write_plan(tmp.path(), "proj", vec![change("a", Status::Done)]);

    let (dest, _) = archive_test(&plan_path, &archive_dir, false).expect("archive ok");

    assert!(archive_dir.is_dir(), "archive_dir must be created");
    assert!(dest.starts_with(&archive_dir));
    assert!(dest.exists());
}

#[test]
fn archive_refuses_pending() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join("archive");
    let plan_path = write_plan(
        tmp.path(),
        "p",
        vec![change("done-one", Status::Done), change("still-pending", Status::Pending)],
    );

    let err = archive_test(&plan_path, &archive_dir, false)
        .expect_err("must refuse pending entry without force");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "plan-has-outstanding-work");
            assert!(detail.contains("still-pending"), "detail must name the entry, got: {detail}");
        }
        other => panic!("expected plan-has-outstanding-work diag, got {other:?}"),
    }

    assert!(plan_path.exists(), "original plan.yaml must still exist");
    if archive_dir.exists() {
        let count =
            std::fs::read_dir(&archive_dir).expect("read_dir").filter_map(Result::ok).count();
        assert_eq!(count, 0, "no archived file should have been written");
    }
}

#[test]
fn archive_refuses_nonterminal() {
    // Post-2.0, only `Done` is terminal for archive purposes.
    // `Pending` and `InProgress` block; v1 has no per-entry
    // `failed`/`blocked`/`skipped` state to cover here.
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join("archive");
    let plan_path = write_plan(
        tmp.path(),
        "p",
        vec![
            change("a", Status::Done),
            change("b", Status::InProgress),
            change("c", Status::Pending),
        ],
    );

    let err = archive_test(&plan_path, &archive_dir, false)
        .expect_err("must refuse non-terminal entries");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "plan-has-outstanding-work");
            assert!(
                detail.contains("\"b\"") && detail.contains("\"c\""),
                "detail must list non-Done entries (b, c) in plan list order, got: {detail}"
            );
        }
        other => panic!("expected plan-has-outstanding-work diag, got {other:?}"),
    }
    assert!(plan_path.exists(), "original plan.yaml must still exist");
}

#[test]
fn archive_force_succeeds() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join("archive");
    let plan_path = write_plan(
        tmp.path(),
        "p",
        vec![
            change("a", Status::Done),
            change("b", Status::InProgress),
            change("c", Status::Pending),
        ],
    );
    let pre_bytes = std::fs::read(&plan_path).expect("read pre-archive");

    let (dest, _) = archive_test(&plan_path, &archive_dir, true).expect("force archive ok");

    assert!(!plan_path.exists(), "original plan.yaml must be gone after forced archive");
    let post_bytes = std::fs::read(&dest).expect("read archived file");
    assert_eq!(
        pre_bytes, post_bytes,
        "forced archive must preserve every entry (including non-terminal) verbatim"
    );

    let archived: Plan = serde_saphyr::from_slice(&post_bytes).expect("parse archived");
    let statuses: Vec<Status> = archived.entries.iter().map(|c| c.status).collect();
    assert_eq!(
        statuses,
        vec![Status::Done, Status::InProgress, Status::Pending],
        "statuses in archive must not be rewritten"
    );
}

#[test]
fn archive_filename_format() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join("archive");
    let plan_path = write_plan(tmp.path(), "my-initiative", vec![change("a", Status::Done)]);

    let (dest, _) = archive_test(&plan_path, &archive_dir, false).expect("archive ok");
    let basename = dest.file_name().and_then(|s| s.to_str()).expect("basename utf8");

    let prefix = "my-initiative-";
    let suffix = ".yaml";
    assert!(basename.starts_with(prefix), "basename should start with prefix, got {basename}");
    assert!(basename.ends_with(suffix), "basename should end with .yaml, got {basename}");
    let middle = &basename[prefix.len()..basename.len() - suffix.len()];
    assert_eq!(middle.len(), 8, "date segment must be 8 chars, got {middle}");
    assert!(
        middle.chars().all(|ch| ch.is_ascii_digit()),
        "date segment must be all digits, got {middle}"
    );
}

#[test]
fn archive_refuses_collision() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join("archive");
    std::fs::create_dir_all(&archive_dir).expect("mkdir archive");

    let plan_path = write_plan(tmp.path(), "dup", vec![change("a", Status::Done)]);

    let existing = archive_dir.join(format!("dup-{}.yaml", today_yyyymmdd()));
    std::fs::write(&existing, "unrelated pre-existing archive").expect("seed existing");

    let err = archive_test(&plan_path, &archive_dir, false)
        .expect_err("must refuse when destination exists");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "plan-archive-target-exists");
            assert!(
                detail.contains("already exists"),
                "message should say 'already exists', got: {detail}"
            );
            assert!(detail.contains("git mv"), "message should suggest `git mv`, got: {detail}");
            assert!(
                detail.contains("wait until tomorrow to re-archive"),
                "message should mention the tomorrow-re-archive fallback, got: {detail}"
            );
        }
        other => panic!("expected Error::Diag, got {other:?}"),
    }

    assert!(plan_path.exists(), "original plan.yaml must not be moved");
    let leftover = std::fs::read_to_string(&existing).expect("read existing");
    assert_eq!(
        leftover, "unrelated pre-existing archive",
        "pre-existing archive file must not be overwritten"
    );
}

#[test]
fn archive_returns_dest() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join("archive");
    let plan_path = write_plan(tmp.path(), "pkg", vec![change("a", Status::Done)]);

    let (dest, plans_dir) = archive_test(&plan_path, &archive_dir, false).expect("archive ok");
    let expected = archive_dir.join(format!("pkg-{}.yaml", today_yyyymmdd()));
    assert_eq!(dest, expected);
    assert!(dest.exists(), "returned path must point at an existing file");
    assert!(plans_dir.is_none(), "no working dir co-moved");
}

#[test]
fn archive_co_moves_working_dir() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join(".specify/archive/plans");
    let plan_path = write_plan_with_working_dir(
        tmp.path(),
        "foo",
        vec![change("a", Status::Done)],
        &[("discovery.md", b"# discovery\n"), ("proposal.md", b"# proposal\n")],
    );
    let working_dir = tmp.path().join(".specify/plans/foo");
    let pre_bytes = std::fs::read(&plan_path).expect("read pre-archive");

    let (dest, dest_working) = archive_test(&plan_path, &archive_dir, false).expect("archive ok");

    let today = today_yyyymmdd();
    assert!(!plan_path.exists(), "plan.yaml must be gone");
    assert!(!working_dir.exists(), ".specify/plans/foo/ must be gone");
    assert_eq!(dest, archive_dir.join(format!("foo-{today}.yaml")));
    assert_eq!(
        dest_working,
        Some(archive_dir.join(format!("foo-{today}"))),
        "co-move must surface the destination dir in the return"
    );

    let dest_dir = dest_working.expect("co-moved");
    assert!(dest_dir.is_dir(), "archived plans dir must exist");
    let discovered =
        std::fs::read(dest_dir.join("discovery.md")).expect("read archived discovery.md");
    assert_eq!(discovered, b"# discovery\n");
    let proposal = std::fs::read(dest_dir.join("proposal.md")).expect("read archived proposal.md");
    assert_eq!(proposal, b"# proposal\n");

    let post_bytes = std::fs::read(&dest).expect("read archived plan");
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn archive_without_working_dir() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join(".specify/archive/plans");
    let plan_path = write_plan(tmp.path(), "solo", vec![change("a", Status::Done)]);

    let (dest, dest_working) = archive_test(&plan_path, &archive_dir, false).expect("archive ok");

    assert!(dest.exists(), "archived plan.yaml must exist");
    assert!(dest_working.is_none(), "no working dir -> None");

    let absent_dir = archive_dir.join(format!("solo-{}", today_yyyymmdd()));
    assert!(!absent_dir.exists(), "co-move dir must not be created when source absent");
}

#[test]
fn archive_sweeps_change_brief() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join(".specify/archive/plans");
    let plan_path = write_plan(tmp.path(), "solo", vec![change("a", Status::Done)]);
    let brief_src = tmp.path().join("change.md");
    let brief_bytes = b"---\nname: solo\n---\n\n# Solo\n";
    std::fs::write(&brief_src, brief_bytes).expect("seed change.md");

    let (dest, dest_working) = archive_test(&plan_path, &archive_dir, false).expect("archive ok");

    assert!(dest.exists(), "archived plan.yaml must exist");
    let dest_dir = dest_working.expect("change.md must force the archive dir");
    let archived_brief = dest_dir.join("change.md");
    assert!(archived_brief.is_file(), "archived change.md missing at {}", archived_brief.display());
    assert_eq!(
        std::fs::read(&archived_brief).expect("read archived brief"),
        brief_bytes,
        "archived bytes must equal source bytes"
    );
    assert!(!brief_src.exists(), "source change.md must be gone after move");
}

#[test]
fn sweeps_change_brief_and_working_dir() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join(".specify/archive/plans");
    let plan_path = write_plan_with_working_dir(
        tmp.path(),
        "both",
        vec![change("a", Status::Done)],
        &[("notes.md", b"# notes\n")],
    );
    let brief_src = tmp.path().join("change.md");
    std::fs::write(&brief_src, b"---\nname: both\n---\n\n# Both\n").expect("seed change.md");

    let (_, dest_working) = archive_test(&plan_path, &archive_dir, false).expect("archive ok");

    let dest_dir = dest_working.expect("co-moved");
    assert!(dest_dir.join("notes.md").is_file(), "working-dir file must co-move");
    assert!(dest_dir.join("change.md").is_file(), "change.md must co-move");
}

#[test]
fn archive_moves_workspace_md_and_slices() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join(".specify/archive/plans");
    let plan_path = write_plan_with_working_dir(
        tmp.path(),
        "traffic",
        vec![change("a", Status::Done)],
        &[("workspace.md", b"# workspace\n"), ("slices/x.yaml", b"id: slice-x\n")],
    );

    let (_, dest_working) = archive_test(&plan_path, &archive_dir, false).expect("archive ok");

    let dest_dir = dest_working.expect("co-moved plans dir");
    let wm = dest_dir.join("workspace.md");
    let slice = dest_dir.join("slices").join("x.yaml");
    assert!(wm.is_file(), "workspace.md missing at {}", wm.display());
    assert!(slice.is_file(), "slices/x.yaml missing at {}", slice.display());
    assert_eq!(std::fs::read(&wm).expect("read"), b"# workspace\n");
    assert_eq!(std::fs::read(&slice).expect("read"), b"id: slice-x\n");
}

#[test]
fn archive_refuses_working_dir_collision() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join(".specify/archive/plans");
    let plan_path = write_plan_with_working_dir(
        tmp.path(),
        "foo",
        vec![change("a", Status::Done)],
        &[("discovery.md", b"# discovery\n")],
    );
    let working_dir = tmp.path().join(".specify/plans/foo");

    let today = today_yyyymmdd();
    let clash = archive_dir.join(format!("foo-{today}"));
    std::fs::create_dir_all(&clash).expect("seed collision dir");

    let err = archive_test(&plan_path, &archive_dir, false)
        .expect_err("must refuse when working-dir destination exists");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "plan-archive-target-exists");
            assert!(
                detail.contains("already exists"),
                "message should mention 'already exists', got: {detail}"
            );
            assert!(
                detail.contains(&format!("foo-{today}")),
                "message should name the colliding path, got: {detail}"
            );
            assert!(detail.contains("git mv"), "message should suggest `git mv`, got: {detail}");
            assert!(
                detail.contains("wait until tomorrow to re-archive"),
                "message should mention the tomorrow-re-archive fallback, got: {detail}"
            );
        }
        other => panic!("expected Error::Diag, got {other:?}"),
    }

    assert!(plan_path.exists(), "plan.yaml must be untouched on preflight failure");
    assert!(working_dir.is_dir(), "working dir must be untouched on preflight failure");
    assert!(
        clash.is_dir() && std::fs::read_dir(&clash).expect("read").next().is_none(),
        "pre-existing collision dir must remain empty and untouched"
    );
    let archived_plan = archive_dir.join(format!("foo-{today}.yaml"));
    assert!(!archived_plan.exists(), "plan.yaml must not have been archived");
}

#[test]
fn archive_preserves_bytes() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join(".specify/archive/plans");
    let payload: &[u8] = b"line-1\nline-2\nunicode: caf\xc3\xa9\n";
    let plan_path = write_plan_with_working_dir(
        tmp.path(),
        "bytes",
        vec![change("a", Status::Done)],
        &[("artefact.bin", payload)],
    );

    let (_, dest_working) = archive_test(&plan_path, &archive_dir, false).expect("archive ok");

    let dest_dir = dest_working.expect("co-moved");
    let read_back = std::fs::read(dest_dir.join("artefact.bin")).expect("read archived artefact");
    assert_eq!(read_back, payload, "archived bytes must equal source bytes exactly");
}

#[test]
fn archive_force_moves_working_dir() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join(".specify/archive/plans");
    let plan_path = write_plan_with_working_dir(
        tmp.path(),
        "mixed",
        vec![change("done-one", Status::Done), change("still-pending", Status::Pending)],
        &[("notes.md", b"# notes\n")],
    );
    let working_dir = tmp.path().join(".specify/plans/mixed");
    let pre_bytes = std::fs::read(&plan_path).expect("read pre-archive");

    let (dest, dest_working) =
        archive_test(&plan_path, &archive_dir, true).expect("force archive ok");

    assert!(!plan_path.exists(), "plan.yaml must be gone");
    assert!(!working_dir.exists(), "working dir must be gone");
    assert!(dest.exists());
    let dest_dir = dest_working.expect("force must still co-move");
    assert!(dest_dir.join("notes.md").exists(), "working file must survive the co-move");

    let post_bytes = std::fs::read(&dest).expect("read archived plan");
    assert_eq!(pre_bytes, post_bytes, "forced archive must preserve plan bytes exactly");
}

#[test]
fn archive_atomic() {
    let tmp = tempdir().expect("tempdir");
    let archive_dir = tmp.path().join("archive");
    let plan_path = write_plan(tmp.path(), "atomic", vec![change("a", Status::Done)]);
    let pre_bytes = std::fs::read(&plan_path).expect("read pre-archive");
    let (dest, _) = archive_test(&plan_path, &archive_dir, false).expect("archive ok");
    assert!(!plan_path.exists());
    assert!(dest.exists());
    assert_eq!(std::fs::read(&dest).expect("read"), pre_bytes, "rename-on-same-fs preserves bytes");
}
