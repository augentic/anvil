//! `kind: cli-contract` evaluator.
//!
//! Checks documentation against the binary-injected [`CliContract`]
//! so cited verbs, flags, journal event ids, and error discriminants
//! cannot drift from the binary. Three `value` mechanism selectors:
//!
//! - `invocations` — every `specify …` command line found in fenced
//!   blocks whose info string is one of `config.langs`, and in inline
//!   code spans, walks the contract's verb tree; unknown subcommands
//!   and undeclared `--flags` are flagged. Shell comments end the
//!   scan of a line, and a brace-expansion shorthand token
//!   (`{create, amend}`) ends the walk — the expansion is ambiguous,
//!   so nothing after it is checked.
//! - `event-ids` — dotted-kebab inline code spans plus
//!   `"<field>": "…"` values in fenced bodies (fields from
//!   `config.json-fields`) must be ids in `journal-event-ids`.
//!   Candidates are gated by the contract's own event-id namespace:
//!   only tokens whose first dotted segment matches a declared
//!   family (`plan.…`, `slice.…`, …) are membership-checked, so
//!   `go.mod` or `users.register` never flag.
//! - `error-codes` — `"<field>": "…"` values in fenced bodies must be
//!   ids in `error-ids`.
//! - `test-citations` — `tests/….rs` inline code spans plus
//!   `tests/…` paths behind the configured `link-prefixes` must
//!   exist in the contract's build-time `tests` inventory (a cited
//!   directory must contain at least one inventoried file). A
//!   contract with an empty inventory disables the selector.
//!
//! The fence language set and every exclusion (`ignore`,
//! `ignore-suffixes`, `allow-prefixes`) are policy supplied by the
//! rule file, never constants in this arm. The contract itself is
//! injected by the root binary — `specify contract dump` and the lint
//! surfaces share one builder — preserving the standards⊥workflow
//! layering: this crate sees only the [`CliContract`] DTO, never
//! `clap` or `specify-workflow` types.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};

use super::{HintError, make_finding};
use crate::lint::WorkspaceModel;
use crate::lint::contract::{CliContract, CommandNode};
use crate::rules::{HintKind, ResolvedRule, RuleHint};

const SOURCE_INVOCATIONS: &str = "invocations";
const SOURCE_EVENT_IDS: &str = "event-ids";
const SOURCE_ERROR_CODES: &str = "error-codes";
const SOURCE_TEST_CITATIONS: &str = "test-citations";

/// Parsed `invocations` hint configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct InvocationsConfig {
    /// Fence info strings whose bodies are scanned for command lines
    /// (e.g. `bash`, `sh`, `console`).
    langs: Vec<String>,
    /// Offending tokens (verbs or flags) exempted by the rule.
    #[serde(default)]
    ignore: Vec<String>,
}

/// Parsed `event-ids` / `error-codes` hint configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct IdsConfig {
    /// JSON field names whose string values are membership-checked in
    /// fenced bodies (e.g. `event` for journal lines, `code` for
    /// error envelopes).
    json_fields: Vec<String>,
    /// Exact tokens exempted by the rule (e.g. dotted YAML paths that
    /// are not event ids).
    #[serde(default)]
    ignore: Vec<String>,
    /// Suffix exemptions for the inline-span scan (e.g. `.yaml`,
    /// `.md` — dotted tokens that are file names, not ids).
    #[serde(default)]
    ignore_suffixes: Vec<String>,
    /// Prefix exemptions for dynamically composed id families.
    #[serde(default)]
    allow_prefixes: Vec<String>,
}

impl IdsConfig {
    fn permits(&self, token: &str) -> bool {
        self.ignore.iter().any(|t| t == token)
            || self.allow_prefixes.iter().any(|p| token.starts_with(p.as_str()))
    }
}

/// Parsed `test-citations` hint configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct TestCitationsConfig {
    /// Link-target prefixes that root a citation of the CLI repo's
    /// `tests/` tree (e.g. a `blob/main/` GitHub URL prefix).
    link_prefixes: Vec<String>,
    /// Exact cited paths exempted by the rule (e.g. downstream
    /// generated-project test layouts that are not CLI tests).
    #[serde(default)]
    ignore: Vec<String>,
}

