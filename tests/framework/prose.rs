//! Prose and manifest predicates: marketplace↔plugins drift, skill
//! numeric-cap drift, the canonical review-team-protocol document,
//! reference-corpus indexes, retired design-history citations, and
//! text pipeline diagrams in docs.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value as JsonValue;

use crate::support::{Finding, extract_fenced_blocks, rel, walk_files, walk_markdown};

/// The marketplace manifest disagrees with the on-disk plugin layout.
pub const CHECK_MARKETPLACE_DRIFT: &str = "plugins.marketplace-drift";
/// A documented skill numeric cap drifted from its canonical source.
pub const CHECK_NUMERIC_CAP: &str = "prose.numeric-cap-exceeded";
/// The canonical review-team-protocol document is missing.
pub const CHECK_CANONICAL_MISSING: &str = "agent-teams.missing-canonical";
/// A reference corpus holds two or more files but no README.md index.
pub const CHECK_CORPUS_UNINDEXED: &str = "reference.corpus-unindexed";
/// Prose cites a retired design-history RFC number below 100.
pub const CHECK_HISTORY_CITATION: &str = "docs.history-citation";
/// A docs page draws a flow diagram inside a fenced `text` block.
pub const CHECK_TEXT_DIAGRAM: &str = "docs.text-pipeline-diagram";

/// Skill description character cap (must match `skill.schema.json`
/// `description.maxLength` and the authoring standard prose).
const DESCRIPTION_CAP: u64 = 512;
/// Skill body line cap (must appear in the authoring standard prose).
const BODY_CAP: u64 = 200;
/// Standards document that must carry both numeric caps in prose.
const STANDARDS_REL: &str = "docs/standards/skill-authoring.md";

/// Canonical document per-target `agent-teams.md` overlays resolve to.
const CANONICAL_REVIEW_PROTOCOL: &str = "docs/reference/review-team-protocol.md";

/// Reference-corpus roots (one directory depth per `*`; `*` does not
/// cross `/`) that owe a `README.md` index at two or more files.
const CORPUS_ROOTS: &[&str] =
    &["sources/*/prose/references/*", "targets/*/prose/references/*", "codex/references/*"];
const CORPUS_INDEX: &str = "README.md";
const CORPUS_MIN_FILES: usize = 2;

/// Trees scanned for retired design-history citations.
const HISTORY_SCOPE_PREFIXES: &[&str] = &["docs/", "codex/", "sources/", "targets/", "plugins/"];
/// Subtrees excluded from the history-citation scan.
const HISTORY_EXCLUDED_PREFIXES: &[&str] = &["docs/assets/", "codex/rules/"];

/// Docs subtrees where fenced `text` flow diagrams are banned.
const TEXT_DIAGRAM_PREFIXES: &[&str] =
    &["docs/explanation/", "docs/orientation/", "docs/tutorials/", "docs/how-to/"];

/// Run every prose predicate rooted at `root`.
pub fn run(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    check_marketplace_drift(root, &mut findings);
    check_numeric_caps(root, &mut findings);
    check_canonical_doc(root, &mut findings);
    check_corpus_indexes(root, &mut findings);
    check_history_citations(root, &mut findings);
    check_text_diagrams(root, &mut findings);
    findings
}

const fn marketplace_drift(message: String) -> Finding {
    Finding::new(CHECK_MARKETPLACE_DRIFT, message)
}

