//! Environment reads for the backend's connect options.
//!
//! Hand-rolled rather than derived: `ConnectOptions` here dispatches on one
//! variable before the others are meaningful, which no declarative
//! environment binding expresses.

use anyhow::{Context as _, Result};

/// A set, non-blank variable.
pub fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.trim().is_empty())
}

/// A `u64` variable interpreted as seconds, or `default` when unset.
pub fn secs(name: &str, default: u64) -> Result<u64> {
    parse(name, default)
}

/// A `u64` variable interpreted as milliseconds, or `default` when unset.
pub fn millis(name: &str, default: u64) -> Result<u64> {
    parse(name, default)
}

/// A `u32` variable interpreted as a repeat count, or `default` when unset.
pub fn count(name: &str, default: u32) -> Result<u32> {
    parse(name, default)
}

/// Whether a variable is set to something other than a falsey word. Present
/// but empty counts as unset, matching the other readers here.
pub fn flag(name: &str) -> bool {
    is_truthy(var(name).as_deref())
}

fn is_truthy(value: Option<&str>) -> bool {
    value.is_some_and(|value| !matches!(value.trim().to_lowercase().as_str(), "0" | "false" | "no"))
}

fn parse<T: std::str::FromStr>(name: &str, default: T) -> Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    var(name).map_or_else(
        || Ok(default),
        |value| {
            value.trim().parse().with_context(|| format!("{name} must be a number, not `{value}`"))
        },
    )
}

// The readers themselves touch process-global state; the tests cover the
// pure parsing beneath them, which is where the behaviour lives.
#[cfg(test)]
mod tests {
    use super::is_truthy;

    #[test]
    fn unset_is_off() {
        assert!(!is_truthy(None));
    }

    #[test]
    fn falsey_words_are_off() {
        for value in ["0", "false", "FALSE", " no "] {
            assert!(!is_truthy(Some(value)), "`{value}` reads as off");
        }
    }

    #[test]
    fn anything_else_is_on() {
        for value in ["1", "true", "yes", "on"] {
            assert!(is_truthy(Some(value)), "`{value}` reads as on");
        }
    }
}