fn parse_config<T: serde::de::DeserializeOwned>(
    rule: &ResolvedRule, hint: &RuleHint, requirement: &'static str,
) -> Result<T, HintError> {
    let raw = hint.config.as_ref().ok_or_else(|| HintError::Unsupported {
        rule_id: rule.rule_id.clone(),
        kind: HintKind::CliContract,
        reason: requirement,
    })?;
    serde_json::from_value(raw.clone()).map_err(|_ignored| HintError::Unsupported {
        rule_id: rule.rule_id.clone(),
        kind: HintKind::CliContract,
        reason: "invalid cli-contract hint config JSON",
    })
}

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, candidates: &[PathBuf], project_dir: &Path,
    model: &WorkspaceModel, contract: Option<&CliContract>, next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let Some(contract) = contract else {
        return Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::CliContract,
            reason: "no CLI contract injected; `cli-contract` hints run only under the \
                     `specify` binary",
        });
    };
    match hint.value.trim() {
        SOURCE_INVOCATIONS => {
            let cfg: InvocationsConfig =
                parse_config(rule, hint, "`invocations` requires a `config: { langs }`")?;
            invocations(rule, candidates, project_dir, model, contract, &cfg, next_id)
        }
        SOURCE_EVENT_IDS => {
            let cfg: IdsConfig =
                parse_config(rule, hint, "`event-ids` requires a `config: { json-fields }`")?;
            event_ids(rule, candidates, project_dir, model, contract, &cfg, next_id)
        }
        SOURCE_ERROR_CODES => {
            let cfg: IdsConfig =
                parse_config(rule, hint, "`error-codes` requires a `config: { json-fields }`")?;
            let probe = IdProbe {
                cfg: &cfg,
                known: &contract.error_ids,
                shape: IdShape::Kebab,
                label: "error id",
                families: None,
            };
            Ok(json_field_findings(rule, candidates, model, &probe, next_id))
        }
        SOURCE_TEST_CITATIONS => {
            let cfg: TestCitationsConfig = parse_config(
                rule,
                hint,
                "`test-citations` requires a `config: { link-prefixes }`",
            )?;
            test_citations(rule, candidates, project_dir, contract, &cfg, next_id)
        }
        _ => Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::CliContract,
            reason: "unknown cli-contract source discriminator",
        }),
    }
}

// --- invocations -----------------------------------------------------

/// One offending token found while walking an invocation.
struct Issue {
    kind: IssueKind,
    token: String,
    /// The resolved verb path up to the offending token
    /// (e.g. `specify plan`).
    context: String,
}

enum IssueKind {
    Verb,
    Flag,
}

fn invocations(
    rule: &ResolvedRule, candidates: &[PathBuf], project_dir: &Path, model: &WorkspaceModel,
    contract: &CliContract, cfg: &InvocationsConfig, next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let candidate_set = super::candidate_set(candidates);
    let mut findings = Vec::new();

    for block in &model.fenced_blocks {
        if !candidate_set.contains(&block.path) {
            continue;
        }
        if !cfg.langs.iter().any(|lang| lang == &block.lang) {
            continue;
        }
        for (offset, line) in logical_lines(&block.body) {
            let file_line = block.line_start.saturating_add(offset);
            check_command_line(
                rule,
                &line,
                &block.path,
                file_line,
                contract,
                cfg,
                &mut findings,
                next_id,
            );
        }
    }

    for (path, text) in markdown_texts(candidates, project_dir)? {
        for (line, span) in inline_spans(&text) {
            if let Some(rest) = span.strip_prefix("specify ") {
                let command = format!("specify {rest}");
                check_command_line(
                    rule,
                    &command,
                    &path,
                    line,
                    contract,
                    cfg,
                    &mut findings,
                    next_id,
                );
            }
        }
    }

    Ok(findings)
}

