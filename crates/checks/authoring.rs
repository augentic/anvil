//! Typed authoring checks over the operator-plugin surface:
//! `plugins/*/skills/*/SKILL.md` frontmatter and the
//! `.cursor-plugin/marketplace.json` manifest. The Rust structs here
//! are the editor contract — serde parse plus the small deterministic
//! grammar checks below.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/checks/ sits two levels under the repo root")
        .to_path_buf()
}

fn is_kebab(value: &str) -> bool {
    !value.is_empty()
        && value.starts_with(|c: char| c.is_ascii_lowercase())
        && value.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !value.contains("--")
        && !value.ends_with('-')
}

// --- SKILL.md frontmatter -------------------------------------------

/// The closed SKILL.md frontmatter shape (Anthropic/Cursor syntax).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillFrontmatter {
    /// Plugin-qualified kebab identifier, max 64 chars.
    name: String,
    /// What the skill does and when to use it; must carry a
    /// "Use when …" clause.
    description: String,
    /// Slash-command placeholder tokens.
    #[serde(rename = "argument-hint")]
    argument_hint: Option<String>,
    /// Space-separated tool names; omit to inherit.
    #[serde(rename = "allowed-tools")]
    #[expect(dead_code, reason = "accepted, no grammar beyond being a string")]
    allowed_tools: Option<String>,
}

fn frontmatter(text: &str, path: &Path) -> String {
    let rest = text
        .strip_prefix("---\n")
        .unwrap_or_else(|| panic!("{}: SKILL.md must open with `---` frontmatter", path.display()));
    let end = rest
        .find("\n---")
        .unwrap_or_else(|| panic!("{}: unterminated frontmatter", path.display()));
    rest[..end].to_string()
}

fn skill_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let plugins = root.join("plugins");
    for plugin in fs::read_dir(&plugins).expect("plugins/ exists") {
        let skills = plugin.expect("plugin dir entry").path().join("skills");
        if !skills.is_dir() {
            continue;
        }
        for skill in fs::read_dir(&skills).expect("skills/ readable") {
            let candidate = skill.expect("skill dir entry").path().join("SKILL.md");
            if candidate.is_file() {
                out.push(candidate);
            }
        }
    }
    out.sort();
    out
}

/// One `argument-hint` token: `<name>` / `[name]` (with optional
/// `|alt`s and a trailing `...`) or a `--flag`.
fn hint_token_ok(token: &str) -> bool {
    if let Some(flag) = token.strip_prefix("--") {
        return is_kebab(flag);
    }
    let token = token.strip_suffix("...").unwrap_or(token);
    let bracketed = (token.starts_with('<') && token.ends_with('>'))
        || (token.starts_with('[') && token.ends_with(']'));
    if !bracketed || token.len() < 3 {
        return false;
    }
    token[1..token.len() - 1].split('|').all(is_kebab)
}

#[test]
fn skill_frontmatter() {
    let root = repo_root();
    let files = skill_files(&root);
    assert!(!files.is_empty(), "no SKILL.md files found under plugins/");
    for path in files {
        let text = fs::read_to_string(&path).expect("SKILL.md readable");
        let fm: SkillFrontmatter = serde_saphyr::from_str(&frontmatter(&text, &path))
            .unwrap_or_else(|err| panic!("{}: frontmatter does not parse: {err}", path.display()));

        assert!(
            is_kebab(&fm.name) && fm.name.len() <= 64,
            "{}: `name` must be a kebab identifier of at most 64 chars, got `{}`",
            path.display(),
            fm.name
        );
        for reserved in ["anthropic", "claude"] {
            assert!(
                !fm.name.contains(reserved),
                "{}: `name` must avoid the reserved word `{reserved}`",
                path.display()
            );
        }
        assert!(
            (10..=512).contains(&fm.description.len()),
            "{}: `description` must be 10–512 chars",
            path.display()
        );
        assert!(
            fm.description.contains("Use when") || fm.description.contains("use when"),
            "{}: `description` must carry a 'Use when …' clause",
            path.display()
        );
        if let Some(hint) = &fm.argument_hint {
            for token in hint.split_whitespace() {
                assert!(
                    hint_token_ok(token),
                    "{}: `argument-hint` token `{token}` is not `<name>`, `[name]`, or `--flag`",
                    path.display()
                );
            }
        }
    }
}

// --- .cursor-plugin/marketplace.json --------------------------------

/// The closed marketplace manifest shape (editor contract).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marketplace {
    name: String,
    owner: Owner,
    metadata: Metadata,
    plugins: Vec<PluginEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Metadata {
    description: String,
    version: String,
    plugin_root: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PluginEntry {
    name: String,
    source: String,
    description: String,
}

#[test]
fn marketplace_manifest() {
    let root = repo_root();
    let path = root.join(".cursor-plugin/marketplace.json");
    let text = fs::read_to_string(&path).expect("marketplace.json readable");
    let manifest: Marketplace = serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("{}: manifest does not parse: {err}", path.display()));

    assert!(!manifest.name.is_empty(), "marketplace `name` must be non-empty");
    assert!(!manifest.owner.name.is_empty(), "owner `name` must be non-empty");
    assert!(manifest.owner.email.contains('@'), "owner `email` must be an email address");
    assert!(!manifest.metadata.description.is_empty(), "metadata `description` non-empty");
    assert!(
        semver::Version::parse(&manifest.metadata.version).is_ok(),
        "metadata `version` must be SemVer, got `{}`",
        manifest.metadata.version
    );
    assert_eq!(manifest.metadata.plugin_root, "plugins", "pluginRoot is pinned to `plugins`");
    assert!(!manifest.plugins.is_empty(), "`plugins` must list at least one plugin");
    for plugin in &manifest.plugins {
        assert!(is_kebab(&plugin.name), "plugin `name` `{}` must be kebab", plugin.name);
        assert!(is_kebab(&plugin.source), "plugin `source` `{}` must be kebab", plugin.source);
        assert!(!plugin.description.is_empty(), "plugin `description` non-empty");
        assert!(
            root.join("plugins").join(&plugin.source).is_dir(),
            "plugin source `plugins/{}` must exist",
            plugin.source
        );
    }
}
