//! The plugin rule (`plugins/emery/rules/emery.mdc`) must never name
//! a verb, flag, or skill the shipped surface does not have: every
//! mention is validated against the live router and skill tree.

mod support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use support::Inert;

// Global flags peeled or handled ahead of the verb grammar — they
// never appear in a route's own `--help`.
const GLOBAL_FLAGS: &[&str] = &["--debug", "--quiet", "--format", "--help", "--version"];

// Builtin routes outside the operation inventory.
const BUILTIN_PATHS: &[&[&str]] = &[&["completions"]];

// The ultrathin wrapper contract: each skill invokes exactly one CLI
// verb, so a flag mentioned on a slash command validates against that
// verb's grammar.
const SKILL_VERBS: &[(&str, &[&str])] = &[("init", &["init"])];

fn plugin_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/emery")
}

fn router() -> omnia_guest::api::command::Router<Inert, emery_transport::command::Globals> {
    support::router()
}

// Every full route path plus every namespace prefix, kebab-joined.
fn known_paths(
    router: &omnia_guest::api::command::Router<Inert, emery_transport::command::Globals>,
) -> (BTreeSet<Vec<String>>, BTreeSet<Vec<String>>) {
    let mut full: BTreeSet<Vec<String>> = BTreeSet::new();
    let mut prefixes: BTreeSet<Vec<String>> = BTreeSet::new();
    for route in router.inventory() {
        let path: Vec<String> = route.selector().path().to_vec();
        for len in 1..path.len() {
            prefixes.insert(path[..len].to_vec());
        }
        full.insert(path);
    }
    for builtin in BUILTIN_PATHS {
        full.insert(builtin.iter().map(ToString::to_string).collect());
    }
    (full, prefixes)
}

// One `emery …` or `/emery:<skill>` mention lifted from the rule.
#[derive(Debug)]
enum Mention {
    Cli(String),
    Skill { name: String, rest: String },
}

// Lift every mention from one inline code span or fenced line.
// `.emery/` and `emery-adapters` never match: `emery` must stand
// alone at a word boundary followed by whitespace or the span end.
fn mentions_in(text: &str, out: &mut Vec<Mention>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while let Some(found) = text[i..].find("emery") {
        let start = i + found;
        let end = start + "emery".len();
        let before = start.checked_sub(1).map(|b| bytes[b] as char);
        let after = bytes.get(end).map(|b| *b as char);
        i = end;
        if before == Some('/') && after == Some(':') {
            // `/emery:<skill>` — the slash-command surface.
            let rest = &text[end + 1..];
            let name: String =
                rest.chars().take_while(|ch| ch.is_ascii_lowercase() || *ch == '-').collect();
            let tail = rest[name.len()..].to_string();
            if !name.is_empty() {
                out.push(Mention::Skill { name, rest: tail });
            }
            continue;
        }
        let boundary_before =
            before.is_none_or(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '/' | '-')));
        let boundary_after = after.is_none_or(char::is_whitespace);
        if boundary_before && boundary_after {
            out.push(Mention::Cli(text[start..].to_string()));
        }
    }
}

// Every mention in the rule document: fenced lines verbatim, inline
// code spans from prose lines.
fn mentions(doc: &str) -> Vec<Mention> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for line in doc.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            mentions_in(line, &mut out);
            continue;
        }
        let mut code = false;
        for part in line.split('`') {
            if code {
                mentions_in(part, &mut out);
            }
            code = !code;
        }
    }
    out
}

fn is_kebab(token: &str) -> bool {
    !token.is_empty()
        && token.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        && !token.starts_with('-')
}

// Walk the mention's leading kebab tokens while they extend a known
// route path or namespace prefix. Returns the walked path and the
// first unconsumed token.
fn walk_path<'a>(
    tokens: &[&'a str], base: &[String], full: &BTreeSet<Vec<String>>,
    prefixes: &BTreeSet<Vec<String>>,
) -> (Vec<String>, Option<&'a str>) {
    let mut path: Vec<String> = base.to_vec();
    let mut rest = None;
    for token in tokens {
        if !is_kebab(token) {
            rest = Some(*token);
            break;
        }
        let mut candidate = path.clone();
        candidate.push((*token).to_string());
        if full.contains(&candidate) || prefixes.contains(&candidate) {
            path = candidate;
        } else {
            rest = Some(*token);
            break;
        }
    }
    (path, rest)
}

