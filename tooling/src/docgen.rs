use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::context::specify_cli_setup_hint;
use crate::error::ToolingError;
use crate::exit::Exit;

pub const BEGIN_MARKER: &str = "<!-- generated:begin -->";
pub const END_MARKER: &str = "<!-- generated:end -->";

const DOC_REL_PATH: &str = "docs/reference/cli-output-shapes.md";

/// `e2e/goldens/<stem>.json` is curated; every fixture must be mapped explicitly
/// so a new fixture forces a deliberate doc decision rather than silently
/// producing a section under a guessed name.
fn e2e_stem_to_group(stem: &str) -> Option<(&'static str, &'static str)> {
    match stem {
        "merge-two-spec" => Some(("specify slice merge run", "two-spec")),
        "task-mark" => Some(("specify slice task mark", "")),
        "task-progress" => Some(("specify slice task progress", "")),
        "validate-good" => Some(("specify slice validate", "clean")),
        "validate-bad" => Some(("specify slice validate", "with-findings")),
        _ => None,
    }
}

#[derive(Debug)]
struct Fixture {
    variant: String,
    body: String,
    rel_path: String,
}

#[derive(Debug)]
struct Group {
    command: String,
    fixtures: Vec<Fixture>,
}

/// Regenerate or verify `docs/reference/cli-output-shapes.md` from CLI fixtures.
pub fn run_envelopes(
    framework_root: &Path,
    specify_cli_dir: &Path,
    verify: bool,
) -> Result<Exit, ToolingError> {
    ensure_specify_cli_fixtures(specify_cli_dir)?;
    let doc_path = framework_root.join(DOC_REL_PATH);
    let generated = render_generated(specify_cli_dir)?;
    let current = fs::read_to_string(&doc_path).map_err(|source| {
        ToolingError::Infrastructure(format!("read {}: {source}", doc_path.display()))
    })?;
    let next = splice_generated(&current, &generated)?;

    if verify {
        if next != current {
            eprintln!(
                "{DOC_REL_PATH} is out of date with the CLI fixtures; run 'cargo docgen-envelopes' to regenerate."
            );
            if let Some(hint) = first_difference(&current, &next) {
                eprintln!("{hint}");
            }
            return Ok(Exit::ValidationFailed);
        }
        return Ok(Exit::Success);
    }

    if next == current {
        println!("{DOC_REL_PATH} already up to date.");
        return Ok(Exit::Success);
    }

    fs::write(&doc_path, &next).map_err(|source| {
        ToolingError::Infrastructure(format!("write {}: {source}", doc_path.display()))
    })?;
    println!("Wrote {DOC_REL_PATH}");
    Ok(Exit::Success)
}

pub fn render_generated(specify_cli_dir: &Path) -> Result<String, ToolingError> {
    ensure_specify_cli_fixtures(specify_cli_dir)?;
    let mut all_groups = load_plan_groups(specify_cli_dir)?;
    all_groups.extend(load_e2e_groups(specify_cli_dir)?);
    all_groups.sort_by(|a, b| a.command.cmp(&b.command));

    let mut sections = Vec::new();
    for group in all_groups {
        sections.push(render_group(&group));
        sections.push(String::new());
    }
    while sections.last().is_some_and(String::is_empty) {
        sections.pop();
    }
    Ok(sections.join("\n"))
}

pub fn splice_generated(current: &str, generated: &str) -> Result<String, ToolingError> {
    let begin_idx = current.find(BEGIN_MARKER).ok_or_else(|| {
        ToolingError::Validation(format!(
            "generation markers {BEGIN_MARKER} / {END_MARKER} not found (or out of order) in {DOC_REL_PATH}"
        ))
    })?;
    let end_idx = current.find(END_MARKER).ok_or_else(|| {
        ToolingError::Validation(format!(
            "generation markers {BEGIN_MARKER} / {END_MARKER} not found (or out of order) in {DOC_REL_PATH}"
        ))
    })?;
    if end_idx < begin_idx {
        return Err(ToolingError::Validation(format!(
            "generation markers {BEGIN_MARKER} / {END_MARKER} not found (or out of order) in {DOC_REL_PATH}"
        )));
    }

    let before = &current[..begin_idx + BEGIN_MARKER.len()];
    let after = &current[end_idx..];
    Ok(format!("{before}\n\n{generated}\n\n{after}"))
}

fn classify_plan(stem: &str) -> (String, String) {
    let Some(dash_idx) = stem.find('-') else {
        return (format!("specify plan {stem}"), String::new());
    };
    let verb = &stem[..dash_idx];
    let variant = stem[dash_idx + 1..].to_string();
    (format!("specify plan {verb}"), variant)
}

