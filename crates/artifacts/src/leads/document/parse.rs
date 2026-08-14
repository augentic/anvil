//! Line-oriented `leads.md` parser.
//!
//! Catalog-only: optional `## Lead inventory` heading, then `###`
//! blocks. Prefix or suffix prose is a parse failure.

use error::{Error, Result};

use super::Leads;
use crate::leads::lead::Lead;

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

    pub fn run(mut self) -> Result<Leads> {
        self.skip_blank();
        if self.cursor < self.lines.len() && is_inventory_heading(self.lines[self.cursor]) {
            self.cursor += 1;
            self.skip_blank();
        } else if self.cursor < self.lines.len() && !is_lead_heading(self.lines[self.cursor]) {
            return Err(parse_err(
                "leads.md is catalog-only: start with `## Lead inventory` or a `###` lead block"
                    .into(),
            ));
        }
        let leads = self.parse_leads()?;
        self.skip_blank();
        if self.cursor < self.lines.len() {
            return Err(parse_err(
                "leads.md is catalog-only: trailing prose after the lead inventory is refused"
                    .into(),
            ));
        }
        Ok(Leads { leads })
    }

    fn skip_blank(&mut self) {
        while self.cursor < self.lines.len() {
            let trimmed = strip_newline(self.lines[self.cursor]).trim();
            if !trimmed.is_empty() {
                break;
            }
            self.cursor += 1;
        }
    }

    fn parse_leads(&mut self) -> Result<Vec<Lead>> {
        let mut out: Vec<Lead> = Vec::new();
        while self.cursor < self.lines.len() {
            let line = self.lines[self.cursor];
            if is_top_level_heading(line) {
                break;
            }
            if is_lead_heading(line) {
                out.push(self.parse_lead_block()?);
                continue;
            }
            let trimmed = strip_newline(line).trim();
            if trimmed.is_empty() {
                self.cursor += 1;
                continue;
            }
            return Err(parse_err(format!(
                "leads.md is catalog-only: unexpected line `{trimmed}`"
            )));
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
        let mut parent: Option<String> = None;
        let mut focus: Option<String> = None;

        while self.cursor < self.lines.len() {
            let raw = self.lines[self.cursor];
            if is_lead_heading(raw) || is_top_level_heading(raw) {
                break;
            }
            let trimmed = strip_newline(raw).trim_start();
            if trimmed.is_empty() {
                self.cursor += 1;
                continue;
            }
            let Some(bullet_body) = bullet_body(trimmed) else {
                return Err(parse_err(format!(
                    "lead `{heading_label}`: unexpected line `{trimmed}`"
                )));
            };
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
                "parent" => {
                    parent = empty_to_none(value);
                }
                "focus" => {
                    focus = empty_to_none(value);
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
            parent,
            focus,
        })
    }
}

fn empty_to_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

/// Parse an inline `topics:` bullet value into kebab slugs.
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
        code: "leads-parse-failed",
        detail,
    }
}
