//! Single source of truth for the file-extension → language-token map
//! shared by the consumer ([`super::files`]) and framework
//! ([`super::framework`]) scan profiles per `WorkspaceModel` file scan.

/// Extension (without the leading dot) → language token.
///
/// The superset of both scan profiles' needs: the project profile only
/// reaches this table for extensions that already passed its include
/// filter, so the extra framework-only entries (`sh` → `shell`) are
/// inert there.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("rs", "rust"),
    ("swift", "swift"),
    ("kt", "kotlin"),
    ("kts", "kotlin"),
    ("gradle", "kotlin"),
    ("ts", "typescript"),
    ("tsx", "typescript"),
    ("js", "javascript"),
    ("jsx", "javascript"),
    ("py", "python"),
    ("sql", "sql"),
    ("md", "markdown"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("json", "json"),
    ("toml", "toml"),
    ("sh", "shell"),
];

/// Infer the language token from a project-relative path's extension.
///
/// `None` for extensions outside [`LANGUAGES`] (the caller treats
/// unknown files as language-agnostic).
#[must_use]
pub fn infer_language(relative: &str) -> Option<String> {
    let ext = relative.rsplit_once('.').map(|(_, ext)| ext)?;
    LANGUAGES.iter().find_map(|(token, lang)| (*token == ext).then(|| (*lang).to_owned()))
}