/// Scan one command line for `specify` occurrences and walk each
/// against the contract's verb tree, minting a finding per issue.
#[expect(
    clippy::too_many_arguments,
    reason = "shared by the fenced and inline scans; bundling the five borrowed locals into a \
              struct would only relocate the argument list"
)]
fn check_command_line(
    rule: &ResolvedRule, line: &str, path: &str, file_line: u32, contract: &CliContract,
    cfg: &InvocationsConfig, findings: &mut Vec<Diagnostic>, next_id: &mut u64,
) {
    let tokens = shell_tokens(line);
    for start in 0..tokens.len() {
        if !is_specify_word(&tokens[start].text) || tokens[start].terminal {
            continue;
        }
        let Some(issue) = walk_invocation(&tokens[start + 1..], &contract.commands) else {
            continue;
        };
        if cfg.ignore.iter().any(|t| t == &issue.token) {
            continue;
        }
        let (what, hint_text) = match issue.kind {
            IssueKind::Verb => ("verb", "not a subcommand of"),
            IssueKind::Flag => ("flag", "not a flag accepted by"),
        };
        findings.push(make_finding(
            rule,
            *next_id,
            format!(
                "Unknown `specify` {what}: {path}:{file_line} — `{}` is {hint_text} `{}`",
                issue.token, issue.context,
            ),
            Some(FindingLocation {
                path: path.to_string(),
                line: Some(file_line),
                column: None,
                end_line: None,
                end_column: None,
            }),
            FindingEvidence::Structured {
                summary: format!("`{}` is {hint_text} `{}`", issue.token, issue.context),
                data: serde_json::json!({
                    "path": path,
                    "line": file_line,
                    "token": issue.token,
                    "context": issue.context,
                }),
                locations: None,
            },
        ));
        *next_id += 1;
    }
}

/// Walk the tokens after a `specify` occurrence down the contract's
/// verb tree.
///
/// Descent follows kebab tokens that name a subcommand of the current
/// node. A kebab token that is *not* a subcommand is an unknown verb
/// only when the current node still requires one (it has subcommands
/// and no positionals); otherwise it is a positional and descent
/// stops. `--flags` are checked against the union of the resolved
/// path's declared flags (global flags live on the root node); a
/// `--flag value` pair skips its value token. A literal `--` ends the
/// walk (everything after is passthrough), as does any shell
/// terminator (`|`, `&&`, `;`, redirection, comment).
fn walk_invocation(tokens: &[ShellToken], root: &CommandNode) -> Option<Issue> {
    let mut node = root;
    let mut allowed_flags: BTreeSet<&str> = flag_set(root);
    let mut context = root.name.clone();
    let mut descending = true;
    let mut idx = 0;

    while idx < tokens.len() {
        let token = &tokens[idx];
        let text = token.text.as_str();
        if text.is_empty() || is_terminator(text) || text == "--" {
            break;
        }
        // Brace-expansion shorthand (`specify plan {create, amend} …`)
        // makes the resolved verb ambiguous; stop checking the rest.
        if text.contains('{') {
            break;
        }
        if let Some(issue) = check_flag_token(text, &allowed_flags, &context) {
            return Some(issue);
        }
        if text.starts_with("--") {
            // Declared (or placeholder) flag: skip a `--flag value`
            // value token so it is not mistaken for a verb.
            if FLAG_RE.is_match(text.split('=').next().unwrap_or(text))
                && !text.contains('=')
                && !token.terminal
                && tokens.get(idx + 1).is_some_and(|next| !next.text.starts_with('-'))
            {
                idx += 1;
            }
        } else if descending && KEBAB_RE.is_match(text) {
            match node.subcommands.iter().find(|sub| sub.name == text) {
                Some(child) => {
                    allowed_flags.extend(flag_set(child));
                    context.push(' ');
                    context.push_str(&child.name);
                    node = child;
                }
                None if !node.subcommands.is_empty() && !has_positionals(node) => {
                    return Some(Issue {
                        kind: IssueKind::Verb,
                        token: text.to_string(),
                        context,
                    });
                }
                None => descending = false,
            }
        } else {
            // Placeholder (`<slice>`, `$VAR`, `…`) or free positional:
            // verb descent is over; flags keep being checked.
            descending = false;
        }
        if token.terminal {
            break;
        }
        idx += 1;
    }
    None
}

