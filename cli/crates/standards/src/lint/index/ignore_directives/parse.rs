//! Comment-aware grammar parser for the `specify-ignore` extractor.
//!
//! Owns the string-aware comment scan, the body slice, and the
//! `specify-ignore: <RULE-ID> <SEPARATOR> <rationale>` grammar. The
//! parent extractor consumes [`parse_line`] and lays target-line
//! semantics on top.

use super::Family;

/// Per-line parse result. `is_trailing` flips when non-whitespace
/// precedes the comment delimiter on the same line.
pub(super) struct Parsed {
    pub rule_id: String,
    pub rationale: Option<String>,
    pub raw: String,
    pub is_trailing: bool,
}

/// Parse a single line. Returns `None` when the line carries no
/// directive (either no recognised comment, or the comment body
/// does not start with `specify-ignore:`).
pub(super) fn parse_line(line: &str, family: Family) -> Option<Parsed> {
    let (pos, delim) = find_comment_start(line, family)?;
    let raw = raw_comment(line, pos, delim);
    let body = comment_body(&raw, delim);
    let (rule_id, rationale) = parse_directive(body.trim())?;
    let is_trailing = line[..pos].chars().any(|c| !c.is_whitespace());
    Some(Parsed {
        rule_id,
        rationale,
        raw,
        is_trailing,
    })
}

/// Locate the first comment delimiter on `line` that is not inside a
/// string literal. String tracking is intentionally minimal — it
/// recognises `"…"` and `'…'` with backslash escapes for C / Hash /
/// `SqlLua` families. HTML scanning has no string semantics in
/// markdown source.
fn find_comment_start(line: &str, family: Family) -> Option<(usize, &'static str)> {
    let bytes = line.as_bytes();
    match family {
        Family::C => scan_with_strings(bytes, |b, i| {
            if b[i] == b'/' && i + 1 < b.len() {
                if b[i + 1] == b'/' {
                    return Some("//");
                }
                if b[i + 1] == b'*' {
                    return Some("/*");
                }
            }
            None
        }),
        Family::Hash => scan_with_strings(bytes, |b, i| (b[i] == b'#').then_some("#")),
        Family::Html => bytes.windows(4).position(|w| w == b"<!--").map(|pos| (pos, "<!--")),
        Family::SqlLua => scan_with_strings(bytes, |b, i| {
            (b[i] == b'-' && i + 1 < b.len() && b[i + 1] == b'-').then_some("--")
        }),
    }
}

/// Shared string-aware scan helper. `match_delim` returns the static
/// delimiter string when bytes at index `i` open a comment.
fn scan_with_strings(
    bytes: &[u8], match_delim: impl Fn(&[u8], usize) -> Option<&'static str>,
) -> Option<(usize, &'static str)> {
    let mut i = 0;
    let mut in_string: Option<u8> = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(quote) = in_string {
            if c == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if c == quote {
                in_string = None;
            }
            i += 1;
            continue;
        }
        if c == b'"' || c == b'\'' {
            in_string = Some(c);
            i += 1;
            continue;
        }
        if let Some(delim) = match_delim(bytes, i) {
            return Some((i, delim));
        }
        i += 1;
    }
    None
}

/// Slice the comment text (including delimiters) out of `line`,
/// stopping at the block-comment closer where present. Single-line
/// delimiters (`//`, `#`, `--`) run to end-of-line.
fn raw_comment(line: &str, pos: usize, delim: &'static str) -> String {
    let from_delim = &line[pos..];
    let close = match delim {
        "/*" => Some("*/"),
        "<!--" => Some("-->"),
        _ => None,
    };
    if let Some(close) = close
        && let Some(rel) = from_delim[delim.len()..].find(close)
    {
        return from_delim[..delim.len() + rel + close.len()].to_owned();
    }
    from_delim.to_owned()
}

/// Strip leading/trailing delimiters from `raw` so the parser sees
/// only the directive body.
fn comment_body<'a>(raw: &'a str, delim: &'static str) -> &'a str {
    let stripped = raw.strip_prefix(delim).unwrap_or(raw);
    match delim {
        "/*" => stripped.strip_suffix("*/").unwrap_or(stripped),
        "<!--" => stripped.strip_suffix("-->").unwrap_or(stripped),
        _ => stripped,
    }
}

/// Parse a trimmed directive body into `(rule_id, rationale?)`.
/// Returns `None` when the body does not start with the
/// `specify-ignore:` marker, the rule-id token is missing, or the
/// rule-id token does not match the wire grammar
/// (`[A-Z][A-Z0-9]*-[0-9]+`). The grammar check keeps documentation
/// of the directive syntax — `<!-- specify-ignore: … -->` placeholders
/// in markdown tables, fenced code blocks, etc. — from being mistaken
/// for live directives.
///
/// Separator handling per D3: both the em-dash `—` and the two-char
/// `--` sequence are accepted. When no separator follows the rule-id
/// the directive is treated as malformed (rationale = `None`); the
/// rationale is captured raw when present regardless of length so
/// the validation pass owns the 16-character check (D12).
fn parse_directive(body: &str) -> Option<(String, Option<String>)> {
    let rest = body.strip_prefix("specify-ignore:")?.trim_start();
    if rest.is_empty() {
        return None;
    }
    let (rule_id, after) = rest.find(char::is_whitespace).map_or_else(
        || (rest.to_owned(), ""),
        |idx| (rest[..idx].to_owned(), rest[idx..].trim_start()),
    );
    if !is_well_formed_rule_id(&rule_id) {
        return None;
    }
    if after.is_empty() {
        return Some((rule_id, None));
    }
    let after_sep = if let Some(rest) = after.strip_prefix('—') {
        rest.trim_start()
    } else if let Some(rest) = after.strip_prefix("--") {
        rest.trim_start()
    } else {
        return Some((rule_id, None));
    };
    let rationale = after_sep.trim_end();
    if rationale.is_empty() {
        Some((rule_id, None))
    } else {
        Some((rule_id, Some(rationale.to_owned())))
    }
}

/// Returns `true` when `token` matches the wire grammar
/// `^[A-Z][A-Z0-9]*-[0-9]+$`. Used to reject directive bodies whose
/// rule-id slot is documentation filler (`…`, `<RULE-ID>`, etc.)
/// rather than a real codex id.
fn is_well_formed_rule_id(token: &str) -> bool {
    let Some((prefix, suffix)) = token.split_once('-') else {
        return false;
    };
    if prefix.is_empty() || suffix.is_empty() {
        return false;
    }
    let mut chars = prefix.chars();
    let first = chars.next().expect("checked non-empty");
    if !first.is_ascii_uppercase() {
        return false;
    }
    if !chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
        return false;
    }
    suffix.chars().all(|c| c.is_ascii_digit())
}
