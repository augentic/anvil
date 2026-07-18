//! Compile-time adapter identity for linked (statically composed)
//! deployments.
//!
//! The component deployment derives identity from the published
//! package and its resolved artifact location. A linked host has no
//! component artifact, so each SDK implementor exposes the equivalent
//! identity as a `const`: the globally unique workflow `(name,
//! version)` pair. Resolve-time metadata (`specify-floor`, build
//! inputs, platforms) remains separate — identity never travels in the
//! WIT `metadata` answer.

/// The immutable `(name, version)` identity a catalog entry provides.
///
/// Published adapters set the exact package version, normally from
/// `env!("CARGO_PKG_VERSION")`. Unpublished mock/probe adapters may
/// use a development placeholder version; they remain bare-only
/// identities for pin matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AdapterIdentity {
    /// Kebab-case adapter name, globally unique across axes for
    /// published adapters.
    pub name: &'static str,
    /// Exact SemVer version string.
    pub version: &'static str,
}
