//! Structural delta merge -- header-based section parsing and baseline application.
//!
//! Operates on well-defined markdown headers (`## ADDED/MODIFIED/REMOVED/RENAMED
//! Requirements` and `### Requirement: <name>`) without interpreting the markdown
//! content within sections.

use anyhow::{Result, bail};

/// A parsed delta section from a spec file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaSection {
    /// New requirements to append to the baseline.
    Added(Vec<RequirementBlock>),
    /// Existing requirements whose content should be replaced.
    Modified(Vec<RequirementBlock>),
    /// Existing requirements to remove from the baseline.
    Removed(Vec<RemovedBlock>),
    /// Existing requirements to rename (content preserved).
    Renamed(Vec<RenameBlock>),
}

/// A requirement block with its raw markdown content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementBlock {
    /// Requirement name (text after `### Requirement: `).
    pub name: String,
    /// Everything from the `### Requirement:` line through the end of the block.
    pub raw_content: String,
}

/// A removed requirement with metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedBlock {
    /// Requirement name to remove from the baseline.
    pub name: String,
    /// Full content of the REMOVED block (reason, migration, etc.).
    pub raw_content: String,
}

/// A renamed requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameBlock {
    /// Original requirement name in the baseline.
    pub from: String,
    /// New name to replace it with.
    pub to: String,
}

const REQUIREMENT_PREFIX: &str = "### Requirement: ";

/// Parse delta sections from a spec file's content.
///
/// Expects `## ADDED Requirements`, `## MODIFIED Requirements`,
/// `## REMOVED Requirements`, and/or `## RENAMED Requirements` headers.
/// Sections without matching headers are silently ignored.
///
/// # Errors
///
/// Returns an error if a `## RENAMED` block is missing `FROM:` or `TO:` lines.
pub fn parse_sections(content: &str) -> Result<Vec<DeltaSection>> {
    let mut sections = Vec::new();
    let h2_blocks = split_h2_sections(content);

    for (header, body) in &h2_blocks {
        let normalized = header.trim().to_ascii_uppercase();

        if normalized.contains("ADDED") {
            let blocks = extract_requirement_blocks(body);
            if !blocks.is_empty() {
                sections.push(DeltaSection::Added(blocks));
            }
        } else if normalized.contains("MODIFIED") {
            let blocks = extract_requirement_blocks(body);
            if !blocks.is_empty() {
                sections.push(DeltaSection::Modified(blocks));
            }
        } else if normalized.contains("REMOVED") {
            let blocks = extract_removed_blocks(body);
            if !blocks.is_empty() {
                sections.push(DeltaSection::Removed(blocks));
            }
        } else if normalized.contains("RENAMED") {
            let renames = extract_rename_blocks(body)?;
            if !renames.is_empty() {
                sections.push(DeltaSection::Renamed(renames));
            }
        }
    }

    Ok(sections)
}

/// Apply delta operations to a baseline spec, returning the updated content.
///
/// Operations are applied in order: renames first (so MODIFIED/REMOVED can
/// target the new name), then removals, modifications, and finally additions.
///
/// # Errors
///
/// Returns an error if a MODIFIED or REMOVED requirement references a name
/// not found in the baseline.
pub fn apply_to_baseline(baseline: &str, deltas: &[DeltaSection]) -> Result<String> {
    let mut blocks = extract_requirement_blocks(baseline);
    let mut preamble = extract_preamble(baseline);

    for section in deltas {
        match section {
            DeltaSection::Renamed(renames) => {
                for rename in renames {
                    let Some(block) = blocks.iter_mut().find(|b| b.name == rename.from) else {
                        bail!("RENAMED requirement '{}' not found in baseline", rename.from);
                    };
                    block.raw_content = block.raw_content.replacen(
                        &format!("{REQUIREMENT_PREFIX}{}", rename.from),
                        &format!("{REQUIREMENT_PREFIX}{}", rename.to),
                        1,
                    );
                    block.name.clone_from(&rename.to);
                }
            }
            DeltaSection::Removed(removed) => {
                for rem in removed {
                    let before_len = blocks.len();
                    blocks.retain(|b| b.name != rem.name);
                    if blocks.len() == before_len {
                        bail!("REMOVED requirement '{}' not found in baseline", rem.name);
                    }
                }
            }
            DeltaSection::Modified(modified) => {
                for m in modified {
                    let Some(block) = blocks.iter_mut().find(|b| b.name == m.name) else {
                        bail!("MODIFIED requirement '{}' not found in baseline", m.name);
                    };
                    block.raw_content.clone_from(&m.raw_content);
                }
            }
            DeltaSection::Added(added) => {
                blocks.extend(added.iter().cloned());
            }
        }
    }

    if preamble.is_empty() {
        preamble = String::new();
    } else if !preamble.ends_with('\n') {
        preamble.push('\n');
    }

    let body: String = blocks.iter().map(|b| b.raw_content.as_str()).collect::<Vec<_>>().join("\n");

    let mut result = preamble;
    result.push_str(&body);
    if !result.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}