/// Check one `--flag` token against the accumulated allow set.
/// Returns `None` for non-flag tokens, placeholder-decorated flags
/// (`--[no-]x`, `--<flag>`), and clap's auto `--help` / `--version`.
fn check_flag_token(text: &str, allowed_flags: &BTreeSet<&str>, context: &str) -> Option<Issue> {
    let name = text.split('=').next().unwrap_or(text);
    if !FLAG_RE.is_match(name) {
        return None;
    }
    if name == "--help" || name == "--version" || allowed_flags.contains(name) {
        return None;
    }
    Some(Issue {
        kind: IssueKind::Flag,
        token: name.to_string(),
        context: context.to_string(),
    })
}

fn flag_set(node: &CommandNode) -> BTreeSet<&str> {
    node.args.iter().filter(|arg| arg.starts_with("--")).map(String::as_str).collect()
}

fn has_positionals(node: &CommandNode) -> bool {
    node.args.iter().any(|arg| arg.starts_with('<'))
}

// --- event-ids -------------------------------------------------------

fn event_ids(
    rule: &ResolvedRule, candidates: &[PathBuf], project_dir: &Path, model: &WorkspaceModel,
    contract: &CliContract, cfg: &IdsConfig, next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let probe = IdProbe {
        cfg,
        known: &contract.journal_event_ids,
        shape: IdShape::DottedKebab,
        label: "journal event id",
        families: Some(event_families(&contract.journal_event_ids)),
    };
    let mut findings = json_field_findings(rule, candidates, model, &probe, next_id);

    for (path, text) in markdown_texts(candidates, project_dir)? {
        for (line, span) in inline_spans(&text) {
            if cfg.ignore_suffixes.iter().any(|s| span.ends_with(s.as_str())) {
                continue;
            }
            if !probe.flags(&span) {
                continue;
            }
            findings.push(unknown_id_finding(rule, *next_id, &path, line, &span, probe.label));
            *next_id += 1;
        }
    }

    Ok(findings)
}

/// The contract-derived event-id namespace: the first dotted segment
/// of every declared id (`plan.transition.approved` contributes
/// `plan`). Dotless ids contribute nothing.
fn event_families(ids: &[String]) -> BTreeSet<&str> {
    ids.iter().filter_map(|id| id.split_once('.').map(|(family, _rest)| family)).collect()
}

// --- test-citations ----------------------------------------------------

fn test_citations(
    rule: &ResolvedRule, candidates: &[PathBuf], project_dir: &Path, contract: &CliContract,
    cfg: &TestCitationsConfig, next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    // An empty inventory means the binary was built without a test
    // tree; there is nothing to check citations against.
    if contract.tests.is_empty() {
        return Ok(Vec::new());
    }

    let mut citations: BTreeSet<(String, u32, String)> = BTreeSet::new();
    for (path, text) in markdown_texts(candidates, project_dir)? {
        for (line, span) in inline_spans(&text) {
            if TEST_PATH_RE.is_match(&span) {
                citations.insert((path.clone(), line, span));
            }
        }
        for (idx, raw_line) in text.lines().enumerate() {
            let line_no = u32::try_from(idx + 1).unwrap_or(u32::MAX);
            for prefix in &cfg.link_prefixes {
                for cited in linked_test_paths(raw_line, prefix) {
                    citations.insert((path.clone(), line_no, cited));
                }
            }
        }
    }

    let mut findings = Vec::new();
    for (path, line, cited) in citations {
        if cfg.ignore.iter().any(|t| t == &cited) || inventory_has(&contract.tests, &cited) {
            continue;
        }
        findings.push(unknown_id_finding(rule, *next_id, &path, line, &cited, "test citation"));
        *next_id += 1;
    }
    Ok(findings)
}

