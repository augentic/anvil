//! `source-adapter` WIT bindings and export macro.
//!
//! [`crate::source!`] wires a [`crate::SourceAdapter`] to the component exports.

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
            emery_floor: metadata.emery_floor,
        }
    }
}

impl From<crate::types::SourceWorkspace> for Workspace {
    fn from(view: crate::types::SourceWorkspace) -> Self {
        Self {
            id: view.id,
            root: view.root,
        }
    }
}

impl From<Workspace> for crate::types::SourceWorkspace {
    fn from(view: Workspace) -> Self {
        Self {
            id: view.id,
            root: view.root,
        }
    }
}

impl From<crate::types::SourceContent> for Content {
    fn from(content: crate::types::SourceContent) -> Self {
        match content {
            crate::types::SourceContent::Workspace(view) => Self::Workspace(view.into()),
            crate::types::SourceContent::Value(value) => Self::Value(value),
        }
    }
}

impl From<Content> for crate::types::SourceContent {
    fn from(content: Content) -> Self {
        match content {
            Content::Workspace(view) => Self::Workspace(view.into()),
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

impl From<crate::types::Error> for Error {
    fn from(error: crate::types::Error) -> Self {
        match error {
            crate::types::Error::InvalidRequest(detail) => Self::InvalidRequest(detail),
            crate::types::Error::Io(detail) => Self::Io(detail),
            crate::types::Error::Internal(detail) => Self::Internal(detail),
        }
    }
}

/// Typed wrappers for source WIT imports.
pub mod import {
    use super::generated::emery::adapter::source as wire;
    use crate::dispatch::DispatchError;
    use crate::types;

    /// Returns resolve-time metadata for `id`.
    #[must_use]
    pub fn metadata(id: &str) -> types::SourceMetadata {
        let record = wire::metadata(id);
        types::SourceMetadata {
            emery_floor: record.emery_floor,
        }
    }

    /// Dispatches `extract` to `id`.
    ///
    /// Open extras parse from canonical JSON fail-closed (A8); invalid
    /// values return a typed error rather than dropping the key.
    ///
    /// # Errors
    ///
    /// Returns the adapter call failure or A8 extras refusal.
    pub async fn extract(
        id: &str, input: &types::SourceInput,
    ) -> Result<types::Evidence, DispatchError> {
        let answer = wire::extract(id.to_string(), input.clone().into())
            .await
            .map_err(|err| DispatchError::Call(err.into()))?;
        answer.try_into()
    }

    impl From<types::SourceWorkspace> for wire::Workspace {
        fn from(view: types::SourceWorkspace) -> Self {
            Self {
                id: view.id,
                root: view.root,
            }
        }
    }

    impl From<types::SourceContent> for wire::Content {
        fn from(content: types::SourceContent) -> Self {
            match content {
                types::SourceContent::Workspace(view) => Self::Workspace(view.into()),
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

    impl From<wire::Error> for types::Error {
        fn from(error: wire::Error) -> Self {
            match error {
                wire::Error::InvalidRequest(detail) => Self::InvalidRequest(detail),
                wire::Error::Io(detail) => Self::Io(detail),
                wire::Error::Internal(detail) => Self::Internal(detail),
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

    impl TryFrom<wire::Claim> for types::Claim {
        type Error = DispatchError;

        fn try_from(claim: wire::Claim) -> Result<Self, DispatchError> {
            let mut extras = serde_json::Map::new();
            for (key, encoded) in claim.extras {
                let value = match serde_json::from_str(&encoded) {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(DispatchError::Extras {
                            key,
                            detail: err.to_string(),
                            encoded,
                        });
                    }
                };
                extras.insert(key, value);
            }
            Ok(Self {
                kind: claim.kind.into(),
                id: claim.id,
                path: claim.path,
                synopsis: claim.synopsis,
                backing: claim.backing.map(Into::into),
                extras,
            })
        }
    }

    impl TryFrom<wire::Evidence> for types::Evidence {
        type Error = DispatchError;

        fn try_from(evidence: wire::Evidence) -> Result<Self, DispatchError> {
            Ok(Self {
                authority: evidence.authority.into(),
                claims: evidence
                    .claims
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()?,
            })
        }
    }
}

/// Maps adapter metadata to its WIT record.
#[must_use]
pub fn dispatch_metadata<A: crate::SourceAdapter>() -> AdapterMetadata {
    A::metadata().into()
}

/// Dispatches extract through adapter `A`.
///
/// # Errors
///
/// Returns the adapter's extract error.
pub async fn dispatch_extract<A: crate::SourceAdapter>(
    id: AdapterId, input: Input,
) -> Result<Evidence, Error> {
    let input = crate::types::SourceInput::from(input);
    let ctx = source_ctx::<A>(&id, &input);
    A::extract(&crate::WasiModel, &ctx, &input).await.map(Into::into).map_err(Into::into)
}

fn source_ctx<'a, A: crate::SourceAdapter>(
    id: &'a str, input: &'a crate::types::SourceInput,
) -> crate::types::Context<'a> {
    let ctx = crate::types::Context::guest(id).with_docs(A::docs());
    match &input.content {
        crate::types::SourceContent::Workspace(view) => ctx.lending(view.root.clone()),
        crate::types::SourceContent::Value(_) => ctx.without_lend(),
    }
}

/// Wires a [`crate::SourceAdapter`] into component exports.
///
/// ```ignore
/// emery_adapter::source!(crate::Captures);
/// ```
#[macro_export]
macro_rules! source {
    ($adapter:ty) => {
        struct Adapter;
        $crate::source::export!(Adapter with_types_in $crate::source);

        impl $crate::source::Guest for Adapter {
            fn metadata(
                _id: $crate::source::AdapterId,
            ) -> $crate::source::AdapterMetadata {
                $crate::source::dispatch_metadata::<$adapter>()
            }

            async fn extract(
                id: $crate::source::AdapterId,
                input: $crate::source::Input,
            ) -> Result<$crate::source::Evidence, $crate::source::Error> {
                $crate::source::dispatch_extract::<$adapter>(id, input).await
            }
        }
    };
}
