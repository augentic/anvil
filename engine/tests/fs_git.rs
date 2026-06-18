//! Filesystem and git helpers shared across the workspace's test trees.
//!
//! Single source for the `GIT_ENV` / `run_git` / `copy_dir` trio that
//! the binary integration tests (`tests/common/mod.rs`) and the
//! `specify-workflow` / `specify-standards` crate test trees all need.
//! Crate test trees pull it in with a `#[path]` module declaration.

#![allow(
    dead_code,
    reason = "shared test helpers; not every including test binary uses every helper"
)]

use std::fs;
use std::path::Path;
use std::process::Command;

/// Deterministic git author/committer identity for tests that exercise
/// real `git commit` invocations.
pub const GIT_ENV: [(&str, &str); 4] = [
    ("GIT_AUTHOR_NAME", "Specify Test"),
    ("GIT_AUTHOR_EMAIL", "specify-test@example.com"),
    ("GIT_COMMITTER_NAME", "Specify Test"),
    ("GIT_COMMITTER_EMAIL", "specify-test@example.com"),
];

/// Run `git` in `root` with [`GIT_ENV`] applied, asserting success
/// and returning captured stdout.
///
/// # Panics
///
/// Panics if git fails to start or exits non-zero.
pub fn run_git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .envs(GIT_ENV)
        .output()
        .unwrap_or_else(|err| panic!("git {} failed to start: {err}", args.join(" ")));
    assert!(
        output.status.success(),
        "git {} failed\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("git stdout utf8")
}

/// Recursively copy `src` into `dst`, creating directories as needed.
///
/// # Panics
///
/// Panics if `src` cannot be read or a file cannot be copied.
pub fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create_dir_all dst");
    for entry in fs::read_dir(src).expect("read_dir src") {
        let entry = entry.expect("dir entry");
        let target = dst.join(entry.file_name());
        if entry.file_type().expect("file_type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy");
        }
    }
}