fn read_json_fixtures(dir: &Path, specify_cli_dir: &Path) -> Result<Vec<(String, String, String)>, ToolingError> {
    let mut out = Vec::new();
    let entries = fs::read_dir(dir).map_err(|source| {
        ToolingError::Infrastructure(format!("read dir {}: {source}", dir.display()))
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| {
            ToolingError::Infrastructure(format!("read dir entry in {}: {source}", dir.display()))
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".json") {
            continue;
        }
        let body = fs::read_to_string(&path).map_err(|source| {
            ToolingError::Infrastructure(format!("read fixture {}: {source}", path.display()))
        })?;
        let body = body.trim_end().to_string();
        let rel_path = relative_path(specify_cli_dir, &path);
        out.push((name.to_string(), body, rel_path));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn relative_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Report the first line where `current` and `next` diverge, framed for an operator
/// reading a CI log. Returns `None` when the files are byte-identical.
fn first_difference(current: &str, next: &str) -> Option<String> {
    let current_lines: Vec<&str> = current.lines().collect();
    let next_lines: Vec<&str> = next.lines().collect();
    for (idx, (cur, nxt)) in current_lines.iter().zip(next_lines.iter()).enumerate() {
        if cur != nxt {
            let line = idx + 1;
            return Some(format!(
                "first diff at {DOC_REL_PATH}:{line}\n  - {cur}\n  + {nxt}"
            ));
        }
    }
    match current_lines.len().cmp(&next_lines.len()) {
        std::cmp::Ordering::Less => {
            let line = current_lines.len() + 1;
            let added = next_lines.get(current_lines.len()).copied().unwrap_or("");
            Some(format!(
                "first diff at {DOC_REL_PATH}:{line}\n  + {added}\n  (regenerated output adds {} more line(s))",
                next_lines.len() - current_lines.len()
            ))
        }
        std::cmp::Ordering::Greater => {
            let line = next_lines.len() + 1;
            let removed = current_lines.get(next_lines.len()).copied().unwrap_or("");
            Some(format!(
                "first diff at {DOC_REL_PATH}:{line}\n  - {removed}\n  (regenerated output drops {} trailing line(s))",
                current_lines.len() - next_lines.len()
            ))
        }
        std::cmp::Ordering::Equal => None,
    }
}

fn ensure_specify_cli_fixtures(specify_cli_dir: &Path) -> Result<(), ToolingError> {
    let plan_dir = specify_cli_dir.join("tests/fixtures/plan");
    if plan_dir.is_dir() {
        return Ok(());
    }
    Err(ToolingError::Infrastructure(format!(
        "specify-cli fixture tree not found at {}; {}",
        plan_dir.display(),
        specify_cli_setup_hint(specify_cli_dir)
    )))
}

fn load_plan_groups(specify_cli_dir: &Path) -> Result<Vec<Group>, ToolingError> {
    let plan_dir = specify_cli_dir.join("tests/fixtures/plan");
    let mut by_command: BTreeMap<String, Group> = BTreeMap::new();
    for (name, body, rel_path) in read_json_fixtures(&plan_dir, specify_cli_dir)? {
        let stem = name.strip_suffix(".json").unwrap_or(&name);
        let (command, variant) = classify_plan(stem);
        by_command
            .entry(command.clone())
            .or_insert_with(|| Group {
                command,
                fixtures: Vec::new(),
            })
            .fixtures
            .push(Fixture {
                variant,
                body,
                rel_path,
            });
    }
    Ok(groups_to_sorted_vec(by_command))
}

fn load_e2e_groups(specify_cli_dir: &Path) -> Result<Vec<Group>, ToolingError> {
    let e2e_dir = specify_cli_dir.join("tests/fixtures/e2e/goldens");
    let mut by_command: BTreeMap<String, Group> = BTreeMap::new();
    for (name, body, rel_path) in read_json_fixtures(&e2e_dir, specify_cli_dir)? {
        let stem = name.strip_suffix(".json").unwrap_or(&name);
        let Some((command, variant)) = e2e_stem_to_group(stem) else {
            return Err(ToolingError::Validation(format!(
                "Unmapped fixture {rel_path}; add it to E2E_STEM_TO_GROUP in tooling/src/docgen.rs"
            )));
        };
        by_command
            .entry(command.to_string())
            .or_insert_with(|| Group {
                command: command.to_string(),
                fixtures: Vec::new(),
            })
            .fixtures
            .push(Fixture {
                variant: variant.to_string(),
                body,
                rel_path,
            });
    }
    Ok(groups_to_sorted_vec(by_command))
}

fn groups_to_sorted_vec(by_command: BTreeMap<String, Group>) -> Vec<Group> {
    let mut groups: Vec<Group> = by_command.into_values().collect();
    for group in &mut groups {
        group.fixtures.sort_by(|a, b| a.variant.cmp(&b.variant));
    }
    groups.sort_by(|a, b| a.command.cmp(&b.command));
    groups
}

fn render_group(group: &Group) -> String {
    let mut lines = Vec::new();
    lines.push(format!("### `{}`", group.command));
    lines.push(String::new());

    if group.fixtures.len() == 1 {
        let fixture = &group.fixtures[0];
        lines.push(format!("Source fixture: `{}`", fixture.rel_path));
        lines.push(String::new());
        lines.push("```json".to_string());
        lines.push(fixture.body.clone());
        lines.push("```".to_string());
        return lines.join("\n");
    }

    for fixture in &group.fixtures {
        let label = if fixture.variant.is_empty() {
            "default"
        } else {
            &fixture.variant
        };
        lines.push(format!("#### `{label}`"));
        lines.push(String::new());
        lines.push(format!("Source fixture: `{}`", fixture.rel_path));
        lines.push(String::new());
        lines.push("```json".to_string());
        lines.push(fixture.body.clone());
        lines.push("```".to_string());
        lines.push(String::new());
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_plan_splits_verb_and_variant() {
        assert_eq!(
            classify_plan("archive-success"),
            ("specify plan archive".to_string(), "success".to_string())
        );
        assert_eq!(
            classify_plan("create"),
            ("specify plan create".to_string(), String::new())
        );
    }

    #[test]
    fn splice_generated_preserves_markers_and_surrounding_content() {
        let current = format!(
            "header\n{BEGIN_MARKER}\nold\n{END_MARKER}\nfooter"
        );
        let next = splice_generated(&current, "new body").expect("splice succeeds");
        assert_eq!(
            next,
            format!("header\n{BEGIN_MARKER}\n\nnew body\n\n{END_MARKER}\nfooter")
        );
    }

    #[test]
    fn splice_generated_rejects_missing_markers() {
        let err = splice_generated("no markers", "body").unwrap_err();
        assert!(matches!(err, ToolingError::Validation(_)));
    }

    #[test]
    fn render_group_single_fixture_omits_variant_heading() {
        let group = Group {
            command: "specify plan create".to_string(),
            fixtures: vec![Fixture {
                variant: String::new(),
                body: r#"{"ok": true}"#.to_string(),
                rel_path: "tests/fixtures/plan/create-foo.json".to_string(),
            }],
        };
        let rendered = render_group(&group);
        assert!(rendered.contains("### `specify plan create`"));
        assert!(!rendered.contains("#### "));
        assert!(rendered.contains(r#"{"ok": true}"#));
    }

    #[test]
    fn render_group_multiple_fixtures_use_variant_labels() {
        let group = Group {
            command: "specify plan archive".to_string(),
            fixtures: vec![
                Fixture {
                    variant: "success".to_string(),
                    body: "{}".to_string(),
                    rel_path: "a.json".to_string(),
                },
                Fixture {
                    variant: String::new(),
                    body: "{}".to_string(),
                    rel_path: "b.json".to_string(),
                },
            ],
        };
        let rendered = render_group(&group);
        assert!(rendered.contains("#### `success`"));
        assert!(rendered.contains("#### `default`"));
    }

    #[test]
    fn first_difference_returns_none_on_identical_input() {
        assert_eq!(first_difference("a\nb\n", "a\nb\n"), None);
    }

    #[test]
    fn first_difference_points_at_changed_line() {
        let hint = first_difference("alpha\nbeta\ngamma\n", "alpha\nBETA\ngamma\n")
            .expect("difference reported");
        assert!(hint.contains(":2"));
        assert!(hint.contains("- beta"));
        assert!(hint.contains("+ BETA"));
    }

    #[test]
    fn first_difference_handles_added_trailing_lines() {
        let hint = first_difference("alpha\n", "alpha\nbeta\n").expect("difference reported");
        assert!(hint.contains(":2"));
        assert!(hint.contains("+ beta"));
        assert!(hint.contains("adds 1 more line"));
    }

    #[test]
    fn first_difference_handles_removed_trailing_lines() {
        let hint = first_difference("alpha\nbeta\n", "alpha\n").expect("difference reported");
        assert!(hint.contains(":2"));
        assert!(hint.contains("- beta"));
        assert!(hint.contains("drops 1 trailing line"));
    }

    #[test]
    fn e2e_unmapped_fixture_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let e2e = dir.path().join("tests/fixtures/e2e/goldens");
        fs::create_dir_all(&e2e).expect("create e2e dir");
        fs::write(e2e.join("unknown-fixture.json"), "{}").expect("write fixture");
        let err = load_e2e_groups(dir.path()).unwrap_err();
        assert!(matches!(err, ToolingError::Validation(_)));
    }
}