/// A cited path is satisfied by an exact inventory file or, for a
/// directory citation, by any inventoried file beneath it.
fn inventory_has(inventory: &[String], cited: &str) -> bool {
    let dir_prefix = format!("{}/", cited.trim_end_matches('/'));
    inventory.iter().any(|entry| entry == cited || entry.starts_with(dir_prefix.as_str()))
}

/// Extract the `tests/…` paths cited behind `prefix` in one line. A
/// path ends at the first URL terminator (whitespace, `)`, quote,
/// backtick, angle bracket, or `#` fragment); a trailing `/` from a
/// directory citation is dropped.
fn linked_test_paths(line: &str, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find(prefix) {
        let after = &rest[at + prefix.len()..];
        if after.starts_with("tests/") {
            let end = after
                .find(|c: char| {
                    c.is_whitespace() || matches!(c, ')' | '"' | '\'' | '`' | '<' | '>' | '#')
                })
                .unwrap_or(after.len());
            let path = after[..end].trim_end_matches('/');
            if !path.is_empty() {
                out.push(path.to_string());
            }
        }
        rest = after;
    }
    out
}

// --- shared id probes --------------------------------------------------

/// Token shape an extracted JSON field value must have to be
/// membership-checked; values of other shapes (placeholders, prose)
/// are skipped.
#[derive(Clone, Copy)]
enum IdShape {
    Kebab,
    DottedKebab,
}

impl IdShape {
    fn matches(self, value: &str) -> bool {
        match self {
            Self::Kebab => KEBAB_RE.is_match(value),
            Self::DottedKebab => DOTTED_KEBAB_RE.is_match(value),
        }
    }
}

/// One selector's membership probe: the known-id list, the required
/// token shape, the optional contract-derived namespace gate, and the
/// rule-supplied exemptions.
struct IdProbe<'a> {
    cfg: &'a IdsConfig,
    known: &'a [String],
    shape: IdShape,
    label: &'static str,
    /// `Some` gates candidates to tokens whose first dotted segment is
    /// a declared family; `None` checks every shape-matching token.
    families: Option<BTreeSet<&'a str>>,
}

impl IdProbe<'_> {
    /// Whether `value` is a candidate of this probe's namespace that is
    /// neither declared by the contract nor exempted by the rule.
    fn flags(&self, value: &str) -> bool {
        self.shape.matches(value)
            && self.in_namespace(value)
            && !self.known.iter().any(|id| id == value)
            && !self.cfg.permits(value)
    }

    fn in_namespace(&self, value: &str) -> bool {
        self.families.as_ref().is_none_or(|families| {
            value.split('.').next().is_some_and(|segment| families.contains(segment))
        })
    }
}

/// Scan candidate fenced bodies for `"<field>": "<value>"` pairs and
/// flag values the probe reports as unknown.
fn json_field_findings(
    rule: &ResolvedRule, candidates: &[PathBuf], model: &WorkspaceModel, probe: &IdProbe<'_>,
    next_id: &mut u64,
) -> Vec<Diagnostic> {
    let candidate_set = super::candidate_set(candidates);
    let mut findings = Vec::new();

    for field in &probe.cfg.json_fields {
        let pattern = format!("\"{}\"\\s*:\\s*\"([^\"\\n]*)\"", regex::escape(field));
        let Ok(re) = Regex::new(&pattern) else {
            continue;
        };
        for block in &model.fenced_blocks {
            if !candidate_set.contains(&block.path) {
                continue;
            }
            for (offset, line) in block.body.lines().enumerate() {
                for caps in re.captures_iter(line) {
                    let value = &caps[1];
                    if !probe.flags(value) {
                        continue;
                    }
                    let file_line =
                        block.line_start.saturating_add(u32::try_from(offset).unwrap_or(u32::MAX));
                    findings.push(unknown_id_finding(
                        rule,
                        *next_id,
                        &block.path,
                        file_line,
                        value,
                        probe.label,
                    ));
                    *next_id += 1;
                }
            }
        }
    }

    findings
}

