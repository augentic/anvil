use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use walkdir::WalkDir;

use crate::context::Context;
use crate::error::ToolingError;
use crate::finding::{Check, Finding, Location};
use crate::helpers::under_symlink;

const RULE_INVOCATION_POSITIONAL: &str = "prose.invocation-positional";
const RULE_OPERATIONAL_VOCABULARY: &str = "prose.operational-vocabulary";
const RULE_NUMERIC_CAP_EXCEEDED: &str = "prose.numeric-cap-exceeded";

const EXPECTED_DESCRIPTION_CAP: usize = 512;
const EXPECTED_BODY_CAP: usize = 200;

struct ForbiddenPattern {
    pattern: Regex,
    fix: &'static str,
}

/// Slash-skill invocations must stay positional — no `--flags` after the skill token.
pub struct InvocationPositional;

/// Retired Specify vocabulary must not appear outside allowlisted paths.
pub struct OperationalVocabulary;

/// Skill description/body numeric caps must stay in sync across schema, standards, and checks.
pub struct SkillNumericCaps;

impl Check for InvocationPositional {
    fn id(&self) -> &'static str {
        RULE_INVOCATION_POSITIONAL
    }

    fn run(&self, ctx: &Context) -> Vec<Finding> {
        check_invocation_positionals(ctx.framework_root()).unwrap_or_else(|error| {
            vec![infrastructure_finding(RULE_INVOCATION_POSITIONAL, error)]
        })
    }
}

impl Check for OperationalVocabulary {
    fn id(&self) -> &'static str {
        RULE_OPERATIONAL_VOCABULARY
    }

    fn run(&self, ctx: &Context) -> Vec<Finding> {
        check_operational_vocabulary(ctx.framework_root()).unwrap_or_else(|error| {
            vec![infrastructure_finding(RULE_OPERATIONAL_VOCABULARY, error)]
        })
    }
}

impl Check for SkillNumericCaps {
    fn id(&self) -> &'static str {
        RULE_NUMERIC_CAP_EXCEEDED
    }

    fn run(&self, ctx: &Context) -> Vec<Finding> {
        check_skill_numeric_caps(ctx.framework_root())
    }
}

fn check_operational_vocabulary(framework_root: &Path) -> Result<Vec<Finding>, ToolingError> {
    let scan_roots = [
        framework_root.join("docs"),
        framework_root.join("plugins"),
        framework_root.join(".cursor"),
    ];
    let scan_files = [
        framework_root.join("AGENTS.md"),
        framework_root.join("README.md"),
    ];
    let allowed_prefixes = [
        "rfcs/",
        "docs/explanation/decision-log.md",
        "docs/explanation/release-notes.md",
    ];
    let allowed_segments = ["/fixtures/", "/archive/"];
    let forbidden = forbidden_patterns();

    let mut targets = collect_walk_targets(&scan_roots, &["md", "mdc", "json", "yaml", "yml"], framework_root)?;
    for path in scan_files {
        if path.is_file() {
            targets.push(path);
        }
    }
    targets.sort();
    targets.dedup();

    let mut findings = Vec::new();
    for path in targets {
        let rel = path
            .strip_prefix(framework_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if allowed_prefixes.iter().any(|prefix| rel.starts_with(prefix)) {
            continue;
        }
        if allowed_segments.iter().any(|segment| rel.contains(segment)) {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => continue,
        };
        for (line_idx, line) in content.lines().enumerate() {
            for entry in &forbidden {
                if entry.pattern.is_match(line) {
                    findings.push(Finding {
                        rule_id: RULE_OPERATIONAL_VOCABULARY,
                        message: format!(
                            "Stale Specify vocabulary in {rel}:{} -- {} -- {}",
                            line_idx + 1,
                            line.trim(),
                            entry.fix
                        ),
                        location: Some(Location {
                            path: path.clone(),
                            line: line_idx + 1,
                            column: None,
                        }),
                    });
                }
            }
        }
    }

    Ok(findings)
}

fn check_skill_numeric_caps(framework_root: &Path) -> Vec<Finding> {
    let files: [(&str, bool, bool); 4] = [
        (".cursor/schemas/skill.schema.json", true, false),
        ("docs/standards/skill-authoring.md", true, true),
        ("tooling/src/check/skill_frontmatter.rs", true, false),
        ("tooling/src/check/skill_body.rs", false, true),
    ];

    let mut findings = Vec::new();
    for (rel, checks_description, checks_body) in files {
        let path = framework_root.join(rel);
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => {
                findings.push(Finding {
                    rule_id: RULE_NUMERIC_CAP_EXCEEDED,
                    message: format!("Skill numeric cap source missing: {rel}"),
                    location: Some(Location {
                        path: path.clone(),
                        line: 1,
                        column: None,
                    }),
                });
                continue;
            }
        };

        if checks_description && !content.contains(&EXPECTED_DESCRIPTION_CAP.to_string()) {
            findings.push(Finding {
                rule_id: RULE_NUMERIC_CAP_EXCEEDED,
                message: format!(
                    "Skill description cap drift in {rel}; expected {EXPECTED_DESCRIPTION_CAP}"
                ),
                location: Some(Location {
                    path: path.clone(),
                    line: 1,
                    column: None,
                }),
            });
        }
        if checks_body && !content.contains(&EXPECTED_BODY_CAP.to_string()) {
            findings.push(Finding {
                rule_id: RULE_NUMERIC_CAP_EXCEEDED,
                message: format!("Skill body cap drift in {rel}; expected {EXPECTED_BODY_CAP}"),
                location: Some(Location {
                    path: path.clone(),
                    line: 1,
                    column: None,
                }),
            });
        }
    }

    findings
}

