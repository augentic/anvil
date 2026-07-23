//! The prose codegen through its public surface: tree discovery,
//! sorted table generation, link checking, and the failure modes a
//! consumer `build.rs` relies on to fail the build.

use std::fs;
use std::path::Path;

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, body).expect("write");
}

#[test]
fn table_is_sorted_and_complete() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let tree = tmp.path().join("prompts");
    write(&tree, "b.md", "# B\n");
    write(&tree, "a.md", "# A\n");
    write(&tree, "nested/c.md", "# C\n");
    write(&tree, "ignored.txt", "not markdown");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("mkdir out");

    prose::emit_from(&tree, &out).expect("emit succeeds");

    let generated = fs::read_to_string(out.join("prose_docs.rs")).expect("generated file");
    let a = generated.find("path: \"a.md\"").expect("a.md embedded");
    let b = generated.find("path: \"b.md\"").expect("b.md embedded");
    let c = generated.find("path: \"nested/c.md\"").expect("nested/c.md embedded");
    assert!(a < b && b < c, "table sorted by tree-relative path");
    assert!(!generated.contains("ignored.txt"), "non-markdown files are skipped");
}

#[test]
fn dangling_link_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let tree = tmp.path().join("prompts");
    write(&tree, "a.md", "see [missing](nope.md)\n");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("mkdir out");

    let err = prose::emit_from(&tree, &out).expect_err("dangling link must fail");
    assert!(err.contains("nope.md"), "{err}");
}

#[test]
fn good_links_pass() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let tree = tmp.path().join("prompts");
    write(&tree, "a.md", "see [b](b.md#anchor), [web](https://example.com), [mail](mailto:x@y)\n");
    write(&tree, "b.md", "# B\n");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("mkdir out");

    prose::emit_from(&tree, &out).expect("emit succeeds");
}

#[test]
fn fenced_code_casts_are_not_links() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let tree = tmp.path().join("prompts");
    write(&tree, "a.md", "see [b](b.md)\n\n```swift\nprocessEffects([UInt8](effects))\n```\n");
    write(&tree, "b.md", "# B\n");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("mkdir out");

    prose::emit_from(&tree, &out).expect("emit succeeds");
}

#[test]
fn empty_tree_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("mkdir out");

    let err = prose::emit_from(&tmp.path().join("prompts"), &out).expect_err("empty tree fails");
    assert!(err.contains("no markdown documents"), "{err}");
}
