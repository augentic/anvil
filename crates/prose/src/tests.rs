use std::fs;
use std::path::Path;

use super::emit_from;

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, body).expect("write");
}

fn symlink_dir(original: impl AsRef<Path>, link: impl AsRef<Path>) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(original, link).expect("symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(original, link).expect("symlink");
}

// Keep: directory symlink and fenced `](` — live engine tree has neither.
#[test]
fn embeds() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let shared = tmp.path().join("shared");
    write(&shared, "rule.md", "# Rule\n");
    let tree = tmp.path().join("prompts");
    write(&tree, "a.md", "see [b](b.md)\n\n```swift\nprocessEffects([UInt8](effects))\n```\n");
    write(&tree, "b.md", "# B\n");
    symlink_dir(&shared, tree.join("runtime"));
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("mkdir out");

    emit_from(&tree, &out).expect("emit");
    let generated = fs::read_to_string(out.join("prose_docs.rs")).expect("generated");
    assert!(generated.contains("path: \"a.md\""), "{generated}");
    assert!(generated.contains("path: \"b.md\""), "{generated}");
    assert!(generated.contains("path: \"runtime/rule.md\""), "{generated}");
}

// Keep: fail-closed refusals no live corpus can arrange.
#[test]
fn refuses() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("mkdir");

    let dangling = tmp.path().join("dangling");
    write(&dangling, "a.md", "see [missing](nope.md)\n");
    let err = emit_from(&dangling, &out).expect_err("dangling");
    assert!(err.to_string().contains("nope.md"), "{err}");

    let cycle = tmp.path().join("cycle");
    write(&cycle, "intro.md", "# Intro\n");
    symlink_dir(Path::new("."), cycle.join("loop"));
    let err = emit_from(&cycle, &out).expect_err("cycle");
    assert!(err.to_string().contains("symlink cycle"), "{err}");
}