fn unknown_id_finding(
    rule: &ResolvedRule, id_num: u64, path: &str, line: u32, token: &str, label: &str,
) -> Diagnostic {
    make_finding(
        rule,
        id_num,
        format!("Unknown {label}: {path}:{line} — `{token}` is not in the CLI contract"),
        Some(FindingLocation {
            path: path.to_string(),
            line: Some(line),
            column: None,
            end_line: None,
            end_column: None,
        }),
        FindingEvidence::Structured {
            summary: format!("`{token}` is not a {label} the binary declares"),
            data: serde_json::json!({ "path": path, "line": line, "token": token }),
            locations: None,
        },
    )
}

// --- scanning helpers --------------------------------------------------

static KEBAB_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$").expect("kebab regex compiles"));
static DOTTED_KEBAB_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-z][a-z0-9]*(?:-[a-z0-9]+)*(?:\.[a-z][a-z0-9]*(?:-[a-z0-9]+)*)+$")
        .expect("dotted-kebab regex compiles")
});
static FLAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^--[a-z][a-z0-9-]*$").expect("flag regex compiles"));
static TEST_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^tests/[A-Za-z0-9_./-]+\.rs$").expect("test path regex compiles")
});
static INLINE_SPAN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`([^`\n]+)`").expect("inline span regex compiles"));

/// One whitespace token of a command line, with shell punctuation
/// stripped and a flag for "the command ends after this token".
struct ShellToken {
    text: String,
    terminal: bool,
}

/// `specify` occurrence test: the bare word, or the word glued to a
/// command substitution opener (`VAR=$(specify …`).
fn is_specify_word(text: &str) -> bool {
    text == "specify" || text.ends_with("(specify")
}

/// Shell terminators that end one command's token stream.
fn is_terminator(text: &str) -> bool {
    matches!(text, "|" | "||" | "&&" | ";" | "&")
        || text.starts_with('#')
        || text.starts_with('>')
        || text.starts_with('<') && !text.ends_with('>')
        || text.starts_with("2>")
}

/// Split a command line on whitespace, stripping the quoting and
/// subshell punctuation that glues onto tokens (`` ` ``, quotes,
/// `$(`, parens) and marking tokens whose trailing `;` / `)` ends
/// the command. A `#` comment token ends the line — prose in a
/// trailing comment must not register `specify` occurrences.
fn shell_tokens(line: &str) -> Vec<ShellToken> {
    line.split_whitespace()
        .take_while(|raw| !raw.starts_with('#'))
        .map(|raw| {
            let stripped = raw.strip_prefix("$(").unwrap_or(raw);
            let stripped = stripped.trim_start_matches(['`', '"', '\'', '(']);
            let trimmed = stripped.trim_end_matches(['`', '"', '\'']);
            let terminal = trimmed.ends_with(';') || trimmed.ends_with(')');
            ShellToken {
                text: trimmed.trim_end_matches([';', ')']).to_string(),
                terminal,
            }
        })
        .collect()
}

/// Split a fence body into logical lines, joining trailing-backslash
/// continuations. Returns `(0-based offset of the first physical
/// line, joined text)` pairs.
fn logical_lines(body: &str) -> Vec<(u32, String)> {
    let mut out: Vec<(u32, String)> = Vec::new();
    let mut current: Option<(u32, String)> = None;
    for (idx, line) in body.lines().enumerate() {
        let offset = u32::try_from(idx).unwrap_or(u32::MAX);
        let (joined, continues) =
            line.strip_suffix('\\').map_or((line, false), |stripped| (stripped, true));
        match current.as_mut() {
            Some((_, text)) => {
                text.push(' ');
                text.push_str(joined.trim_start());
            }
            None => current = Some((offset, joined.to_string())),
        }
        if !continues && let Some(done) = current.take() {
            out.push(done);
        }
    }
    if let Some(done) = current.take() {
        out.push(done);
    }
    out
}

/// Read each markdown candidate's text. Missing files are skipped
/// (scan/eval races); other I/O failures abort the hint.
fn markdown_texts(
    candidates: &[PathBuf], project_dir: &Path,
) -> Result<Vec<(String, String)>, HintError> {
    let mut out = Vec::new();
    for candidate in candidates {
        if candidate.extension().is_none_or(|ext| !ext.eq_ignore_ascii_case("md")) {
            continue;
        }
        let absolute = project_dir.join(candidate);
        let bytes = match std::fs::read(&absolute) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(HintError::Filesystem {
                    op: "read",
                    path: absolute,
                    source: err,
                });
            }
        };
        out.push((
            candidate.to_string_lossy().into_owned(),
            String::from_utf8_lossy(&bytes).into_owned(),
        ));
    }
    Ok(out)
}

