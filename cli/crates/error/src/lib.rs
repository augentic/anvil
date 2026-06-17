//! Unified error types for the `specify` CLI and its domain crates.
//! Every public function returns `Result<T, Error>`; variants are
//! structured so the binary can route them to exit codes and formats.

pub mod codes;
pub mod error;
pub mod serde_rfc3339;
pub mod serde_rfc3339_opt;

pub use error::Error;

/// Workspace-wide `Result` alias bound to [`Error`].
///
/// Lets call sites write `specify_error::Result<T>` (or `Result<T>`
/// after `use specify_error::Result`) without restating the error
/// parameter; supply an explicit `E` to override on the rare path that
/// returns a non-[`Error`] failure.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Kebab-case predicate shared across every workspace crate.
///
/// Mirrors the JSON Schema regex `^[a-z0-9]+(-[a-z0-9]+)*$` carried by
/// `schemas/plan/plan.schema.json` `$defs.kebabName.pattern`: one or
/// more hyphen-separated segments; each segment is non-empty and
/// contains only ASCII lowercase letters and digits.
#[must_use]
pub fn is_kebab(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|seg| {
            !seg.is_empty() && seg.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

/// [`is_kebab`] plus a leading ASCII-lowercase-letter requirement.
///
/// The `^[a-z][a-z0-9]*(-[a-z0-9]+)*$` shape used by component slugs
/// and target names, which (unlike plain kebab) may not start with a
/// digit.
#[must_use]
pub fn is_kebab_leading_alpha(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase()) && is_kebab(s)
}

#[cfg(test)]
#[test]
fn is_kebab_accepts_and_rejects() {
    for ok in ["a", "abc", "alpha-gateway", "x-1", "a1-b2"] {
        assert!(is_kebab(ok), "expected `{ok}` to pass");
    }
    for bad in ["", "-a", "a-", "a--b", "A", "alpha_gateway", "alpha gateway"] {
        assert!(!is_kebab(bad), "expected `{bad}` to fail");
    }
}

#[cfg(test)]
#[test]
fn is_kebab_leading_alpha_rejects_digit_start() {
    for ok in ["a", "tab-bar", "x-1"] {
        assert!(is_kebab_leading_alpha(ok), "expected `{ok}` to pass");
    }
    for bad in ["", "1a", "9-lives", "-a", "a--b", "A"] {
        assert!(!is_kebab_leading_alpha(bad), "expected `{bad}` to fail");
    }
}
