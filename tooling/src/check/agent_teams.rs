use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::context::Context;
use crate::finding::{Check, Finding, Location};

const CANONICAL_REL: &str = "docs/reference/review-team-protocol.md";
const RULE_NON_CANONICAL: &str = "agent-teams.non-canonical-overlay";
const RULE_MISSING_CANONICAL: &str = "agent-teams.missing-canonical";

/// Per-target `agent-teams.md` canonicalisation guard.
pub struct AgentTeamsCheck;

impl Check for AgentTeamsCheck {
    fn run(&self, ctx: &Context) -> Vec<Finding> {
        run(ctx)
    }
}

/// Run the agent-teams overlay check against `ctx`.
pub fn run(ctx: &Context) -> Vec<Finding> {
    check_overlays(ctx.framework_root())
}

fn check_overlays(root: &Path) -> Vec<Finding> {
    let canonical_path = root.join(CANONICAL_REL);
    let canonical_bytes = match fs::read(&canonical_path) {
        Ok(bytes) => bytes,
        Err(_) => {
            return vec![finding(
                RULE_MISSING_CANONICAL,
                format!(
                    "Agent-teams canonical: {CANONICAL_REL} is missing — cannot validate per-adapter copies"
                ),
                Some(canonical_path),
            )];
        }
    };
    let canonical_hash = sha256(&canonical_bytes);

    let targets_dir = root.join("adapters").join("targets");
    let targets = match fs::read_dir(&targets_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut findings = Vec::new();
    for entry in targets.flatten() {
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let ref_path = entry.path().join("references").join("agent-teams.md");
        let ref_rel = path_relative(root, &ref_path);

        let metadata = match fs::symlink_metadata(&ref_path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        if metadata.file_type().is_symlink() {
            match fs::canonicalize(&ref_path) {
                Ok(resolved) => {
                    let expected = match fs::canonicalize(&canonical_path) {
                        Ok(path) => path,
                        Err(source) => {
                            findings.push(finding(
                                RULE_MISSING_CANONICAL,
                                format!(
                                    "Agent-teams canonical: {CANONICAL_REL} is missing — cannot validate per-adapter copies ({source})"
                                ),
                                Some(canonical_path.clone()),
                            ));
                            break;
                        }
                    };
                    if resolved != expected {
                        findings.push(finding(
                            RULE_NON_CANONICAL,
                            format!(
                                "Agent-teams overlay: {ref_rel} — symlink resolves to '{}', expected '{CANONICAL_REL}'",
                                path_relative(root, &resolved)
                            ),
                            Some(ref_path),
                        ));
                    }
                }
                Err(_) => {
                    findings.push(finding(
                        RULE_NON_CANONICAL,
                        format!("Agent-teams overlay: {ref_rel} — symlink does not resolve"),
                        Some(ref_path),
                    ));
                }
            }
            continue;
        }

        if metadata.is_file() {
            let local_bytes = match fs::read(&ref_path) {
                Ok(bytes) => bytes,
                Err(source) => {
                    findings.push(finding(
                        RULE_NON_CANONICAL,
                        format!("Agent-teams overlay: {ref_rel} — cannot read file: {source}"),
                        Some(ref_path),
                    ));
                    continue;
                }
            };
            if sha256(&local_bytes) != canonical_hash {
                findings.push(finding(
                    RULE_NON_CANONICAL,
                    format!(
                        "Agent-teams overlay: {ref_rel} — content drifted from canonical '{CANONICAL_REL}' (replace with a symlink or re-sync the file)"
                    ),
                    Some(ref_path),
                ));
            }
            continue;
        }

        findings.push(finding(
            RULE_NON_CANONICAL,
            format!(
                "Agent-teams overlay: {ref_rel} — must be a regular file or symlink, found unsupported entry type"
            ),
            Some(ref_path),
        ));
    }

    findings
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn path_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|rel| rel.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.display().to_string())
}

fn finding(rule_id: &'static str, message: String, path: Option<PathBuf>) -> Finding {
    Finding {
        rule_id,
        message,
        location: path.map(|path| Location {
            path,
            line: 1,
            column: None,
        }),
    }
}
