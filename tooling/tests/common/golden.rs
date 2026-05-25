use std::fs;
use std::path::Path;

use walkdir::WalkDir;

/// `true` when `REGENERATE_GOLDENS=1` is set in the environment.
pub fn regenerate_goldens() -> bool {
    std::env::var("REGENERATE_GOLDENS")
        .map(|value| value == "1")
        .unwrap_or(false)
}

/// Compare `actual` text to a golden file; overwrite when regenerating.
pub fn assert_golden(actual: &str, golden_path: impl AsRef<Path>) -> Result<(), String> {
    let golden_path = golden_path.as_ref();
    if regenerate_goldens() {
        if let Some(parent) = golden_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("create golden parent {}: {err}", parent.display())
            })?;
        }
        fs::write(golden_path, actual).map_err(|err| {
            format!("write golden {}: {err}", golden_path.display())
        })?;
        return Ok(());
    }

    let expected = fs::read_to_string(golden_path).map_err(|_| {
        format!(
            "golden missing: {} — re-run with REGENERATE_GOLDENS=1 to create it",
            golden_path.display()
        )
    })?;

    if actual != expected {
        return Err(format!(
            "golden mismatch: {}\n--- expected ---\n{expected}\n--- actual ---\n{actual}\nre-run with REGENERATE_GOLDENS=1 to update",
            golden_path.display()
        ));
    }
    Ok(())
}

/// Recursive directory diff; overwrite expected tree when regenerating.
pub fn assert_golden_tree(actual_dir: impl AsRef<Path>, expected_dir: impl AsRef<Path>) -> Result<(), String> {
    let actual_dir = actual_dir.as_ref();
    let expected_dir = expected_dir.as_ref();

    let actual = read_tree(actual_dir)?;
    if regenerate_goldens() {
        fs::create_dir_all(expected_dir).map_err(|err| {
            format!("create expected dir {}: {err}", expected_dir.display())
        })?;
        let expected = read_tree(expected_dir).unwrap_or_default();
        for (rel, content) in &actual {
            let path = expected_dir.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|err| {
                    format!("create parent {}: {err}", parent.display())
                })?;
            }
            fs::write(&path, content).map_err(|err| format!("write {}: {err}", path.display()))?;
        }
        for rel in expected.keys() {
            if !actual.contains_key(rel) {
                let path = expected_dir.join(rel);
                let _ = fs::remove_file(&path);
            }
        }
        return Ok(());
    }

    let expected = read_tree(expected_dir).map_err(|err| err.to_string())?;
    let actual_keys: Vec<_> = actual.keys().cloned().collect();
    let expected_keys: Vec<_> = expected.keys().cloned().collect();
    if actual_keys != expected_keys {
        let only_expected: Vec<_> = expected_keys
            .iter()
            .filter(|key| !actual.contains_key(*key))
            .cloned()
            .collect();
        let only_actual: Vec<_> = actual_keys
            .iter()
            .filter(|key| !expected.contains_key(*key))
            .cloned()
            .collect();
        return Err(format!(
            "golden tree file set mismatch under {}:\n  only in expected: {}\n  only in actual:   {}",
            expected_dir.display(),
            if only_expected.is_empty() {
                "<none>".into()
            } else {
                only_expected.join(", ")
            },
            if only_actual.is_empty() {
                "<none>".into()
            } else {
                only_actual.join(", ")
            }
        ));
    }

    for rel in actual_keys {
        if actual.get(&rel) != expected.get(&rel) {
            return Err(format!(
                "golden tree mismatch at {}/{rel}\nre-run with REGENERATE_GOLDENS=1 to update",
                expected_dir.display()
            ));
        }
    }
    Ok(())
}

fn read_tree(root: &Path) -> Result<std::collections::BTreeMap<String, String>, String> {
    if !root.is_dir() {
        return Err(format!("{} is not a directory", root.display()));
    }
    let mut out = std::collections::BTreeMap::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .map_err(|err| err.to_string())?
            .to_string_lossy()
            .into_owned();
        let content = fs::read_to_string(entry.path())
            .map_err(|err| format!("read {}: {err}", entry.path().display()))?;
        out.insert(rel, content);
    }
    Ok(out)
}