fn check_invocation_positionals(framework_root: &Path) -> Result<Vec<Finding>, ToolingError> {
    let scan_roots = [
        framework_root.join("docs"),
        framework_root.join("plugins"),
        framework_root.join("adapters").join("sources"),
        framework_root.join("adapters").join("targets"),
    ];
    let scan_files = [
        framework_root.join("README.md"),
        framework_root.join("AGENTS.md"),
        framework_root.join("rfcs").join("roadmap.md"),
        framework_root.join(".cursor").join("rules").join("project.mdc"),
    ];

    let skill_token_re = Regex::new(r"/[a-z][a-z0-9-]*:[a-z][a-z0-9-]*").expect("skill token regex");
    let flag_token_re = Regex::new(r"--[a-z][a-z0-9-]*").expect("flag token regex");
    let cli_command_re =
        Regex::new(r"\b(specify|cargo|gh|git|deno|npm|pnpm|yarn)\s").expect("cli command regex");
    let markdown_link_re = Regex::new(r"\]\([^)]+\)").expect("markdown link regex");

    let mut targets = collect_walk_targets(&scan_roots, &["md", "mdc"], framework_root)?;
    for path in scan_files {
        if path.is_file() {
            targets.push(path);
        }
    }
    targets.sort();
    targets.dedup();

    let mut findings = Vec::new();
    for path in targets {
        let rel = path
            .strip_prefix(framework_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let content = fs::read_to_string(&path).map_err(ToolingError::from)?;
        let lines: Vec<&str> = content.lines().collect();

        let mut line_idx = 0;
        while line_idx < lines.len() {
            let start_line = line_idx;
            let mut logical = lines[line_idx].to_string();
            let mut end = line_idx;

            for next_idx in (line_idx + 1)..lines.len().min(line_idx + 8) {
                let previous_continues = logical.trim_end().ends_with('\\');
                let next_is_indented = lines[next_idx]
                    .chars()
                    .next()
                    .is_some_and(|ch| ch == ' ' || ch == '\t');
                if !previous_continues && !next_is_indented {
                    break;
                }
                logical.push('\n');
                logical.push_str(lines[next_idx]);
                end = next_idx;
                if !lines[next_idx].trim_end().ends_with('\\') && !next_is_indented {
                    break;
                }
            }

            let scan_logical = markdown_link_re.replace_all(&logical, "]");
            if let Some(skill_match) = skill_token_re.find(&scan_logical) {
                let after_skill = &scan_logical[skill_match.end()..];
                if let Some(flag_match) = flag_token_re.find(after_skill) {
                    let between = &after_skill[..flag_match.start()];
                    if !cli_command_re.is_match(between) {
                        let line_suffix = if end > start_line {
                            format!("-{}", end + 1)
                        } else {
                            String::new()
                        };
                        findings.push(Finding {
                            rule_id: RULE_INVOCATION_POSITIONAL,
                            message: format!(
                                "Slash skill invocation uses flag-style arguments in {rel}:{start}{line_suffix} — use positional skill arguments; reserve --flags for underlying CLI commands",
                                start = start_line + 1,
                                line_suffix = line_suffix,
                            ),
                            location: Some(Location {
                                path: path.clone(),
                                line: start_line + 1,
                                column: None,
                            }),
                        });
                    }
                }
            }

            line_idx = end + 1;
        }
    }

    Ok(findings)
}

fn forbidden_patterns() -> Vec<ForbiddenPattern> {
    vec![
        ForbiddenPattern {
            pattern: Regex::new(r"\.specify/changes/").expect("forbidden pattern"),
            fix: "use `.specify/slices/` for slice-local state",
        },
        ForbiddenPattern {
            pattern: Regex::new(r"\bspecify validate\b").expect("forbidden pattern"),
            fix: "use `specify slice validate`",
        },
        ForbiddenPattern {
            pattern: Regex::new(r"\bspecify merge\b").expect("forbidden pattern"),
            fix: "use `specify slice merge run`",
        },
        ForbiddenPattern {
            pattern: Regex::new(r"\bspecify change plan\b").expect("forbidden pattern"),
            fix: "use `specify plan`",
        },
        ForbiddenPattern {
            pattern: Regex::new(r"\bspecify change draft\b").expect("forbidden pattern"),
            fix: "use `/spec:plan` or `specify plan create`",
        },
        ForbiddenPattern {
            pattern: Regex::new(r"\b[Ii]nitiative\b").expect("forbidden pattern"),
            fix: "use `change` for the umbrella and `slice` for entries",
        },
    ]
}

fn collect_walk_targets(
    roots: &[PathBuf],
    extensions: &[&str],
    framework_root: &Path,
) -> Result<Vec<PathBuf>, ToolingError> {
    let mut out = Vec::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(root).follow_links(false).into_iter() {
            let entry = entry.map_err(|source| {
                ToolingError::Infrastructure(format!("walk {}: {source}", root.display()))
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.into_path();
            if !has_extension(&path, extensions) {
                continue;
            }
            if under_symlink(framework_root, &path)? {
                continue;
            }
            out.push(path);
        }
    }
    Ok(out)
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| extensions.contains(&ext))
}

fn infrastructure_finding(rule_id: &'static str, error: ToolingError) -> Finding {
    Finding {
        rule_id,
        message: error.to_string(),
        location: None,
    }
}
