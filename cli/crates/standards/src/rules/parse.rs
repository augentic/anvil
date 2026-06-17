//! Rule frontmatter parser per the rules contract §"Codex file shape".
//!
//! Splits a rule markdown file into YAML frontmatter +
//! verbatim body, validates the frontmatter against the embedded
//! `schemas/rules/rule.schema.json`, lifts the `snake_case`
//! authoring keys (`lint_mode`, `rule_hints`,
//! `replaced_by`) to the kebab-case wire shape carried by
//! [`Rule`], and returns the typed rule with `body` set to the
//! exact post-delimiter bytes.
//!
//! # Body fidelity
//!
//! Reviewing agents consume the body verbatim as policy text, so
//! the parser preserves every newline, heading, and code fence
//! after the closing `---` delimiter. The only normalisation is
//! stripping the single newline (`\n` or `\r\n`) that immediately
//! follows the closing delimiter — without that, a file ending
//! `...---\n## Rule\n...` would yield a body starting with
//! `\n## Rule\n` instead of `## Rule\n`.
//!
//! # Schema vs wire shape
//!
//! The canonical schema at `schemas/rules/rule.schema.json` is
//! the single source of truth for the runtime resolver (no vendored
//! codex-rule schema copy exists). It carries `snake_case` keys.
//! Validation runs against the original raw frontmatter, *before* the
//! `snake_case -> kebab-case` lift, which keeps schema semantics
//! aligned with what authors actually wrote on disk. The lift then
//! rewrites keys at every nesting level so the value can deserialize
//! cleanly into [`Rule`].
//!
//! # Out of scope
//!
//! No regex compilation. The rule-hints contract
//! extensibility" requires the runtime resolver to never compile a
//! regex it never executes — hint execution belongs to `specify lint`
//! (CH-13 +). Applicability filtering, deprecation filtering, and
//! stable ordering are CH-13 / CH-14.

use std::path::Path;
use std::{fs, io};

use serde_json::{Map as JsonMap, Value as JsonValue};
use specify_schema::{RULE_JSON_SCHEMA, ValidationStatus, validate_value};

use super::Rule;

/// Canonical codex-rule frontmatter schema, sourced from
/// [`specify_schema::RULE_JSON_SCHEMA`]. Per the standards-layer contract
/// §"Eliminates the vendored codex-rule schema", that constant is the
/// single embedded source of truth.
const RULE_SCHEMA: &str = RULE_JSON_SCHEMA;

/// Failure modes for [`parse_rule`] / [`parse_rule_file`].
///
/// Mirrors the staging order of the parser: open delimiter first,
/// closing delimiter, YAML parse, schema validation, JSON
/// conversion. I/O failures from the file convenience wrapper
/// surface as [`ParseError::Io`].
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// File does not start with `---\n` or `---\r\n`.
    #[error("rule missing leading YAML frontmatter delimiter '---'")]
    MissingOpeningDelimiter,
    /// No matching `---` line is found before EOF.
    #[error("rule missing closing YAML frontmatter delimiter '---'")]
    MissingClosingDelimiter,
    /// `serde_saphyr` could not parse the frontmatter block as YAML.
    #[error("rule frontmatter YAML parse failed: {0}")]
    Yaml(String),
    /// Frontmatter parsed but failed `rule.schema.json`.
    #[error("rule frontmatter schema validation failed: {0}")]
    Schema(String),
    /// Post-lift JSON did not deserialize into [`Rule`].
    #[error("rule frontmatter JSON conversion failed: {0}")]
    JsonConvert(String),
    /// File read failed (file convenience wrapper only).
    #[error("rule file read failed: {0}")]
    Io(#[from] io::Error),
}