/// Split content into `(header_text, body_text)` pairs at `## ` boundaries.
fn split_h2_sections(content: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current_header = String::new();
    let mut current_body = String::new();
    let mut in_section = false;

    for line in content.lines() {
        if let Some(header) = line.strip_prefix("## ") {
            if in_section {
                sections.push((current_header.clone(), current_body.clone()));
                current_body.clear();
            }
            current_header = header.to_string();
            in_section = true;
        } else if in_section {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }

    if in_section {
        sections.push((current_header, current_body));
    }

    sections
}

/// Extract text before the first `### Requirement:` or `## ` header.
fn extract_preamble(content: &str) -> String {
    let mut preamble = String::new();
    for line in content.lines() {
        if line.starts_with("## ") || line.starts_with(REQUIREMENT_PREFIX) {
            break;
        }
        preamble.push_str(line);
        preamble.push('\n');
    }
    preamble
}

/// Extract `### Requirement: <name>` blocks from a body of text.
fn extract_requirement_blocks(body: &str) -> Vec<RequirementBlock> {
    let mut blocks = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_content = String::new();

    for line in body.lines() {
        if let Some(name) = line.strip_prefix(REQUIREMENT_PREFIX) {
            if let Some(prev_name) = current_name.take() {
                blocks.push(RequirementBlock {
                    name: prev_name,
                    raw_content: current_content.clone(),
                });
                current_content.clear();
            }
            current_name = Some(name.trim().to_string());
            current_content.push_str(line);
            current_content.push('\n');
        } else if current_name.is_some() {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    if let Some(name) = current_name {
        blocks.push(RequirementBlock {
            name,
            raw_content: current_content,
        });
    }

    blocks
}

/// Extract removed requirement blocks (same structure but typed differently).
fn extract_removed_blocks(body: &str) -> Vec<RemovedBlock> {
    extract_requirement_blocks(body)
        .into_iter()
        .map(|b| RemovedBlock {
            name: b.name,
            raw_content: b.raw_content,
        })
        .collect()
}

/// Extract rename blocks from a RENAMED section.
///
/// Expected format within each `### Requirement:` block:
/// ```text
/// FROM: Old Name
/// TO: New Name
/// ```
fn extract_rename_blocks(body: &str) -> Result<Vec<RenameBlock>> {
    let mut renames = Vec::new();
    let mut from: Option<String> = None;
    let mut to: Option<String> = None;

    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("FROM:") {
            from = Some(val.trim().to_string());
        } else if let Some(val) = trimmed.strip_prefix("TO:") {
            to = Some(val.trim().to_string());
        }

        if let (Some(f), Some(t)) = (&from, &to) {
            renames.push(RenameBlock {
                from: f.clone(),
                to: t.clone(),
            });
            from = None;
            to = None;
        }
    }

    if from.is_some() || to.is_some() {
        bail!("incomplete RENAMED block: both FROM: and TO: lines are required");
    }

    Ok(renames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_added_section() {
        let content = "\
## ADDED Requirements

### Requirement: User can export data
The system SHALL allow users to export data.

#### Scenario: Successful export
- **WHEN** user clicks Export
- **THEN** CSV file is downloaded
";

        let sections = parse_sections(content).unwrap();
        assert_eq!(sections.len(), 1);

        let DeltaSection::Added(blocks) = &sections[0] else {
            panic!("expected Added section");
        };
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].name, "User can export data");
        assert!(blocks[0].raw_content.contains("SHALL"));
    }

    #[test]
    fn parse_multiple_sections() {
        let content = "\
## ADDED Requirements

### Requirement: New feature
Description here.

## MODIFIED Requirements

### Requirement: Existing feature
Updated description.

## REMOVED Requirements

### Requirement: Old feature
**Reason**: No longer needed
**Migration**: Use new feature instead
";

        let sections = parse_sections(content).unwrap();
        assert_eq!(sections.len(), 3);
        assert!(matches!(&sections[0], DeltaSection::Added(_)));
        assert!(matches!(&sections[1], DeltaSection::Modified(_)));
        assert!(matches!(&sections[2], DeltaSection::Removed(_)));
    }

    #[test]
    fn parse_renamed_section() {
        let content = "\
## RENAMED Requirements

FROM: Old Name
TO: New Name

FROM: Another Old
TO: Another New
";

        let sections = parse_sections(content).unwrap();
        assert_eq!(sections.len(), 1);

        let DeltaSection::Renamed(renames) = &sections[0] else {
            panic!("expected Renamed section");
        };
        assert_eq!(renames.len(), 2);
        assert_eq!(renames[0].from, "Old Name");
        assert_eq!(renames[0].to, "New Name");
        assert_eq!(renames[1].from, "Another Old");
        assert_eq!(renames[1].to, "Another New");
    }

    #[test]
    fn incomplete_rename_is_error() {
        let content = "\
## RENAMED Requirements

FROM: Only From
";
        let result = parse_sections(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("incomplete"));
    }

    #[test]
    fn apply_added_to_empty_baseline() {
        let baseline = "";
        let deltas = vec![DeltaSection::Added(vec![RequirementBlock {
            name: "New req".to_string(),
            raw_content: "### Requirement: New req\nThe system SHALL do X.\n".to_string(),
        }])];

        let result = apply_to_baseline(baseline, &deltas).unwrap();
        assert!(result.contains("### Requirement: New req"));
        assert!(result.contains("SHALL do X"));
    }

    #[test]
    fn apply_added_to_existing_baseline() {
        let baseline = "\
### Requirement: Existing
The system SHALL do A.
";
        let deltas = vec![DeltaSection::Added(vec![RequirementBlock {
            name: "New req".to_string(),
            raw_content: "### Requirement: New req\nThe system SHALL do B.\n".to_string(),
        }])];

        let result = apply_to_baseline(baseline, &deltas).unwrap();
        assert!(result.contains("Existing"));
        assert!(result.contains("New req"));
    }

    #[test]
    fn apply_modified() {
        let baseline = "\
### Requirement: User auth
The system SHALL authenticate via password.

#### Scenario: Login
- **WHEN** user submits credentials
- **THEN** session is created
";

        let deltas = vec![DeltaSection::Modified(vec![RequirementBlock {
            name: "User auth".to_string(),
            raw_content: "### Requirement: User auth\nThe system SHALL authenticate via OAuth.\n"
                .to_string(),
        }])];

        let result = apply_to_baseline(baseline, &deltas).unwrap();
        assert!(result.contains("OAuth"));
        assert!(!result.contains("password"));
    }

    #[test]
    fn apply_removed() {
        let baseline = "\
### Requirement: Keep me
Content A.

### Requirement: Remove me
Content B.

### Requirement: Also keep
Content C.
";

        let deltas = vec![DeltaSection::Removed(vec![RemovedBlock {
            name: "Remove me".to_string(),
            raw_content: String::new(),
        }])];

        let result = apply_to_baseline(baseline, &deltas).unwrap();
        assert!(result.contains("Keep me"));
        assert!(result.contains("Also keep"));
        assert!(!result.contains("Remove me"));
        assert!(!result.contains("Content B"));
    }

    #[test]
    fn apply_renamed() {
        let baseline = "\
### Requirement: Old name
Content here.
";

        let deltas = vec![DeltaSection::Renamed(vec![RenameBlock {
            from: "Old name".to_string(),
            to: "New name".to_string(),
        }])];

        let result = apply_to_baseline(baseline, &deltas).unwrap();
        assert!(result.contains("### Requirement: New name"));
        assert!(!result.contains("### Requirement: Old name"));
        assert!(result.contains("Content here"));
    }

    #[test]
    fn modified_missing_requirement_is_error() {
        let baseline = "### Requirement: Exists\nContent.\n";
        let deltas = vec![DeltaSection::Modified(vec![RequirementBlock {
            name: "Does not exist".to_string(),
            raw_content: "### Requirement: Does not exist\nNew content.\n".to_string(),
        }])];

        let result = apply_to_baseline(baseline, &deltas);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn removed_missing_requirement_is_error() {
        let baseline = "### Requirement: Exists\nContent.\n";
        let deltas = vec![DeltaSection::Removed(vec![RemovedBlock {
            name: "Ghost".to_string(),
            raw_content: String::new(),
        }])];

        apply_to_baseline(baseline, &deltas).unwrap_err();
    }

    #[test]
    fn empty_content_produces_no_sections() {
        let sections = parse_sections("").unwrap();
        assert!(sections.is_empty());
    }

    #[test]
    fn combined_operations() {
        let baseline = "\
### Requirement: Alpha
Alpha content.

### Requirement: Beta
Beta content.

### Requirement: Gamma
Gamma content.
";

        let deltas = vec![
            DeltaSection::Renamed(vec![RenameBlock {
                from: "Alpha".to_string(),
                to: "Alpha Prime".to_string(),
            }]),
            DeltaSection::Removed(vec![RemovedBlock {
                name: "Beta".to_string(),
                raw_content: String::new(),
            }]),
            DeltaSection::Modified(vec![RequirementBlock {
                name: "Gamma".to_string(),
                raw_content: "### Requirement: Gamma\nUpdated gamma.\n".to_string(),
            }]),
            DeltaSection::Added(vec![RequirementBlock {
                name: "Delta".to_string(),
                raw_content: "### Requirement: Delta\nNew delta.\n".to_string(),
            }]),
        ];

        let result = apply_to_baseline(baseline, &deltas).unwrap();
        assert!(result.contains("Alpha Prime"));
        assert!(!result.contains("### Requirement: Alpha\n"));
        assert!(!result.contains("Beta"));
        assert!(result.contains("Updated gamma"));
        assert!(result.contains("Delta"));
    }
}
