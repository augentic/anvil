//! Spec-format parsing for requirement blocks, scenarios, and delta
//! sections. Public functions are pure, infallible, and deliberately
//! lenient — coherence checks belong to the merge engine.

use std::sync::OnceLock;

use regex::Regex;

pub mod provenance;

// Hard-coded spec format constants (matches
// `plugins/spec/references/spec-format.md`).

/// Markdown heading prefix for requirement blocks.
pub const REQ_HEADING: &str = "### Requirement:";
/// Line prefix that introduces a requirement's ID.
pub const REQ_ID_PREFIX: &str = "ID:";
/// Regex pattern for valid requirement IDs (`REQ-NNN`).
pub const REQ_ID_PATTERN: &str = r"^REQ-[0-9]{3}$";
/// Regex pattern for valid task IDs (`TASK-NNN`).
pub const TASK_ID_PATTERN: &str = r"^TASK-[0-9]{3}$";
/// Markdown heading prefix for scenario blocks.
pub const SCENARIO_HEADING: &str = "#### Scenario:";
/// Section heading for added requirements in a delta spec.
pub const DELTA_ADDED: &str = "## ADDED Requirements";
/// Section heading for modified requirements in a delta spec.
pub const DELTA_MODIFIED: &str = "## MODIFIED Requirements";
/// Section heading for removed requirements in a delta spec.
pub const DELTA_REMOVED: &str = "## REMOVED Requirements";
/// Section heading for renamed requirements in a delta spec.
pub const DELTA_RENAMED: &str = "## RENAMED Requirements";

// ---------------------------------------------------------------------------
// Public data types
// ---------------------------------------------------------------------------

/// A single requirement block parsed from a spec document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// The full `### Requirement: …` heading line.
    pub heading: String,
    /// Requirement name extracted from the heading.
    pub name: String,
    /// Requirement ID (e.g. `REQ-001`), or empty if no `ID:` line was found.
    pub id: String,
    /// Full body text including heading and scenario lines.
    pub body: String,
    /// Scenarios parsed from `#### Scenario:` sub-headings.
    pub scenarios: Vec<Scenario>,
}

/// A single scenario block within a requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    /// Scenario name extracted from the `#### Scenario:` heading.
    pub name: String,
    /// Full scenario body including its heading line.
    pub body: String,
}

/// Result of parsing a baseline spec document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSpec {
    /// Text before the first requirement heading.
    pub preamble: String,
    /// Requirement blocks in document order.
    pub requirements: Vec<Requirement>,
}

/// Result of parsing a delta spec document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaSpec {
    /// Requirements whose name changed.
    pub renamed: Vec<Rename>,
    /// Requirements that were removed.
    pub removed: Vec<Requirement>,
    /// Requirements that were modified.
    pub modified: Vec<Requirement>,
    /// Requirements that were added.
    pub added: Vec<Requirement>,
}

/// An `ID:` / `TO:` pair from the renamed section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rename {
    /// Requirement ID being renamed.
    pub id: String,
    /// New name for the requirement.
    pub new_name: String,
}

// ---------------------------------------------------------------------------
// ID grammar — the single authority for the closed `REQ` / `TASK` ID shapes.
// The validate and workflow layers call these predicates rather than
// re-encoding the pattern, so the grammar lives in exactly one place.
// ---------------------------------------------------------------------------

fn req_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(REQ_ID_PATTERN).expect("REQ_ID_PATTERN is a valid regex"))
}

fn task_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(TASK_ID_PATTERN).expect("TASK_ID_PATTERN is a valid regex"))
}

/// Return `true` iff `id` is a full-string match for the closed
/// `^REQ-[0-9]{3}$` requirement-ID grammar.
#[must_use]
pub fn is_req_id(id: &str) -> bool {
    full_match(req_id_re(), id)
}

/// Return `true` iff `id` is a full-string match for the closed
/// `^TASK-[0-9]{3}$` task-ID grammar.
#[must_use]
pub fn is_task_id(id: &str) -> bool {
    full_match(task_id_re(), id)
}