/// The `.cursor-plugin/marketplace.json` manifest must satisfy the
/// embedded marketplace schema and agree bidirectionally with the
/// on-disk `plugins/` layout (an absent manifest is a legitimate skip).
fn check_marketplace_drift(root: &Path, findings: &mut Vec<Finding>) {
    let manifest_path = root.join(".cursor-plugin").join("marketplace.json");
    if !manifest_path.exists() {
        return;
    }
    let manifest_rel = rel(root, &manifest_path);
    let Ok(contents) = fs::read_to_string(&manifest_path) else {
        findings.push(marketplace_drift(format!("{manifest_rel} — cannot read manifest")));
        return;
    };
    let value: JsonValue = match serde_json::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            findings.push(marketplace_drift(format!("{manifest_rel} — cannot parse: {error}")));
            return;
        }
    };

    match schema::cached_validator(schema::MARKETPLACE_JSON_SCHEMA) {
        Ok(validator) => {
            let errors: Vec<Finding> = validator
                .iter_errors(&value)
                .map(|error| {
                    marketplace_drift(format!(
                        "{manifest_rel} — schema violation at {}: {error}",
                        error.instance_path()
                    ))
                })
                .collect();
            if !errors.is_empty() {
                findings.extend(errors);
                return;
            }
        }
        Err(error) => {
            findings.push(marketplace_drift(format!(
                "{manifest_rel} — cannot compile marketplace schema: {error}"
            )));
            return;
        }
    }

    let Some(plugins) = value.get("plugins").and_then(JsonValue::as_array) else {
        findings.push(marketplace_drift(format!("{manifest_rel} — missing plugins array")));
        return;
    };
    let declared: BTreeSet<&str> =
        plugins.iter().filter_map(|p| p.get("source").and_then(JsonValue::as_str)).collect();

    let plugins_dir = root.join("plugins");
    if let Ok(entries) = fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() || !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().join(".cursor-plugin").join("plugin.json").is_file()
                && !declared.contains(name.as_str())
            {
                findings.push(marketplace_drift(format!(
                    "plugin '{name}' has .cursor-plugin/plugin.json but is not in \
                     marketplace.json"
                )));
            }
        }
    }

    for plugin in plugins {
        let Some(name) = plugin.get("name").and_then(JsonValue::as_str) else {
            continue;
        };
        let Some(source) = plugin.get("source").and_then(JsonValue::as_str) else {
            continue;
        };
        let plugin_root = plugins_dir.join(source);
        if !plugin_root.join("skills").is_dir() {
            findings.push(marketplace_drift(format!(
                "plugin '{name}' declared in marketplace.json but skills/ not found"
            )));
            continue;
        }
        if !plugin_root.join(".cursor-plugin").join("plugin.json").is_file() {
            findings.push(marketplace_drift(format!(
                "plugin '{name}' has skills/ but .cursor-plugin/plugin.json not found"
            )));
        }
    }
}

/// The skill description / body caps must stay in sync across the
/// embedded skill schema and the authoring standard prose.
fn check_numeric_caps(root: &Path, findings: &mut Vec<Finding>) {
    let standards_path = root.join(STANDARDS_REL);
    if !standards_path.exists() {
        findings.push(Finding::new(
            CHECK_NUMERIC_CAP,
            format!("skill numeric cap source missing: {STANDARDS_REL}"),
        ));
        return;
    }

    match schema_description_max_length() {
        Some(max_length) if max_length == DESCRIPTION_CAP => {}
        Some(max_length) => findings.push(Finding::new(
            CHECK_NUMERIC_CAP,
            format!(
                "skill.schema.json description.maxLength is {max_length}, expected \
                 {DESCRIPTION_CAP}"
            ),
        )),
        None => findings.push(Finding::new(
            CHECK_NUMERIC_CAP,
            "skill.schema.json declares no description.maxLength".to_owned(),
        )),
    }

    let Ok(content) = fs::read_to_string(&standards_path) else {
        findings.push(Finding::new(
            CHECK_NUMERIC_CAP,
            format!("skill numeric cap source unreadable: {STANDARDS_REL}"),
        ));
        return;
    };
    if !content.contains(&DESCRIPTION_CAP.to_string()) {
        findings.push(Finding::new(
            CHECK_NUMERIC_CAP,
            format!("description cap {DESCRIPTION_CAP} missing from {STANDARDS_REL}"),
        ));
    }
    if !content.contains(&BODY_CAP.to_string()) {
        findings.push(Finding::new(
            CHECK_NUMERIC_CAP,
            format!("body cap {BODY_CAP} missing from {STANDARDS_REL}"),
        ));
    }
}

fn schema_description_max_length() -> Option<u64> {
    let schema: JsonValue = serde_json::from_str(schema::SKILL_JSON_SCHEMA).ok()?;
    schema.get("properties")?.get("description")?.get("maxLength").and_then(JsonValue::as_u64)
}

/// The canonical review-team-protocol document must exist: per-target
/// `agent-teams.md` overlays in the adapters repo symlink to it.
fn check_canonical_doc(root: &Path, findings: &mut Vec<Finding>) {
    if !root.join(CANONICAL_REVIEW_PROTOCOL).is_file() {
        findings.push(Finding::new(
            CHECK_CANONICAL_MISSING,
            format!("required file '{CANONICAL_REVIEW_PROTOCOL}' is missing"),
        ));
    }
}