// The mention's flag tokens (`--…`), stripped of surrounding
// brackets/quotes.
fn flags_of(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter_map(|token| {
            let token = token.trim_matches(|ch: char| {
                matches!(ch, '[' | ']' | '(' | ')' | '"' | '\'' | ',' | ';' | '.')
            });
            token.starts_with("--").then(|| {
                token
                    .split_once('=')
                    .map_or(token, |(flag, _value)| flag)
                    .trim_end_matches(|ch: char| !(ch.is_ascii_alphanumeric()))
                    .to_string()
            })
        })
        .collect()
}

async fn assert_flags(
    router: &omnia_guest::api::command::Router<Inert, emery_transport::command::Globals>,
    path: &[String], text: &str,
) {
    let mut help: Option<String> = None;
    for flag in flags_of(text) {
        if GLOBAL_FLAGS.contains(&flag.as_str()) {
            continue;
        }
        assert!(
            !path.is_empty(),
            "flag `{flag}` mentioned with no verb to validate against (in `{text}`)"
        );
        if help.is_none() {
            let mut argv = vec!["emery".to_string()];
            argv.extend(path.iter().cloned());
            argv.push("--help".to_string());
            let response = router.execute(argv.iter().map(String::as_str)).await;
            assert_eq!(response.exit, 0, "`emery {} --help` must succeed", path.join(" "));
            help = Some(String::from_utf8_lossy(&response.stdout).into_owned());
        }
        let help = help.as_deref().expect("help rendered above");
        assert!(
            help.contains(&flag),
            "rule names `{flag}` on `emery {}`, but the grammar has no such flag",
            path.join(" ")
        );
    }
}

// The rule can only name verbs, flags, and skills the shipped surface
// has — mechanical enforcement over the live router, not prose review.
#[tokio::test]
async fn rule_matches_router() {
    let rule = plugin_dir().join("rules/emery.mdc");
    let doc = std::fs::read_to_string(&rule)
        .unwrap_or_else(|err| panic!("reading {}: {err}", rule.display()));
    let router = router();
    let (full, prefixes) = known_paths(&router);

    let mentions = mentions(&doc);
    assert!(
        mentions.iter().any(|mention| matches!(mention, Mention::Cli(_))),
        "the rule mentions no `emery` command at all — the extractor regressed"
    );

    for mention in mentions {
        match mention {
            Mention::Cli(text) => {
                // Alternation (`emery slice list | validate | model show`)
                // resolves every alternative against the first segment's
                // namespace.
                let mut segments = text.split('|');
                let first = segments.next().expect("split yields at least one segment");
                let tokens: Vec<&str> = first.split_whitespace().skip(1).collect();
                let (path, rest) = walk_path(&tokens, &[], &full, &prefixes);
                if !full.contains(&path) {
                    // A namespace mention is fine; a further kebab token
                    // would have been an unknown verb.
                    assert!(
                        rest.is_none_or(|token| !is_kebab(token)),
                        "rule names `emery {} {}`, which is not a routed verb (in `{text}`)",
                        path.join(" "),
                        rest.unwrap_or_default(),
                    );
                }
                assert_flags(&router, &path, first).await;
                let namespace =
                    if path.is_empty() { Vec::new() } else { path[..path.len() - 1].to_vec() };
                for segment in segments {
                    let tokens: Vec<&str> = segment.split_whitespace().collect();
                    let (alt, _rest) = walk_path(&tokens, &namespace, &full, &prefixes);
                    assert!(
                        full.contains(&alt),
                        "rule alternative `{segment}` does not resolve to a routed verb \
                         under `{}` (in `{text}`)",
                        namespace.join(" "),
                    );
                }
            }
            Mention::Skill { name, rest } => {
                let skill = plugin_dir().join("skills").join(&name).join("SKILL.md");
                assert!(skill.is_file(), "rule names `/emery:{name}`, but {skill:?} is missing");
                let verb = SKILL_VERBS
                    .iter()
                    .find_map(|(skill, verb)| (*skill == name).then_some(*verb))
                    .unwrap_or_else(|| {
                        panic!("skill `{name}` has no CLI verb mapping in this test — add it")
                    });
                let path: Vec<String> = verb.iter().map(ToString::to_string).collect();
                assert_flags(&router, &path, &rest).await;
            }
        }
    }
}

// Every shipped skill is reachable from the rule, so the operator-facing
// index cannot silently omit a surface.
#[test]
fn rule_names_every_skill() {
    let doc = std::fs::read_to_string(plugin_dir().join("rules/emery.mdc")).expect("rule");
    let skills = std::fs::read_dir(plugin_dir().join("skills")).expect("skills dir");
    for entry in skills {
        let entry = entry.expect("skill entry");
        if !entry.path().join("SKILL.md").is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(
            doc.contains(&format!("/emery:{name}")) || doc.contains(&format!("skills/{name}/")),
            "shipped skill `{name}` is not mentioned by the always-applied rule"
        );
    }
}