/// `true` iff `re` matches `id` spanning the whole string. The anchored `$`
/// in the ID patterns also matches just before a trailing `\n`, so a bare
/// `is_match` would accept `"REQ-001\n"`; the explicit full-span check
/// rejects it, matching the strict grammar both consuming layers enforce.
fn full_match(re: &Regex, id: &str) -> bool {
    re.find(id).is_some_and(|m| m.start() == 0 && m.end() == id.len())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a baseline spec into its preamble and requirement blocks.
///
/// The preamble is every line before the first `### Requirement:` heading or
/// `## `-prefixed heading, whichever appears first. Blocks with no `ID:`
/// line get `id == String::new()` (not `None`, not elided).
#[must_use]
pub fn parse_baseline(text: &str) -> ParsedSpec {
    let lines: Vec<&str> = text.split('\n').collect();
    let heading_prefix = REQ_HEADING;
    let id_prefix = REQ_ID_PREFIX;

    let mut blocks: Vec<Requirement> = Vec::new();
    let mut preamble_lines: Vec<&str> = Vec::new();
    let mut current_lines: Vec<&str> = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_id: Option<String> = None;
    let mut in_preamble = true;

    for line in &lines {
        let stripped = line.trim();

        if let Some(rest) = stripped.strip_prefix(heading_prefix) {
            if in_preamble {
                in_preamble = false;
            } else {
                flush_block(&mut blocks, &mut current_lines, &mut current_name, &mut current_id);
            }
            current_name = Some(rest.trim().to_string());
            current_lines.clear();
            current_lines.push(line);
            continue;
        }

        if !in_preamble
            && current_name.is_some()
            && current_id.is_none()
            && let Some(rest) = stripped.strip_prefix(id_prefix)
        {
            current_id = Some(rest.trim().to_string());
        }

        if in_preamble {
            // A bare `## ` heading (not `### Requirement:`) inside the
            // preamble ends the preamble and begins a pseudo-block with
            // `current_name = None`, which flush_block later discards —
            // the stray header and its trailing lines are dropped.
            if stripped.starts_with("## ") && !stripped.starts_with(heading_prefix) {
                in_preamble = false;
                flush_block(&mut blocks, &mut current_lines, &mut current_name, &mut current_id);
                current_lines.clear();
                current_lines.push(line);
                current_name = None;
            } else {
                preamble_lines.push(line);
            }
        } else {
            current_lines.push(line);
        }
    }
    flush_block(&mut blocks, &mut current_lines, &mut current_name, &mut current_id);

    ParsedSpec {
        preamble: preamble_lines.join("\n"),
        requirements: blocks,
    }
}

/// Parse a delta spec into its four operation sections.
///
/// Section headers are matched case-insensitively on the stripped line.
#[must_use]
pub fn parse_delta(text: &str) -> DeltaSpec {
    #[derive(Copy, Clone)]
    enum Section {
        Renamed,
        Removed,
        Modified,
        Added,
    }

    let op_headings: [(&str, Section); 4] = [
        (DELTA_RENAMED, Section::Renamed),
        (DELTA_REMOVED, Section::Removed),
        (DELTA_MODIFIED, Section::Modified),
        (DELTA_ADDED, Section::Added),
    ];

    let lines: Vec<&str> = text.split('\n').collect();
    let mut renamed_lines: Vec<&str> = Vec::new();
    let mut removed_lines: Vec<&str> = Vec::new();
    let mut modified_lines: Vec<&str> = Vec::new();
    let mut added_lines: Vec<&str> = Vec::new();
    let mut current_section: Option<Section> = None;

    for line in &lines {
        let stripped = line.trim();
        let mut matched: Option<Section> = None;
        for (heading, section) in &op_headings {
            if stripped.eq_ignore_ascii_case(heading) {
                matched = Some(*section);
                break;
            }
        }
        if let Some(section) = matched {
            current_section = Some(section);
            continue;
        }
        match current_section {
            Some(Section::Renamed) => renamed_lines.push(line),
            Some(Section::Removed) => removed_lines.push(line),
            Some(Section::Modified) => modified_lines.push(line),
            Some(Section::Added) => added_lines.push(line),
            None => {}
        }
    }

    // Walk the RENAMED section line-by-line, tracking the last seen
    // `ID:` line and emitting an entry the first time a following `TO:`
    // line shows up. The ID is cleared after a successful emission (so
    // paired lines consume each other); empty-string IDs never emit.
    let mut renamed: Vec<Rename> = Vec::new();
    let id_prefix = REQ_ID_PREFIX;
    let mut current_id: Option<String> = None;
    for line in &renamed_lines {
        let stripped = line.trim();
        if let Some(rest) = stripped.strip_prefix(id_prefix) {
            current_id = Some(rest.trim().to_string());
        } else if stripped.to_ascii_uppercase().starts_with("TO:") {
            let has_usable_id = matches!(&current_id, Some(id) if !id.is_empty());
            if has_usable_id && let Some(id) = current_id.take() {
                let new_name = stripped.get(3..).unwrap_or("").trim().to_string();
                renamed.push(Rename { id, new_name });
            }
        }
    }

    let removed = parse_baseline(&removed_lines.join("\n")).requirements;
    let modified = parse_baseline(&modified_lines.join("\n")).requirements;
    let added = parse_baseline(&added_lines.join("\n")).requirements;

    DeltaSpec {
        renamed,
        removed,
        modified,
        added,
    }
}

/// Return `true` when `text` contains any of the four `## …` delta section
/// headings as a full stripped line, case-insensitive.
///
/// Matching stripped whole lines — the same rule [`parse_delta`] uses to
/// dispatch on a section header — avoids false positives from prose like
/// `"## ADDED Requirements were discussed in the meeting"`.
#[must_use]
pub fn has_delta_headers(text: &str) -> bool {
    let headings = [DELTA_ADDED, DELTA_MODIFIED, DELTA_REMOVED, DELTA_RENAMED];
    for line in text.split('\n') {
        let stripped = line.trim();
        for heading in &headings {
            if stripped.eq_ignore_ascii_case(heading) {
                return true;
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn flush_block(
    blocks: &mut Vec<Requirement>, current_lines: &mut Vec<&str>,
    current_name: &mut Option<String>, current_id: &mut Option<String>,
) {
    if let Some(name) = current_name.take() {
        let heading = current_lines.first().copied().unwrap_or("").to_string();
        let body = current_lines.join("\n");
        let scenarios = parse_scenarios(&body);
        blocks.push(Requirement {
            heading,
            name,
            id: current_id.take().unwrap_or_default(),
            body,
            scenarios,
        });
    }
    current_lines.clear();
    *current_name = None;
    *current_id = None;
}

fn parse_scenarios(body: &str) -> Vec<Scenario> {
    let lines: Vec<&str> = body.split('\n').collect();
    let starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim().starts_with(SCENARIO_HEADING))
        .map(|(idx, _)| idx)
        .collect();

    let mut scenarios = Vec::with_capacity(starts.len());
    for (i, &start) in starts.iter().enumerate() {
        let end = starts.get(i + 1).copied().unwrap_or(lines.len());
        let block_lines = &lines[start..end];
        let name =
            block_lines[0].trim().strip_prefix(SCENARIO_HEADING).unwrap_or("").trim().to_string();
        let scenario_body = block_lines.join("\n");
        scenarios.push(Scenario {
            name,
            body: scenario_body,
        });
    }
    scenarios
}

#[cfg(test)]
mod tests;
