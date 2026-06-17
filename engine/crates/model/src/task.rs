//! `tasks.md` parser. Public surface is `parse_tasks` and
//! `mark_complete`; selection helpers stay private. Format is
//! documented in `plugins/spec/references/specify.md` §"Tasks Document".

use std::sync::OnceLock;

use regex::Regex;
use specify_error::Error;

/// A single task entry parsed from `tasks.md`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Task {
    /// The `##` heading text without the leading `## `, e.g. `"1. Setup"`.
    /// Empty string if the task appears before any `## ` heading.
    pub group: String,
    /// The `X.Y` (or deeper) number captured from the task line, e.g. `"1.2"`.
    pub number: String,
    /// Trimmed task text with any trailing skill-directive comment stripped.
    pub description: String,
    /// `true` iff the checkbox was `[x]` or `[X]`.
    pub complete: bool,
    /// Parsed skill directive, if present.
    pub skill_directive: Option<SkillDirective>,
}

/// A `<!-- skill: plugin:skill -->` directive attached to a task line.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SkillDirective {
    /// Plugin name (e.g. `"omnia"`).
    pub plugin: String,
    /// Skill name within the plugin (e.g. `"crate-writer"`).
    pub skill: String,
}

/// Aggregate task statistics plus the full task list in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    /// Total number of tasks parsed.
    pub total: usize,
    /// Number of tasks marked complete.
    pub complete: usize,
    /// All parsed tasks in document order.
    pub tasks: Vec<Task>,
}

// ---------------------------------------------------------------------------
// Compiled regexes (constructed once, on first use).
// ---------------------------------------------------------------------------

fn task_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Groups:
        //   1 = checkbox contents (" " | "x" | "X")
        //   2 = dotted task number (e.g. "1.2" or "1.2.3")
        //   3 = rest of the line
        Regex::new(r"^\s*-\s+\[( |x|X)\]\s+(\d+(?:\.\d+)*)\s+(.*)$")
            .expect("task line regex is valid")
    })
}

fn skill_directive_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // The word `skill` is matched case-insensitively via `(?i:skill)`;
        // plugin and skill names are case-sensitive because they map to
        // directory names on disk.
        Regex::new(r"<!--\s*(?i:skill)\s*:\s*([a-z0-9_-]+)\s*:\s*([a-z0-9_-]+)\s*-->")
            .expect("skill directive regex is valid")
    })
}

// ---------------------------------------------------------------------------
// parse_tasks
// ---------------------------------------------------------------------------

/// Parse `tasks.md` content into [`Progress`].
///
/// Lenient: unparseable lines are ignored, as are `### …` and deeper
/// headings. Tasks appearing before the first `## ` heading receive
/// `group == ""`.
#[must_use]
pub fn parse_tasks(content: &str) -> Progress {
    let mut current_group = String::new();
    let mut tasks: Vec<Task> = Vec::new();
    let mut complete_count = 0_usize;

    for line in content.lines() {
        // `## ` (exactly two hashes + a space) starts a new group.
        // `###` and deeper headings don't reset the group.
        if let Some(rest) = line.strip_prefix("## ")
            && !rest.starts_with('#')
        {
            current_group = rest.trim_end().to_string();
            continue;
        }

        let Some(caps) = task_line_re().captures(line) else {
            continue;
        };

        let complete = matches!(&caps[1], "x" | "X");
        let number = caps[2].to_string();
        let rest = caps[3].trim();

        let (description, skill_directive) = extract_skill_directive(rest);

        if complete {
            complete_count += 1;
        }

        tasks.push(Task {
            group: current_group.clone(),
            number,
            description,
            complete,
            skill_directive,
        });
    }

    Progress {
        total: tasks.len(),
        complete: complete_count,
        tasks,
    }
}

/// Split a task's rest-of-line into `(description, skill_directive)`.
///
/// If a trailing `<!-- skill: plugin:skill -->` comment is present it is
/// stripped from the description and returned as a [`SkillDirective`].
/// Non-matching comments (e.g. `<!-- TODO: … -->`) are left untouched.
fn extract_skill_directive(rest: &str) -> (String, Option<SkillDirective>) {
    let Some(caps) = skill_directive_re().captures(rest) else {
        return (rest.trim().to_string(), None);
    };

    let m = caps.get(0).expect("group 0 always present on a successful match");
    let directive = SkillDirective {
        plugin: caps[1].to_string(),
        skill: caps[2].to_string(),
    };

    let mut description = String::with_capacity(rest.len());
    description.push_str(&rest[..m.start()]);
    description.push_str(&rest[m.end()..]);

    (description.trim().to_string(), Some(directive))
}

// ---------------------------------------------------------------------------
// mark_complete
// ---------------------------------------------------------------------------

/// Mark the first task line with `number == task_number` complete.
///
/// Idempotent: if the task is already `[x]`/`[X]`, returns the input
/// verbatim. Returns `Error::Diag { code: "task-not-found", .. }`
/// if no task with that number exists.
///
/// When multiple task lines share the same number (user mistake) this
/// targets the first unmarked occurrence; if the first occurrence is already
/// complete the call is treated as a no-op even if later duplicates are
/// unmarked.
///
/// # Errors
///
/// [`Error::Diag`] `task-not-found` if no task line carries
/// `task_number`.
///
/// # Panics
///
/// Panics if the task regex matches a line as unchecked but the line
/// does not contain `[ ]` — this is structurally unreachable.
pub fn mark_complete(content: &str, task_number: &str) -> Result<String, Error> {
    let re = task_line_re();
    let mut first_match: Option<(usize, usize, bool)> = None;

    // Walk lines and record their absolute byte offsets so we can rewrite
    // exactly one checkbox without touching surrounding bytes.
    let mut offset = 0_usize;
    for line in content.split_inclusive('\n') {
        // Line length without the trailing '\n' (if any) — used to scope the
        // regex search to just this line.
        let line_without_nl = line.strip_suffix('\n').unwrap_or(line);

        if let Some(caps) = re.captures(line_without_nl)
            && &caps[2] == task_number
        {
            let checkbox_state = &caps[1];
            let complete = matches!(checkbox_state, "x" | "X");
            first_match = Some((offset, line_without_nl.len(), complete));
            break;
        }

        offset += line.len();
    }

    let Some((line_start, line_len, already_complete)) = first_match else {
        return Err(Error::Diag {
            code: "task-not-found",
            detail: format!("task {task_number} not found"),
        });
    };

    if already_complete {
        return Ok(content.to_string());
    }

    // Find the `[ ]` byte range inside this line. The task regex guarantees
    // `[ ]` is present, so the offset lookup is infallible.
    let line_slice = &content[line_start..line_start + line_len];
    let bracket_rel = line_slice
        .find("[ ]")
        .expect("task regex guarantees '[ ]' is present when checkbox is unchecked");
    let bracket_abs = line_start + bracket_rel;

    let mut out = String::with_capacity(content.len());
    out.push_str(&content[..bracket_abs]);
    out.push_str("[x]");
    out.push_str(&content[bracket_abs + 3..]);
    Ok(out)
}

#[cfg(test)]
mod tests;