/// Extract inline code spans (single-backtick) outside fenced blocks.
/// Returns `(1-based line, span content)` pairs.
fn inline_spans(text: &str) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    let mut fence: Option<&str> = None;
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(open_marker) = fence {
            if trimmed.starts_with(open_marker) {
                fence = None;
            }
            continue;
        }
        if trimmed.starts_with("```") {
            fence = Some("```");
            continue;
        }
        if trimmed.starts_with("~~~") {
            fence = Some("~~~");
            continue;
        }
        let line_no = u32::try_from(idx + 1).unwrap_or(u32::MAX);
        for caps in INLINE_SPAN_RE.captures_iter(line) {
            out.push((line_no, caps[1].to_string()));
        }
    }
    out
}

#[cfg(test)]
mod unit {
    use std::fs;
    use std::path::Path;

    use serde_json::json;

    use super::*;
    use crate::lint::FencedBlock;
    use crate::lint::eval::testkit::{candidates, empty_model, hint, hint_with_config, rule};

    fn node(name: &str, args: &[&str], subcommands: Vec<CommandNode>) -> CommandNode {
        CommandNode {
            name: name.to_string(),
            about: None,
            args: args.iter().map(|a| (*a).to_string()).collect(),
            subcommands,
        }
    }

    fn contract() -> CliContract {
        CliContract {
            version: 1,
            binary_version: "0.0.0".to_string(),
            commands: node(
                "specify",
                &["--format"],
                vec![node(
                    "plan",
                    &[],
                    vec![
                        node("add", &["<name>", "--target"], vec![]),
                        node("transition", &["<name>", "<state>", "--undo"], vec![]),
                    ],
                )],
            ),
            exit_codes: vec![],
            error_ids: vec!["adapter-not-found".to_string()],
            journal_event_ids: vec!["plan.transition.approved".to_string()],
            schemas: vec![],
            tests: vec!["tests/plan/end_to_end.rs".to_string()],
        }
    }

    fn block(path: &str, lang: &str, body: &str) -> FencedBlock {
        FencedBlock {
            path: path.to_string(),
            line_start: 10,
            line_end: 10 + u32::try_from(body.lines().count()).unwrap_or(0),
            lang: lang.to_string(),
            body: body.to_string(),
        }
    }

    fn run(
        hint_value: &str, config: serde_json::Value, model: &WorkspaceModel, cands: &[PathBuf],
        project_dir: &Path,
    ) -> Vec<Diagnostic> {
        let hint = hint_with_config(HintKind::CliContract, hint_value, Some(config));
        evaluate(&rule(), &hint, cands, project_dir, model, Some(&contract()), &mut 1)
            .expect("evaluate")
    }

    #[test]
    fn missing_contract_is_unsupported() {
        let model = empty_model();
        let hint = hint(HintKind::CliContract, "invocations");
        let result = evaluate(&rule(), &hint, &[], Path::new("/tmp"), &model, None, &mut 1);
        result.unwrap_err();
    }

