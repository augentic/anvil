//! WIT bindings
//!
//! The generated Rust bindings for the `source-adapter` WIT world, plus the
//! conversions between the generated wire records and the contract types the
//! rest of the workspace uses.
//!
//! Both directions come from one generation: adapters export through it via
//! the SDK's `source!` macro, and the engine guest calls into it through
//! [`import`]. A single generation guarantees the two sides agree on the wire
//! shape by construction.
//!
//! The WIT `error` variant lives here alone: an adapter's `omnia_guest::Error`
//! is lowered onto it on export, and [`import::extract`] lifts it back into
//! the same classes, so neither side of the seam names the wire variant.

mod generated {
    #![allow(
        missing_docs,
        unsafe_code,
        clippy::pedantic,
        clippy::nursery,
        reason = "wit-bindgen generated bindings are not hand-maintained; the generated code cannot carry this workspace's lint posture"
    )]

    wit_bindgen::generate!({
        world: "source-adapter",
        path: "../../wit",
        // Only judgment operations are async.
        generate_all,
        pub_export_macro: true,
    });
}

pub use generated::exports::emery::adapter::source::*;
pub use generated::*;

impl From<crate::types::SourceMetadata> for AdapterMetadata {
    fn from(metadata: crate::types::SourceMetadata) -> Self {
        Self {
            emery_version: metadata.emery_version,
        }
    }
}

impl From<crate::types::SourceContent> for Content {
    fn from(content: crate::types::SourceContent) -> Self {
        match content {
            crate::types::SourceContent::Workspace(root) => Self::Workspace(root),
            crate::types::SourceContent::Value(value) => Self::Value(value),
        }
    }
}

impl From<Content> for crate::types::SourceContent {
    fn from(content: Content) -> Self {
        match content {
            Content::Workspace(root) => Self::Workspace(root),
            Content::Value(value) => Self::Value(value),
        }
    }
}

impl From<crate::types::SourceInput> for Input {
    fn from(input: crate::types::SourceInput) -> Self {
        Self {
            key: input.key,
            content: input.content.into(),
        }
    }
}

impl From<Input> for crate::types::SourceInput {
    fn from(input: Input) -> Self {
        Self {
            key: input.key,
            content: input.content.into(),
        }
    }
}

impl From<crate::types::Authority> for Authority {
    fn from(authority: crate::types::Authority) -> Self {
        match authority {
            crate::types::Authority::Intent => Self::Intent,
            crate::types::Authority::Documentation => Self::Documentation,
            crate::types::Authority::Behaviour => Self::Behaviour,
        }
    }
}

impl From<crate::types::ClaimKind> for ClaimKind {
    fn from(kind: crate::types::ClaimKind) -> Self {
        match kind {
            crate::types::ClaimKind::Intent => Self::Intent,
            crate::types::ClaimKind::Requirement => Self::Requirement,
            crate::types::ClaimKind::Criterion => Self::Criterion,
            crate::types::ClaimKind::Decision => Self::Decision,
            crate::types::ClaimKind::Section => Self::Section,
            crate::types::ClaimKind::Diagram => Self::Diagram,
            crate::types::ClaimKind::Contract => Self::Contract,
            crate::types::ClaimKind::Example => Self::Example,
            crate::types::ClaimKind::Excerpt => Self::Excerpt,
            crate::types::ClaimKind::Type => Self::Type,
            crate::types::ClaimKind::Call => Self::Call,
            crate::types::ClaimKind::Region => Self::Region,
            crate::types::ClaimKind::Container => Self::Container,
            crate::types::ClaimKind::Leaf => Self::Leaf,
        }
    }
}

impl From<crate::types::Backing> for Backing {
    fn from(backing: crate::types::Backing) -> Self {
        match backing {
            crate::types::Backing::Payload(payload) => Self::Payload(payload),
            crate::types::Backing::Path(path) => Self::Path(path),
        }
    }
}

impl From<crate::types::Claim> for Claim {
    fn from(claim: crate::types::Claim) -> Self {
        // Open body fields ride the wire as canonical JSON text (A8);
        // `serde_json::Value` always encodes.
        let extras =
            claim.extras.into_iter().map(|(key, value)| (key, value.to_string())).collect();
        Self {
            kind: claim.kind.into(),
            id: claim.id,
            path: claim.path,
            synopsis: claim.synopsis,
            backing: claim.backing.map(Into::into),
            extras,
        }
    }
}

impl From<crate::types::Evidence> for Evidence {
    fn from(evidence: crate::types::Evidence) -> Self {
        Self {
            authority: evidence.authority.into(),
            claims: evidence.claims.into_iter().map(Into::into).collect(),
        }
    }
}

