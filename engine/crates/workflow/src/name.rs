//! Identifier newtypes for the journal / plan / slice chains.
//!
//! [`SliceName`] and [`PlanName`] wrap the kebab-case identifiers that
//! flow through `plan.yaml.name`, `plan.yaml.slices[].name`, and every
//! journal event payload. They exist to stop a plan name being passed
//! where a slice name is expected (the two sit adjacently in several
//! journal variants) without changing the wire format: both are
//! `#[serde(transparent)]`, so a value serialises and deserialises
//! exactly as the bare string it wraps.

use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

/// Declares an identifier newtype around `String` with the ergonomics
/// every call site relies on: cheap construction from string-likes,
/// `Display`, `AsRef<str>` / `Deref<str>` / `Borrow<str>` so the inner
/// `str` API and `&str`-keyed maps keep working, and equality against
/// bare strings for assertions. Serialises transparently.
macro_rules! identifier_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps `value` without validation; the kebab-case grammar
            /// is enforced by the CLI argument parser and the schemas,
            /// not by the newtype.
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Borrows the wrapped identifier as a `&str`.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Unwraps into the owned `String`.
            #[must_use]
            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl From<&String> for $name {
            fn from(value: &String) -> Self {
                Self(value.clone())
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }

        impl PartialEq<String> for $name {
            fn eq(&self, other: &String) -> bool {
                &self.0 == other
            }
        }

        impl PartialEq<$name> for str {
            fn eq(&self, other: &$name) -> bool {
                self == other.0
            }
        }
    };
}

identifier_newtype! {
    /// A slice identifier — `plan.yaml.slices[].name` and the
    /// `.specify/slices/<name>/` directory stem.
    SliceName
}

identifier_newtype! {
    /// A plan / change identifier — `plan.yaml.name`.
    PlanName
}

#[cfg(test)]
mod tests;
