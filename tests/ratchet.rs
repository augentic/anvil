//! LOC ratchet + prose budgets over `scripts/ratchet.toml`
//! (CONSTITUTION.md invariants 2 and 4, remediation Phase 2): growth
//! past a committed ceiling is a red build demanding an ADR reference.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// The committed baseline file, workspace-relative.
const BASELINE: &str = "scripts/ratchet.toml";

#[test]
fn rust_ceilings() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ceilings = section(root, "rust");

    // The measured set is every workspace crate plus the root `src/`;
    // an uncovered tree or a stale entry is a violation, so the
    // baseline can never silently drift from the tree.
    let mut measured = BTreeMap::new();
    measured.insert("src".to_owned(), count(&root.join("src"), &["rs"]));
    let crates = fs::read_dir(root.join("crates")).expect("crates directory");
    for entry in crates {
        let dir = entry.expect("crate directory entry").path();
        if dir.is_dir() {
            let name = dir.file_name().expect("crate name").to_string_lossy();
            measured.insert(format!("crates/{name}"), count(&dir.join("src"), &["rs"]));
        }
    }

    enforce("rust", &ceilings, &measured);
}

#[test]
fn prose_ceilings() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ceilings = section(root, "prose");

    let measured = ceilings
        .keys()
        .map(|key| {
            let path = root.join(key);
            let lines =
                if path.is_dir() { count(&path, &["md", "mdc"]) } else { count_file(&path) };
            (key.clone(), lines)
        })
        .collect();

    enforce("prose", &ceilings, &measured);
}

/// Compare measured counts against ceilings; any excess, uncovered
/// tree, or stale entry fails with the ADR demand.
fn enforce(kind: &str, ceilings: &BTreeMap<String, usize>, measured: &BTreeMap<String, usize>) {
    let mut violations = String::new();
    for (key, lines) in measured {
        match ceilings.get(key) {
            None => {
                writeln!(violations, "  {key} has no [{kind}] ceiling in {BASELINE}")
                    .expect("infallible write to String");
            }
            Some(ceiling) if lines > ceiling => {
                writeln!(
                    violations,
                    "  {key} grew past its ceiling ({lines} > {ceiling} non-blank lines)"
                )
                .expect("infallible write to String");
            }
            Some(_) => {}
        }
    }
    for key in ceilings.keys() {
        if !measured.contains_key(key) {
            writeln!(violations, "  {BASELINE} entry `{key}` measures nothing that exists")
                .expect("infallible write to String");
        }
    }
    assert!(
        violations.is_empty(),
        "ratchet violations (CONSTITUTION.md invariant 4). Raising a ceiling in {BASELINE} \
         requires an ADR reference in the same change; shrinking is free:\n{violations}"
    );
}

/// Parse one `[section]` of the baseline: `"key" = N` lines only.
fn section(root: &Path, wanted: &str) -> BTreeMap<String, usize> {
    let text = fs::read_to_string(root.join(BASELINE)).expect("read ratchet baseline");
    let mut current = String::new();
    let mut entries = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            current = name.to_owned();
            continue;
        }
        if current != wanted {
            continue;
        }
        let (key, value) = line.split_once('=').expect("`\"key\" = N` baseline entry");
        let key = key.trim().trim_matches('"').to_owned();
        let value = value.trim().parse().expect("integer ratchet ceiling");
        assert!(entries.insert(key, value).is_none(), "duplicate ratchet entry in [{wanted}]");
    }
    entries
}

/// Non-blank lines across every `.{exts}` file under `dir`
/// (absent dirs count zero, so `src`-less crates stay covered).
fn count(dir: &Path, exts: &[&str]) -> usize {
    let Ok(entries) = fs::read_dir(dir) else { return 0 };
    let mut lines = 0;
    for entry in entries {
        let path = entry.expect("source directory entry").path();
        if path.is_dir() {
            lines += count(&path, exts);
        } else if path.extension().is_some_and(|found| exts.iter().any(|ext| found == *ext)) {
            lines += count_file(&path);
        }
    }
    lines
}

fn count_file(path: &Path) -> usize {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read counted file {}: {error}", path.display()));
    text.lines().filter(|line| !line.trim().is_empty()).count()
}
