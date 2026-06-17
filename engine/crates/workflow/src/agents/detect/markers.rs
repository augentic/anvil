//! Marker-file parsers — lightweight grammars for TOML, JSON-with-comments,
//! Makefile target lines, and `go.mod` version directives — consumed by
//! the per-language detection passes in [`super::runtimes`].

use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct TomlMarker {
    values: Vec<TomlValue>,
}

impl TomlMarker {
    pub fn parse(contents: &str) -> Result<Self, String> {
        let mut marker = Self::default();
        let mut section: Vec<String> = Vec::new();
        let mut multiline_depth = 0_usize;

        for (index, raw_line) in contents.lines().enumerate() {
            let line_number = index + 1;
            let line = strip_toml_comment(raw_line).trim().to_string();
            if line.is_empty() {
                continue;
            }
            if multiline_depth > 0 {
                multiline_depth = update_delimiter_depth(multiline_depth, &line)
                    .map_err(|err| format!("line {line_number}: {err}"))?;
                continue;
            }
            if line.starts_with('[') {
                section = parse_toml_section(&line)
                    .map_err(|err| format!("line {line_number}: {err}"))?;
                continue;
            }
            let (key, value) =
                parse_toml_assignment(&line).map_err(|err| format!("line {line_number}: {err}"))?;
            multiline_depth =
                delimiter_depth(value).map_err(|err| format!("line {line_number}: {err}"))?;
            marker.values.push(TomlValue {
                section: section.clone(),
                key: key.to_string(),
                value: parse_toml_scalar(value).unwrap_or_default(),
            });
        }

        if multiline_depth > 0 {
            return Err("unterminated array or inline table".to_string());
        }
        Ok(marker)
    }

    pub fn value<const N: usize>(&self, section: [&str; N], key: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|value| {
                value.key == key
                    && value.section.len() == section.len()
                    && value.section.iter().zip(section).all(|(left, right)| left.as_str() == right)
            })
            .map(|value| value.value.as_str())
    }
}

#[derive(Debug, Clone)]
struct TomlValue {
    section: Vec<String>,
    key: String,
    value: String,
}

fn parse_toml_section(line: &str) -> Result<Vec<String>, String> {
    let Some(inner) = line.strip_prefix('[').and_then(|value| value.strip_suffix(']')) else {
        return Err("malformed section header".to_string());
    };
    let inner = inner.strip_prefix('[').and_then(|value| value.strip_suffix(']')).unwrap_or(inner);
    if inner.trim().is_empty() {
        return Err("empty section header".to_string());
    }
    Ok(inner.split('.').map(|part| part.trim().trim_matches('"').to_string()).collect())
}

fn parse_toml_assignment(line: &str) -> Result<(&str, &str), String> {
    let Some((key, value)) = line.split_once('=') else {
        return Err("expected key-value assignment".to_string());
    };
    let key = key.trim();
    let value = value.trim();
    if key.is_empty() {
        return Err("empty key".to_string());
    }
    if value.is_empty() {
        return Err("empty value".to_string());
    }
    Ok((key, value))
}

fn parse_toml_scalar(value: &str) -> Option<String> {
    let value = value.trim();
    if let Some(stripped) = value.strip_prefix('"').and_then(|value| value.strip_suffix('"')) {
        return Some(stripped.to_string());
    }
    if let Some(stripped) = value.strip_prefix('\'').and_then(|value| value.strip_suffix('\'')) {
        return Some(stripped.to_string());
    }
    if value.starts_with('[') || value.starts_with('{') {
        return None;
    }
    Some(value.to_string())
}

fn strip_toml_comment(line: &str) -> String {
    strip_comment(line, '#')
}

pub fn strip_json_comments(contents: &str) -> String {
    contents.lines().map(|line| strip_comment(line, '/')).collect::<Vec<_>>().join("\n")
}

fn strip_comment(line: &str, comment: char) -> String {
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = line.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if !in_string
            && ch == comment
            && (comment != '/' || chars.peek().is_some_and(|(_next_index, next)| *next == '/'))
        {
            return line[..index].to_string();
        }
    }
    line.to_string()
}

fn delimiter_depth(value: &str) -> Result<usize, String> {
    update_delimiter_depth(0, value)
}

fn update_delimiter_depth(mut depth: usize, value: &str) -> Result<usize, String> {
    let mut in_string = false;
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '[' | '{' => depth = depth.saturating_add(1),
            ']' | '}' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| "unexpected closing delimiter".to_string())?;
            }
            _ => {}
        }
    }
    if in_string {
        return Err("unterminated string".to_string());
    }
    Ok(depth)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MakeTargets {
    pub has_test: bool,
    pub has_checks: bool,
}

pub fn parse_make_targets(contents: &str) -> MakeTargets {
    let mut targets = MakeTargets::default();
    for raw_line in contents.lines() {
        let line = raw_line.trim_end();
        if line.starts_with('\t') || line.trim_start().starts_with('#') {
            continue;
        }
        let Some((target_names, _recipe)) = line.split_once(':') else {
            continue;
        };
        if target_names.trim_start().starts_with('.') {
            continue;
        }
        for target in target_names.split_whitespace() {
            match target {
                "test" => targets.has_test = true,
                "checks" => targets.has_checks = true,
                _ => {}
            }
        }
    }
    targets
}