    #[test]
    fn unknown_verb_and_flag_flagged() {
        let mut model = empty_model();
        let body = "specify plan add my-slice --target omnia\n\
                    specify plan destroy my-slice\n\
                    specify plan add my-slice --bogus\n";
        model.fenced_blocks = vec![block("docs/a.md", "bash", body)];
        let out = run(
            "invocations",
            json!({ "langs": ["bash"] }),
            &model,
            &candidates(&["docs/a.md"]),
            Path::new("/tmp"),
        );
        let titles: Vec<&str> = out.iter().map(|f| f.title.as_str()).collect();
        assert_eq!(out.len(), 2, "{titles:?}");
        assert!(titles.iter().any(|t| t.contains("`destroy`")), "{titles:?}");
        assert!(titles.iter().any(|t| t.contains("`--bogus`")), "{titles:?}");
    }

    #[test]
    fn comments_and_brace_expansion_end_the_walk() {
        let mut model = empty_model();
        let body = "specify plan add x # specify plan destroy y\n\
                    specify plan {add, transition} whatever-junk\n";
        model.fenced_blocks = vec![block("docs/a.md", "bash", body)];
        let out = run(
            "invocations",
            json!({ "langs": ["bash"] }),
            &model,
            &candidates(&["docs/a.md"]),
            Path::new("/tmp"),
        );
        assert!(out.is_empty(), "{:?}", out.iter().map(|f| &f.title).collect::<Vec<_>>());
    }

    #[test]
    fn inline_spans_checked_in_markdown() {
        let tmp = tempfile::tempdir().expect("tmp");
        fs::write(tmp.path().join("doc.md"), "Run `specify plan destroy x` to clean up.\n")
            .expect("doc");
        let model = empty_model();
        let out = run(
            "invocations",
            json!({ "langs": ["bash"] }),
            &model,
            &candidates(&["doc.md"]),
            tmp.path(),
        );
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("`destroy`"), "{}", out[0].title);
    }

    #[test]
    fn unknown_event_id_flagged_in_namespace_only() {
        let mut model = empty_model();
        let body = r#"{ "event": "plan.transition.approved" }
{ "event": "plan.no-such-event" }
{ "event": "users.register" }
"#;
        model.fenced_blocks = vec![block("docs/a.md", "json", body)];
        let out = run(
            "event-ids",
            json!({ "json-fields": ["event"] }),
            &model,
            &candidates(&["docs/a.md"]),
            Path::new("/tmp"),
        );
        // `users.register` is outside the declared family namespace.
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("`plan.no-such-event`"), "{}", out[0].title);
    }

    #[test]
    fn unknown_error_code_flagged() {
        let mut model = empty_model();
        let body = r#"{ "code": "adapter-not-found" }
{ "code": "no-such-error" }
"#;
        model.fenced_blocks = vec![block("docs/a.md", "json", body)];
        let out = run(
            "error-codes",
            json!({ "json-fields": ["code"] }),
            &model,
            &candidates(&["docs/a.md"]),
            Path::new("/tmp"),
        );
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("`no-such-error`"), "{}", out[0].title);
    }

    #[test]
    fn stale_test_citation_flagged() {
        let tmp = tempfile::tempdir().expect("tmp");
        fs::write(
            tmp.path().join("doc.md"),
            "See `tests/plan/end_to_end.rs` and `tests/gone/deleted.rs`.\n",
        )
        .expect("doc");
        let model = empty_model();
        let out = run(
            "test-citations",
            json!({ "link-prefixes": ["blob/main/"] }),
            &model,
            &candidates(&["doc.md"]),
            tmp.path(),
        );
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("`tests/gone/deleted.rs`"), "{}", out[0].title);
    }

    #[test]
    fn missing_config_is_unsupported() {
        let model = empty_model();
        let hint = hint(HintKind::CliContract, "invocations");
        let result =
            evaluate(&rule(), &hint, &[], Path::new("/tmp"), &model, Some(&contract()), &mut 1);
        result.unwrap_err();
    }

    #[test]
    fn unknown_source_is_unsupported() {
        let model = empty_model();
        let hint = hint_with_config(HintKind::CliContract, "no-such-source", Some(json!({})));
        let result =
            evaluate(&rule(), &hint, &[], Path::new("/tmp"), &model, Some(&contract()), &mut 1);
        result.unwrap_err();
    }
}
