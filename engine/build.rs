//! Embed a sorted inventory of the `tests/` tree at build time so
//! `specify contract dump` publishes the named-test surface that
//! documentation cites (the cli-contract named-test citations).

use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=tests");
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let tests_dir = manifest_dir.join("tests");
    let mut paths = Vec::new();
    collect(&tests_dir, &tests_dir, &mut paths);
    paths.sort();
    let out_path =
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set")).join("tests-inventory.txt");
    fs::write(&out_path, paths.join("\n")).expect("write tests inventory");
}

/// Recursively collect every file under `dir` as a `tests/`-prefixed
/// forward-slash relative path. A missing directory yields an empty
/// inventory (e.g. a packaged build without the test tree).
fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, out);
        } else if let Ok(relative) = path.strip_prefix(root) {
            let relative = relative.to_string_lossy().replace('\\', "/");
            out.push(format!("tests/{relative}"));
        }
    }
}
