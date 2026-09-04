//! Plugin-rule mentions against the shipped CLI surface.
//!
//! The always-applied Cursor rule may only name live verbs, flags, and
//! shipped skills — a cross-cutting product contract, not an
//! engine-internal one.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use emery_source::types::{Evidence, SourceInput, SourceMetadata};
use emery_source::{DispatchError, Source};
use omnia_test::guest::Memory;

type Grammar = emery_engine::cli::Cli<Inert>;

// Capabilities are never dispatched; the suite only inspects the grammar.
#[derive(Clone, Debug, Default)]
struct Inert {
    storage: Arc<Memory>,
}

omnia_test::delegate!(impl Inert {
    StateStore + BlobStore => storage,
});

impl omnia_guest::Model for Inert {
    fn complete(
        &self, _request: omnia_guest::model::Request,
    ) -> impl Future<Output = Result<omnia_guest::model::Reply, omnia_guest::model::Error>> {
        std::future::ready(never_dispatched())
    }

    fn complete_with<H, F>(
        &self, _request: omnia_guest::model::Request, _handler: H,
    ) -> impl Future<Output = Result<omnia_guest::model::Reply, omnia_guest::model::Error>> + Send
    where
        H: FnMut(omnia_guest::model::ToolCall) -> F + Send,
        F: Future<Output = Result<String, String>> + Send,
    {
        std::future::ready(never_dispatched())
    }
}

impl Source for Inert {
    fn extract(
        &self, _id: &str, _input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, DispatchError>> + Send {
        std::future::ready(never_extracted())
    }

    fn metadata(&self, _id: &str) -> SourceMetadata {
        unreachable!("the plugin suite never dispatches Source")
    }
}

impl omnia_guest::Plugins for Inert {
    fn load(
        &self, _plugin: &omnia_guest::plugins::PluginRef,
    ) -> impl Future<Output = Result<omnia_guest::plugins::Plugin, omnia_guest::plugins::Error>> + Send
    {
        std::future::ready(never_loaded())
    }
}

fn never_dispatched() -> Result<omnia_guest::model::Reply, omnia_guest::model::Error> {
    unreachable!("the plugin suite never dispatches the model")
}

fn never_loaded() -> Result<omnia_guest::plugins::Plugin, omnia_guest::plugins::Error> {
    unreachable!("the plugin suite never dispatches the loader")
}

fn never_extracted() -> Result<Evidence, DispatchError> {
    unreachable!("the plugin suite never dispatches Source")
}

fn grammar() -> Grammar {
    emery_engine::cli::router(Inert::default())
}

// Global flags do not appear in verb-specific help.
const GLOBAL_FLAGS: &[&str] = &["--debug", "--quiet", "--format", "--help", "--version"];

// Each skill's flags validate against its single wrapped verb.
const SKILL_VERBS: &[(&str, &str)] = &[("specify", "specify")];

fn plugin_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/emery")
}

fn known_verbs() -> BTreeSet<String> {
    emery_engine::cli::verbs().into_iter().collect()
}

#[derive(Debug)]
enum Mention {
    Cli(String),
    Skill { name: String, rest: String },
}

// Require a standalone `emery`, excluding `.emery/` and `emery-adapters`.
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

// Return the first live verb and the first unconsumed token.
fn walk_verb<'a>(
    tokens: &[&'a str], verbs: &BTreeSet<String>,
) -> (Option<String>, Option<&'a str>) {
    let mut rest = None;
    let mut verb = None;
    for token in tokens {
        if !is_kebab(token) {
            rest = Some(*token);
            break;
        }
        if verb.is_none() && verbs.contains(*token) {
            verb = Some((*token).to_owned());
            continue;
        }
        rest = Some(*token);
        break;
    }
    (verb, rest)
}

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

async fn assert_flags(router: &Grammar, verb: &str, text: &str) {
    let mut help: Option<String> = None;
    for flag in flags_of(text) {
        if GLOBAL_FLAGS.contains(&flag.as_str()) {
            continue;
        }
        assert!(
            !verb.is_empty(),
            "flag `{flag}` mentioned with no verb to validate against (in `{text}`)"
        );
        if help.is_none() {
            let response = router.execute(["emery", verb, "--help"]).await;
            assert_eq!(response.exit, 0, "`emery {verb} --help` must succeed");
            help = Some(String::from_utf8_lossy(&response.stdout).into_owned());
        }
        let help = help.as_deref().expect("help rendered above");
        assert!(
            help.contains(&flag),
            "rule names `{flag}` on `emery {verb}`, but the grammar has no such flag"
        );
    }
}

// Plugin-rule CLI mentions must resolve to live verbs and flags.
#[tokio::test]
async fn rule_matches_router() {
    let rule = plugin_dir().join("rules/emery.mdc");
    let doc = std::fs::read_to_string(&rule)
        .unwrap_or_else(|err| panic!("reading {}: {err}", rule.display()));
    let router = grammar();
    let verbs = known_verbs();

    let mentions = mentions(&doc);
    assert!(
        mentions.iter().any(|mention| matches!(mention, Mention::Cli(_))),
        "the rule mentions no `emery` command at all — the extractor regressed"
    );

    for mention in mentions {
        match mention {
            Mention::Cli(text) => {
                let mut segments = text.split('|');
                let first = segments.next().expect("split yields at least one segment");
                let tokens: Vec<&str> = first.split_whitespace().skip(1).collect();
                let (verb, rest) = walk_verb(&tokens, &verbs);
                if verb.is_none() {
                    assert!(
                        rest.is_none_or(|token| !is_kebab(token)),
                        "rule names `emery {}`, which is not a verb (in `{text}`)",
                        rest.unwrap_or_default(),
                    );
                }
                assert_flags(&router, verb.as_deref().unwrap_or(""), first).await;
                for segment in segments {
                    let tokens: Vec<&str> = segment.split_whitespace().collect();
                    let (alt, _rest) = walk_verb(&tokens, &verbs);
                    assert!(
                        alt.is_some(),
                        "rule alternative `{segment}` does not resolve to a verb (in `{text}`)",
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
                assert_flags(&router, verb, &rest).await;
            }
        }
    }
}

// Every shipped skill is named by the always-applied rule.
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
