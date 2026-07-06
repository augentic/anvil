//! Logical line joining for `kind: regex` hints.

use std::sync::LazyLock;

use regex::Regex;

static MARKDOWN_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\]\([^)]+\)").expect("markdown link regex"));

/// Join backslash-continued and indented continuation rows into logical lines.
///
/// Returns `(1-based start line, logical text)` pairs mirroring the imperative
/// `prose.invocation-positional` predicate.
#[must_use]
pub fn logical_lines_with_starts(text: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut line_idx = 0;

    while line_idx < lines.len() {
        let start_line = line_idx + 1;
        let mut logical = lines[line_idx].to_string();
        let mut end = line_idx;

        for (next_idx, next_line) in
            lines.iter().enumerate().take(lines.len().min(line_idx + 8)).skip(line_idx + 1)
        {
            let previous_continues = logical.trim_end().ends_with('\\');
            let next_is_indented =
                next_line.chars().next().is_some_and(|ch| ch == ' ' || ch == '\t');
            if !previous_continues && !next_is_indented {
                break;
            }
            logical.push('\n');
            logical.push_str(next_line);
            end = next_idx;
            if !next_line.trim_end().ends_with('\\') && !next_is_indented {
                break;
            }
        }

        out.push((start_line, logical));
        line_idx = end + 1;
    }

    out
}

/// True when a logical line carries flag-style tokens after a slash-skill token
/// without an intervening CLI command (CORE-023 semantics).
#[must_use]
pub fn violates_slash_skill_positional(logical: &str) -> bool {
    static SKILL_TOKEN_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"/[a-z][a-z0-9-]*:[a-z][a-z0-9-]*").expect("skill token"));
    static FLAG_TOKEN_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"--[a-z][a-z0-9-]*").expect("flag token"));
    static CLI_COMMAND_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\b(specify|cargo|gh|git|deno|npm|pnpm|yarn)\s").expect("cli command")
    });

    let scan_logical = MARKDOWN_LINK_RE.replace_all(logical, "]");
    let Some(skill_match) = SKILL_TOKEN_RE.find(&scan_logical) else {
        return false;
    };
    let after_skill = &scan_logical[skill_match.end()..];
    let Some(flag_match) = FLAG_TOKEN_RE.find(after_skill) else {
        return false;
    };
    let between = &after_skill[..flag_match.start()];
    !CLI_COMMAND_RE.is_match(between)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_backslash_continuation() {
        let text = "/spec:build \\\n  --retry\n";
        let pairs = logical_lines_with_starts(text);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, 1);
        assert!(violates_slash_skill_positional(&pairs[0].1));
    }
}
