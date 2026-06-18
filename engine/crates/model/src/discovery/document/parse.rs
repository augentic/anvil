//! Line-oriented `discovery.md` parser.
//!
//! [`Parser`] walks the document line-by-line, preserving all prose
//! outside the `## Lead inventory` section verbatim and collecting each
//! `### <id>` block plus its bullet list as a [`Lead`]. The assembled
//! [`Discovery`] is returned to [`super::Discovery::parse`].

use specify_error::{Error, Result};

use super::Discovery;
use crate::discovery::lead::Lead;

pub struct Parser<'a> {
    lines: Vec<&'a str>,
    cursor: usize,
}

impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            lines: text.split_inclusive('\n').collect(),
            cursor: 0,
        }
    }

    pub fn run(mut self) -> Result<Discovery> {
        let prefix = self.consume_until_inventory();
        let has_inventory_heading = self.cursor < self.lines.len();
        let leads = if has_inventory_heading {
            self.cursor += 1;
            self.parse_leads()?
        } else {
            Vec::new()
        };
        let suffix = self.lines[self.cursor..].concat();
        Ok(Discovery {
            prefix,
            leads,
            suffix,
            has_inventory_heading,
        })
    }

    fn consume_until_inventory(&mut self) -> String {
        let mut out = String::new();
        while self.cursor < self.lines.len() {
            let line = self.lines[self.cursor];
            if is_inventory_heading(line) {
                return out;
            }
            out.push_str(line);
            self.cursor += 1;
        }
        out
    }

    fn parse_leads(&mut self) -> Result<Vec<Lead>> {
        let mut out: Vec<Lead> = Vec::new();
        while self.cursor < self.lines.len() {
            let line = self.lines[self.cursor];
            if is_top_level_heading(line) {
                break;
            }
            if is_lead_heading(line) {
                let lead = self.parse_lead_block()?;
                out.push(lead);
                continue;
            }
            self.cursor += 1;
        }
        Ok(out)
    }

    fn parse_lead_block(&mut self) -> Result<Lead> {
        let heading = self.lines[self.cursor];
        let heading_label = lead_heading_id(heading).unwrap_or("").trim().to_string();
        self.cursor += 1;

        let mut lead: Option<String> = None;
        let mut source: Option<String> = None;
        let mut synopsis: Option<String> = None;
        let mut topics: Vec<String> = Vec::new();

        while self.cursor < self.lines.len() {
            let raw = self.lines[self.cursor];
            if is_lead_heading(raw) || is_top_level_heading(raw) {
                break;
            }
            let trimmed = strip_newline(raw).trim_start();
            if let Some(bullet_body) = bullet_body(trimmed) {
                let (key, value) = split_bullet(bullet_body)?;
                match key {
                    "lead" => {
                        if lead.is_some() {
                            return Err(parse_err(format!(
                                "lead `{heading_label}`: duplicate `lead:` bullet"
                            )));
                        }
                        lead = Some(value.to_string());
                    }
                    "source" => {
                        source = Some(value.to_string());
                    }
                    "synopsis" => {
                        synopsis = Some(value.to_string());
                    }
                    "topics" => {
                        topics = parse_topics(value);
                    }
                    "aliases" => {
                        return Err(parse_err(format!(
                            "lead `{heading_label}`: `aliases:` is not supported; remove the bullet \
                             and use the canonical `lead` id in plan bindings"
                        )));
                    }
                    other => {
                        return Err(parse_err(format!(
                            "lead `{heading_label}`: unknown bullet `{other}`"
                        )));
                    }
                }
            }
            self.cursor += 1;
        }

        let lead = lead.ok_or_else(|| {
            parse_err(format!("lead `{heading_label}` is missing the `lead:` bullet"))
        })?;
        let source = source.unwrap_or_default();
        let synopsis = synopsis
            .ok_or_else(|| parse_err(format!("lead `{lead}` is missing the `synopsis:` bullet")))?;
        Ok(Lead {
            lead,
            source,
            synopsis,
            topics,
        })
    }
}

/// Parse an inline `topics:` bullet value into kebab slugs.
///
/// Accepts the flow-sequence form `[a, b, c]` (matching the lead schema
/// example) as well as a bare comma-separated list; brackets are
/// stripped, entries are trimmed, and empties are dropped. Slug shape is
/// enforced downstream by schema validation, not here.
fn parse_topics(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub fn is_inventory_heading(line: &str) -> bool {
    let trimmed = strip_newline(line).trim();
    trimmed.eq_ignore_ascii_case("## Lead inventory")
}

fn is_top_level_heading(line: &str) -> bool {
    let trimmed = strip_newline(line);
    trimmed.starts_with("## ") && !is_inventory_heading(line)
}

fn is_lead_heading(line: &str) -> bool {
    let trimmed = strip_newline(line);
    trimmed.starts_with("### ")
}

fn lead_heading_id(line: &str) -> Option<&str> {
    let trimmed = strip_newline(line);
    trimmed.strip_prefix("### ")
}

fn strip_newline(line: &str) -> &str {
    line.strip_suffix('\n').map_or(line, |s| s.strip_suffix('\r').unwrap_or(s))
}

fn bullet_body(trimmed: &str) -> Option<&str> {
    trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* "))
}

fn split_bullet(body: &str) -> Result<(&str, &str)> {
    let (key, value) = body
        .split_once(':')
        .ok_or_else(|| parse_err(format!("bullet `{body}` must use `key: value` form")))?;
    Ok((key.trim(), value.trim()))
}

const fn parse_err(detail: String) -> Error {
    Error::Diag {
        code: "discovery-parse-failed",
        detail,
    }
}