// The wire carries the description alone: a refusal of the input lowers to
// `invalid-request`, every other class to `internal`, and the lift restores
// the class. `io` is accepted on lift but never produced.
impl From<omnia_guest::Error> for Error {
    fn from(error: omnia_guest::Error) -> Self {
        let description = error.description();
        match error {
            omnia_guest::Error::BadRequest { .. } | omnia_guest::Error::NotFound { .. } => {
                Self::InvalidRequest(description)
            }
            omnia_guest::Error::ServerError { .. } | omnia_guest::Error::BadGateway { .. } => {
                Self::Internal(description)
            }
        }
    }
}

/// Typed wrappers for source WIT imports.
pub mod import {
    use omnia_guest::{Error, bad_gateway, bad_request};

    use super::generated::emery::adapter::source as wire;
    use crate::types;

    /// Returns resolve-time metadata for `id`.
    #[must_use]
    pub fn metadata(id: &str) -> types::SourceMetadata {
        let record = wire::metadata(id);
        types::SourceMetadata {
            emery_version: record.emery_version,
        }
    }

    /// Dispatches `extract` to `id`.
    ///
    /// Open extras parse from canonical JSON fail-closed (A8); invalid
    /// values return a typed error rather than dropping the key.
    ///
    /// # Errors
    ///
    /// An adapter refusing its input is `BadRequest`; any other adapter
    /// failure, or an extra that is not canonical JSON, is `BadGateway`.
    pub async fn extract(id: &str, input: &types::SourceInput) -> Result<types::Evidence, Error> {
        let answer =
            wire::extract(id.to_string(), input.clone().into()).await.map_err(|err| match err {
                wire::Error::InvalidRequest(detail) => bad_request!("source `{id}`: {detail}"),
                wire::Error::Io(detail) | wire::Error::Internal(detail) => {
                    bad_gateway!("source `{id}`: {detail}")
                }
            })?;
        evidence(answer).map_err(|detail| bad_gateway!("source `{id}`: {detail}"))
    }

    impl From<types::SourceContent> for wire::Content {
        fn from(content: types::SourceContent) -> Self {
            match content {
                types::SourceContent::Workspace(root) => Self::Workspace(root),
                types::SourceContent::Value(value) => Self::Value(value),
            }
        }
    }

    impl From<types::SourceInput> for wire::Input {
        fn from(input: types::SourceInput) -> Self {
            Self {
                key: input.key,
                content: input.content.into(),
            }
        }
    }

    impl From<wire::Authority> for types::Authority {
        fn from(authority: wire::Authority) -> Self {
            match authority {
                wire::Authority::Intent => Self::Intent,
                wire::Authority::Documentation => Self::Documentation,
                wire::Authority::Behaviour => Self::Behaviour,
            }
        }
    }

    impl From<wire::ClaimKind> for types::ClaimKind {
        fn from(kind: wire::ClaimKind) -> Self {
            match kind {
                wire::ClaimKind::Intent => Self::Intent,
                wire::ClaimKind::Requirement => Self::Requirement,
                wire::ClaimKind::Criterion => Self::Criterion,
                wire::ClaimKind::Decision => Self::Decision,
                wire::ClaimKind::Section => Self::Section,
                wire::ClaimKind::Diagram => Self::Diagram,
                wire::ClaimKind::Contract => Self::Contract,
                wire::ClaimKind::Example => Self::Example,
                wire::ClaimKind::Excerpt => Self::Excerpt,
                wire::ClaimKind::Type => Self::Type,
                wire::ClaimKind::Call => Self::Call,
                wire::ClaimKind::Region => Self::Region,
                wire::ClaimKind::Container => Self::Container,
                wire::ClaimKind::Leaf => Self::Leaf,
            }
        }
    }

    impl From<wire::Backing> for types::Backing {
        fn from(backing: wire::Backing) -> Self {
            match backing {
                wire::Backing::Payload(payload) => Self::Payload(payload),
                wire::Backing::Path(path) => Self::Path(path),
            }
        }
    }

    fn evidence(evidence: wire::Evidence) -> Result<types::Evidence, String> {
        Ok(types::Evidence {
            authority: evidence.authority.into(),
            claims: evidence.claims.into_iter().map(claim).collect::<Result<_, _>>()?,
        })
    }

    fn claim(claim: wire::Claim) -> Result<types::Claim, String> {
        let mut extras = serde_json::Map::new();
        for (key, encoded) in claim.extras {
            let value = serde_json::from_str(&encoded)
                .map_err(|err| format!("extra `{key}` is not canonical JSON ({err}): {encoded}"))?;
            extras.insert(key, value);
        }
        Ok(types::Claim {
            kind: claim.kind.into(),
            id: claim.id,
            path: claim.path,
            synopsis: claim.synopsis,
            backing: claim.backing.map(Into::into),
            extras,
        })
    }
}