/// Each directory matching a corpus root glob with at least
/// `CORPUS_MIN_FILES` files beneath it must carry a `README.md`
/// directly inside it.
fn check_corpus_indexes(root: &Path, findings: &mut Vec<Finding>) {
    let mut files = Vec::new();
    for top in ["sources", "targets", "codex", "adapters"] {
        let dir = root.join(top);
        if dir.is_dir() {
            walk_files(&dir, &mut files);
        }
    }
    let relatives: Vec<String> = files.iter().map(|path| rel(root, path)).collect();

    let mut corpus_dirs: BTreeSet<String> = BTreeSet::new();
    for relative in &relatives {
        let mut prefix = relative.as_str();
        while let Some(pos) = prefix.rfind('/') {
            prefix = &prefix[..pos];
            if CORPUS_ROOTS.iter().any(|pattern| glob_dir_matches(pattern, prefix)) {
                corpus_dirs.insert(prefix.to_owned());
            }
        }
    }

    for dir in corpus_dirs {
        let beneath = format!("{dir}/");
        let count = relatives.iter().filter(|r| r.starts_with(&beneath)).count();
        if count < CORPUS_MIN_FILES {
            continue;
        }
        let required = format!("{dir}/{CORPUS_INDEX}");
        if relatives.contains(&required) {
            continue;
        }
        findings.push(Finding::new(
            CHECK_CORPUS_UNINDEXED,
            format!(
                "reference directory '{dir}' ({count} files) is missing its '{CORPUS_INDEX}' \
                 index"
            ),
        ));
    }
}

/// Segment-wise glob match where `*` matches exactly one path segment
/// (never crosses `/`).
fn glob_dir_matches(pattern: &str, dir: &str) -> bool {
    let pattern_segments: Vec<&str> = pattern.split('/').collect();
    let dir_segments: Vec<&str> = dir.split('/').collect();
    pattern_segments.len() == dir_segments.len()
        && pattern_segments.iter().zip(&dir_segments).all(|(p, d)| *p == "*" || p == d)
}

static RFC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)RFC[-\s]+(\d+)").expect("rfc citation pattern"));

/// Retired Specify design-history citations (`RFC-N`, N < 100) must
/// not appear in operator-facing prose; standards RFCs (>= 100) pass.
fn check_history_citations(root: &Path, findings: &mut Vec<Finding>) {
    let mut paths = Vec::new();
    for prefix in HISTORY_SCOPE_PREFIXES {
        let dir = root.join(prefix.trim_end_matches('/'));
        if dir.is_dir() {
            paths.extend(walk_markdown(&dir));
        }
    }
    let agents = root.join("AGENTS.md");
    if agents.is_file() {
        paths.push(agents);
    }

    for path in paths {
        let relative = rel(root, &path);
        if HISTORY_EXCLUDED_PREFIXES.iter().any(|prefix| relative.starts_with(prefix)) {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for (line_idx, line) in content.lines().enumerate() {
            for cap in RFC_RE.captures_iter(line) {
                let number: u64 = cap[1].parse().unwrap_or(u64::MAX);
                if number < 100 {
                    findings.push(Finding::new(
                        CHECK_HISTORY_CITATION,
                        format!(
                            "{relative}:{} — retired design-history citation '{}'",
                            line_idx + 1,
                            &cap[0]
                        ),
                    ));
                }
            }
        }
    }
}

/// Prose docs must not draw pipeline diagrams inside a fenced `text`
/// block; one containing a flow arrow is a diagram in disguise
/// (author an SVG under `docs/assets/diagrams/` instead).
fn check_text_diagrams(root: &Path, findings: &mut Vec<Finding>) {
    for prefix in TEXT_DIAGRAM_PREFIXES {
        let dir = root.join(prefix.trim_end_matches('/'));
        if !dir.is_dir() {
            continue;
        }
        for path in walk_markdown(&dir) {
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let relative = rel(root, &path);
            for block in extract_fenced_blocks(&content) {
                if block.lang == "text" && (block.body.contains("->") || block.body.contains('→'))
                {
                    findings.push(Finding::new(
                        CHECK_TEXT_DIAGRAM,
                        format!(
                            "{relative}:{} — flow diagram inside a ```text fence; author an SVG \
                             under docs/assets/diagrams/ instead",
                            block.line_start
                        ),
                    ));
                }
            }
        }
    }
}
