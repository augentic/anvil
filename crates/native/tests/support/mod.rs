//! Shared probe implementors for the native-host test binaries.
//!
//! One [`Probe`] source registered as `mock`, the [`Floored`] source
//! carrying a metadata floor, the [`Pinned`] source with a published
//! identity version, and the [`BadVersion`] source with a non-SemVer
//! identity — enough surface for the catalog, provider, and command
//! suites without a dependency on any concrete adapter crate.

#![allow(dead_code, reason = "each test binary uses a subset of the shared support surface")]

use adapter::registry::Doc;
use adapter::seam::{Context, Error, Evidence, SourceInput, SourceMetadata};
use adapter::{AdapterIdentity, Source};
use omnia_guest::Model;

/// The single embedded reference document every probe serves.
pub const DOCS: &[Doc] = &[Doc {
    path: "prompts/guidance.md",
    body: "mock guidance",
}];

/// Source probe implementor, registered as `mock`.
///
/// `extract` fails with a typed error naming its input key, so the
/// suites can assert dispatch threading.
#[derive(Clone, Copy, Debug)]
pub struct Probe;

impl Source for Probe {
    const IDENTITY: AdapterIdentity = AdapterIdentity {
        name: "mock",
        version: "0.0.0",
    };

    fn metadata() -> SourceMetadata {
        SourceMetadata { emery_floor: None }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn extract<P: Model>(
        _model: &P, _ctx: &Context<'_>, input: &SourceInput,
    ) -> Result<Evidence, Error> {
        Err(Error::Internal(format!("no evidence for {}", input.key)))
    }
}

/// A source declaring an `emery` compatibility floor, for
/// metadata-projection asserts.
#[derive(Clone, Copy, Debug)]
pub struct Floored;

impl Source for Floored {
    const IDENTITY: AdapterIdentity = AdapterIdentity {
        name: "floored",
        version: "0.0.0",
    };

    fn metadata() -> SourceMetadata {
        SourceMetadata {
            emery_floor: Some("9.9.9".to_string()),
        }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn extract<P: Model>(
        _model: &P, _ctx: &Context<'_>, input: &SourceInput,
    ) -> Result<Evidence, Error> {
        Err(Error::Internal(format!("no evidence for {}", input.key)))
    }
}

/// A source carrying a published (non-placeholder) identity version,
/// for exact-pin matching asserts.
#[derive(Clone, Copy, Debug)]
pub struct Pinned;

impl Source for Pinned {
    const IDENTITY: AdapterIdentity = AdapterIdentity {
        name: "pinned",
        version: "1.2.3",
    };

    fn metadata() -> SourceMetadata {
        SourceMetadata { emery_floor: None }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn extract<P: Model>(
        _model: &P, _ctx: &Context<'_>, input: &SourceInput,
    ) -> Result<Evidence, Error> {
        Err(Error::Internal(format!("no evidence for {}", input.key)))
    }
}

/// A source whose compile-time identity version is not SemVer, for
/// catalog-validation asserts.
#[derive(Clone, Copy, Debug)]
pub struct BadVersion;

impl Source for BadVersion {
    const IDENTITY: AdapterIdentity = AdapterIdentity {
        name: "bad-version",
        version: "dev",
    };

    fn metadata() -> SourceMetadata {
        SourceMetadata { emery_floor: None }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn extract<P: Model>(
        _model: &P, _ctx: &Context<'_>, input: &SourceInput,
    ) -> Result<Evidence, Error> {
        Err(Error::Internal(format!("no evidence for {}", input.key)))
    }
}