pub fn parse_go_version(contents: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("go ")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

pub fn relative_marker_path(project_dir: &Path, path: &Path) -> String {
    path.strip_prefix(project_dir)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makefile_targets_are_detected_shallowly() {
        let targets = parse_make_targets(
            ".PHONY: test checks\nnot-test:\n\ntest:\n\tcargo test\nchecks: lint\n",
        );

        assert!(targets.has_test);
        assert!(targets.has_checks);
    }

    // A `.PHONY: test checks` declaration names the targets but is itself
    // skipped (it starts with `.`); without a real `test:` / `checks:`
    // rule nothing should be detected. Recipe lines (tab-indented) and
    // comments must not register either. This is the boundary that keeps
    // detection from firing on a bare `.PHONY` line.
    #[test]
    fn make_targets_ignore_phony_recipes_and_comments() {
        let only_phony = parse_make_targets(".PHONY: test checks\n");
        assert!(!only_phony.has_test, "a .PHONY line alone is not a target");
        assert!(!only_phony.has_checks);

        let recipe_and_comment = parse_make_targets("\ttest:\n# checks: real\n");
        assert!(!recipe_and_comment.has_test, "tab-indented line is a recipe, not a target");
        assert!(!recipe_and_comment.has_checks, "commented line is not a target");

        // A multi-name target line and an exact-name requirement.
        let multi = parse_make_targets("build test: deps\ntesting:\n");
        assert!(multi.has_test, "`test` among several names on one line counts");
        let mismatched = parse_make_targets("testing:\n\ttest stuff\n");
        assert!(!mismatched.has_test, "`testing` is not the `test` target");
    }

    #[test]
    fn go_version_parsing() {
        assert_eq!(parse_go_version("module demo\n\ngo 1.22\n").as_deref(), Some("1.22"));
        assert_eq!(parse_go_version("  go 1.21.0  \n").as_deref(), Some("1.21.0"));
        // First `go` directive wins.
        assert_eq!(parse_go_version("go 1.20\ngo 1.30\n").as_deref(), Some("1.20"));
        // `golang` is not the `go ` directive; a bare `go` and an empty
        // version both yield nothing.
        assert_eq!(parse_go_version("golang 1.0\n"), None);
        assert_eq!(parse_go_version("go\n"), None);
        assert_eq!(parse_go_version("go \n"), None);
        assert_eq!(parse_go_version("module demo\n"), None);
    }

    // `strip_json_comments` must only strip `//` that sits outside a JSON
    // string. A `//` inside a string value (the classic `"http://..."`
    // trap), a single `/`, and an escaped quote that keeps the scanner
    // "in string" must all be preserved.
    #[test]
    fn json_comments_respect_strings() {
        assert_eq!(strip_json_comments("{\"a\": 1} // tail").trim_end(), "{\"a\": 1}");
        assert_eq!(strip_json_comments("{\"url\": \"http://x/y\"}"), "{\"url\": \"http://x/y\"}");
        assert_eq!(strip_json_comments("{\"a/b\": 1}"), "{\"a/b\": 1}");
        // The escaped quote keeps the scanner inside the string, so the
        // following `//` is data, not a comment.
        assert_eq!(strip_json_comments("{\"a\\\"//b\": 1}"), "{\"a\\\"//b\": 1}");
    }

    #[test]
    fn toml_marker_reads_nested_and_array_tables() {
        let nested = TomlMarker::parse("[tool.poetry]\nname = \"demo\"\n").expect("parse nested");
        assert_eq!(nested.value(["tool", "poetry"], "name"), Some("demo"));
        // A section-arity mismatch must not match.
        assert_eq!(nested.value(["tool"], "name"), None);

        let array_table =
            TomlMarker::parse("[[bin]]\npath = 'src/main.rs'\n").expect("parse [[..]]");
        assert_eq!(array_table.value(["bin"], "path"), Some("src/main.rs"));

        // Bare scalars round-trip; container values are skipped (None
        // scalar -> stored empty) but must not derail parsing.
        let scalars = TomlMarker::parse(
            "[toolchain]\nchannel = \"stable\" # pinned\ncount = 3\nlist = [1, 2]\n",
        )
        .expect("parse scalars");
        assert_eq!(scalars.value(["toolchain"], "channel"), Some("stable"));
        assert_eq!(scalars.value(["toolchain"], "count"), Some("3"));
        assert_eq!(scalars.value(["toolchain"], "list"), Some(""));

        // A multi-line array balances its delimiters across lines.
        let multiline =
            TomlMarker::parse("members = [\n  \"a\",\n  \"b\",\n]\nedition = \"2021\"\n")
                .expect("parse multiline array");
        assert_eq!(multiline.value([], "edition"), Some("2021"));
    }

    #[test]
    fn toml_marker_rejects_malformed_input() {
        TomlMarker::parse("members = [\n  \"a\",\n").expect_err("unterminated array");
        TomlMarker::parse("count = ]\n").expect_err("unexpected closing delimiter");
        TomlMarker::parse("name = \"unterminated\n").expect_err("unterminated string");
        TomlMarker::parse("[]\n").expect_err("empty section header");
        TomlMarker::parse("= value\n").expect_err("empty key");
    }

    #[test]
    fn relative_marker_path_uses_forward_slashes() {
        let project = Path::new("/proj");
        assert_eq!(
            relative_marker_path(project, Path::new("/proj/nested/Cargo.toml")),
            "nested/Cargo.toml"
        );
        // A path outside the project is returned as-is, not panicked on.
        let outside = relative_marker_path(project, Path::new("/elsewhere/go.mod"));
        assert!(outside.ends_with("go.mod"), "{outside}");
    }
}