/// Parse a rule markdown document into a [`Rule`].
///
/// The body field is set to the verbatim post-delimiter bytes; the
/// rest of the struct comes from the validated, kebab-lifted YAML
/// frontmatter. See module docs for delimiter handling, lift
/// behavior, and out-of-scope items.
///
/// # Errors
///
/// Returns the matching [`ParseError`] variant for each parsing
/// stage; see the variant docs.
pub fn parse_rule(content: &str) -> Result<Rule, ParseError> {
    let (frontmatter, body) = split_frontmatter(content)?;

    let raw_value: JsonValue =
        serde_saphyr::from_str(frontmatter).map_err(|err| ParseError::Yaml(err.to_string()))?;

    let summaries = validate_value(
        &raw_value,
        RULE_SCHEMA,
        "rule-schema",
        "rule frontmatter conforms to schemas/rules/rule.schema.json",
    );
    let failures: Vec<String> = summaries
        .into_iter()
        .filter(|summary| summary.status == ValidationStatus::Fail)
        .filter_map(|summary| summary.detail)
        .collect();
    if !failures.is_empty() {
        return Err(ParseError::Schema(failures.join("; ")));
    }

    let lifted = snake_to_kebab_keys(raw_value);
    let mut rule: Rule =
        serde_json::from_value(lifted).map_err(|err| ParseError::JsonConvert(err.to_string()))?;
    body.clone_into(&mut rule.body);
    Ok(rule)
}

/// Read `path` and parse it via [`parse_rule`].
///
/// The body is preserved exactly as on disk; the parser does not
/// line-normalize. I/O failures map to [`ParseError::Io`]; every
/// other failure mode is delegated to [`parse_rule`].
///
/// # Errors
///
/// Returns [`ParseError::Io`] if the file cannot be read; otherwise
/// the error from [`parse_rule`].
pub fn parse_rule_file(path: &Path) -> Result<Rule, ParseError> {
    let content = fs::read_to_string(path)?;
    parse_rule(&content)
}

/// Split `content` into `(frontmatter, body)` slices.
///
/// Accepts `---\n` or `---\r\n` for the opening delimiter and
/// `\n---\n`, `\n---\r\n`, or trailing `\n---` at EOF for the
/// closing delimiter. The single newline that introduces the body
/// (the one immediately after the closing `---`) is stripped so
/// callers see body content starting at column 0; everything else
/// is verbatim.
///
/// A sibling `Option`-returning copy lives at `specify-model`'s
/// `decision::split_frontmatter`; the `specify-standards` ⊥
/// `specify-model` dependency-direction invariant blocks one shared impl.
fn split_frontmatter(content: &str) -> Result<(&str, &str), ParseError> {
    let rest = if let Some(rest) = content.strip_prefix("---\n") {
        rest
    } else if let Some(rest) = content.strip_prefix("---\r\n") {
        rest
    } else {
        return Err(ParseError::MissingOpeningDelimiter);
    };

    let mut search_from = 0;
    while let Some(rel) = rest[search_from..].find("\n---") {
        let pos = search_from + rel;
        let after = pos + "\n---".len();
        let tail = &rest[after..];
        if tail.is_empty() {
            return Ok((&rest[..pos], ""));
        }
        if let Some(body) = tail.strip_prefix('\n') {
            return Ok((&rest[..pos], body));
        }
        if let Some(body) = tail.strip_prefix("\r\n") {
            return Ok((&rest[..pos], body));
        }
        // `\n---` followed by other text (e.g. `\n---ignored`) is not
        // a delimiter line; advance and keep searching.
        search_from = after;
    }
    Err(ParseError::MissingClosingDelimiter)
}

/// Recursively rewrite every JSON object key from `snake_case` to
/// kebab-case.
///
/// Rule frontmatter keys never contain `_` for any reason
/// other than the `snake_case` authoring convention (`lint_mode`,
/// `rule_hints`, `replaced_by`), so a blind `_` -> `-`
/// rewrite is safe. String VALUES (e.g. adapter names like
/// `typescript`) are untouched — only keys are transformed.
fn snake_to_kebab_keys(value: JsonValue) -> JsonValue {
    match value {
        JsonValue::Object(map) => {
            let mut out = JsonMap::with_capacity(map.len());
            for (k, v) in map {
                out.insert(k.replace('_', "-"), snake_to_kebab_keys(v));
            }
            JsonValue::Object(out)
        }
        JsonValue::Array(items) => {
            JsonValue::Array(items.into_iter().map(snake_to_kebab_keys).collect())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests;
