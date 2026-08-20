//! `source-adapter` WIT bindings: the `source!` export macro plus the
//! engine guest's [`import`] seam wrappers. One `wit_bindgen::generate!`
//! here; leaf crates wire a [`crate::Source`] with `emery_adapter::source!(…)`.

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
        // Judgment ops are async; `metadata` is sync.
        generate_all,
        pub_export_macro: true,
    });
}

pub use generated::exports::emery::adapter::source::*;
pub use generated::*;

impl From<crate::seam::SourceMetadata> for AdapterMetadata {
    fn from(metadata: crate::seam::SourceMetadata) -> Self {
        Self {
            emery_floor: metadata.emery_floor,
        }
    }
}

impl From<crate::seam::SourceWorkspace> for Workspace {
    fn from(view: crate::seam::SourceWorkspace) -> Self {
        Self {
            id: view.id,
            root: view.root,
        }
    }
}

impl From<Workspace> for crate::seam::SourceWorkspace {
    fn from(view: Workspace) -> Self {
        Self {
            id: view.id,
            root: view.root,
        }
    }
}

impl From<crate::seam::SourceContent> for Content {
    fn from(content: crate::seam::SourceContent) -> Self {
        match content {
            crate::seam::SourceContent::Workspace(view) => Self::Workspace(view.into()),
            crate::seam::SourceContent::Value(value) => Self::Value(value),
        }
    }
}

impl From<Content> for crate::seam::SourceContent {
    fn from(content: Content) -> Self {
        match content {
            Content::Workspace(view) => Self::Workspace(view.into()),
            Content::Value(value) => Self::Value(value),
        }
    }
}

impl From<crate::seam::SourceInput> for Input {
    fn from(input: crate::seam::SourceInput) -> Self {
        Self {
            key: input.key,
            content: input.content.into(),
        }
    }
}

impl From<Input> for crate::seam::SourceInput {
    fn from(input: Input) -> Self {
        Self {
            key: input.key,
            content: input.content.into(),
        }
    }
}

impl From<crate::seam::Authority> for Authority {
    fn from(authority: crate::seam::Authority) -> Self {
        match authority {
            crate::seam::Authority::Intent => Self::Intent,
            crate::seam::Authority::Documentation => Self::Documentation,
            crate::seam::Authority::Behaviour => Self::Behaviour,
        }
    }
}

impl From<crate::seam::ClaimKind> for ClaimKind {
    fn from(kind: crate::seam::ClaimKind) -> Self {
        match kind {
            crate::seam::ClaimKind::Intent => Self::Intent,
            crate::seam::ClaimKind::Requirement => Self::Requirement,
            crate::seam::ClaimKind::Criterion => Self::Criterion,
            crate::seam::ClaimKind::Decision => Self::Decision,
            crate::seam::ClaimKind::Section => Self::Section,
            crate::seam::ClaimKind::Diagram => Self::Diagram,
            crate::seam::ClaimKind::Contract => Self::Contract,
            crate::seam::ClaimKind::Example => Self::Example,
            crate::seam::ClaimKind::Excerpt => Self::Excerpt,
            crate::seam::ClaimKind::Type => Self::Type,
            crate::seam::ClaimKind::Call => Self::Call,
            crate::seam::ClaimKind::Region => Self::Region,
            crate::seam::ClaimKind::Container => Self::Container,
            crate::seam::ClaimKind::Leaf => Self::Leaf,
        }
    }
}

impl From<crate::seam::Backing> for Backing {
    fn from(backing: crate::seam::Backing) -> Self {
        match backing {
            crate::seam::Backing::Payload(payload) => Self::Payload(payload),
            crate::seam::Backing::Path(path) => Self::Path(path),
        }
    }
}

impl From<crate::seam::Claim> for Claim {
    fn from(claim: crate::seam::Claim) -> Self {
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

impl From<crate::seam::Evidence> for Evidence {
    fn from(evidence: crate::seam::Evidence) -> Self {
        Self {
            authority: evidence.authority.into(),
            claims: evidence.claims.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::seam::Error> for Error {
    fn from(error: crate::seam::Error) -> Self {
        match error {
            crate::seam::Error::InvalidRequest(detail) => Self::InvalidRequest(detail),
            crate::seam::Error::Io(detail) => Self::Io(detail),
            crate::seam::Error::Internal(detail) => Self::Internal(detail),
        }
    }
}

/// Import-side seam surface: the engine guest dispatches the linked
/// source component through these wrappers, staying seam-typed.
pub mod import {
    use super::generated::emery::adapter::source as wire;
    use crate::seam;

    /// Import dispatch failure: the operation's seam error, or an
    /// evidence extra whose wire value is not canonical JSON.
    #[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
    pub enum Error {
        /// The dispatched operation failed across the seam.
        #[error(transparent)]
        Seam(#[from] seam::Error),
        /// An open extra failed the canonical JSON parse (A8).
        #[error("extra `{key}` is not canonical JSON ({detail}): {encoded}")]
        Extras {
            /// The extra's key.
            key: String,
            /// The parse failure.
            detail: String,
            /// The value as it crossed the wire.
            encoded: String,
        },
    }

    /// Resolve-time metadata of the source component routed by `id`.
    #[must_use]
    pub fn metadata(id: &str) -> seam::SourceMetadata {
        let record = wire::metadata(id);
        seam::SourceMetadata {
            emery_floor: record.emery_floor,
        }
    }

    /// Dispatch `extract` on the source component routed by `id`.
    ///
    /// The answer lifts onto the seam DTOs, parsing each open extra's
    /// canonical JSON encoding fail-closed (A8): a value that does not
    /// parse is a typed error, never a dropped key.
    ///
    /// # Errors
    ///
    /// The seam failure, or the A8 extras refusal.
    pub async fn extract(id: &str, input: &seam::SourceInput) -> Result<seam::Evidence, Error> {
        let answer = wire::extract(id.to_string(), input.clone().into())
            .await
            .map_err(|err| Error::Seam(err.into()))?;
        answer.try_into()
    }

    impl From<seam::SourceWorkspace> for wire::Workspace {
        fn from(view: seam::SourceWorkspace) -> Self {
            Self {
                id: view.id,
                root: view.root,
            }
        }
    }

    impl From<seam::SourceContent> for wire::Content {
        fn from(content: seam::SourceContent) -> Self {
            match content {
                seam::SourceContent::Workspace(view) => Self::Workspace(view.into()),
                seam::SourceContent::Value(value) => Self::Value(value),
            }
        }
    }

    impl From<seam::SourceInput> for wire::Input {
        fn from(input: seam::SourceInput) -> Self {
            Self {
                key: input.key,
                content: input.content.into(),
            }
        }
    }

    impl From<wire::Error> for seam::Error {
        fn from(error: wire::Error) -> Self {
            match error {
                wire::Error::InvalidRequest(detail) => Self::InvalidRequest(detail),
                wire::Error::Io(detail) => Self::Io(detail),
                wire::Error::Internal(detail) => Self::Internal(detail),
            }
        }
    }

    impl From<wire::Authority> for seam::Authority {
        fn from(authority: wire::Authority) -> Self {
            match authority {
                wire::Authority::Intent => Self::Intent,
                wire::Authority::Documentation => Self::Documentation,
                wire::Authority::Behaviour => Self::Behaviour,
            }
        }
    }

    impl From<wire::ClaimKind> for seam::ClaimKind {
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

    impl From<wire::Backing> for seam::Backing {
        fn from(backing: wire::Backing) -> Self {
            match backing {
                wire::Backing::Payload(payload) => Self::Payload(payload),
                wire::Backing::Path(path) => Self::Path(path),
            }
        }
    }

    impl TryFrom<wire::Claim> for seam::Claim {
        type Error = Error;

        fn try_from(claim: wire::Claim) -> Result<Self, Error> {
            let mut extras = serde_json::Map::new();
            for (key, encoded) in claim.extras {
                let value = match serde_json::from_str(&encoded) {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(Error::Extras {
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

    impl TryFrom<wire::Evidence> for seam::Evidence {
        type Error = Error;

        fn try_from(evidence: wire::Evidence) -> Result<Self, Error> {
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

/// Map [`crate::Source::metadata`] onto the WIT record.
#[must_use]
pub fn dispatch_metadata<A: crate::Source>() -> AdapterMetadata {
    A::metadata().into()
}

/// # Errors
///
/// As the implementor's [`extract`](crate::Source::extract).
pub async fn dispatch_extract<A: crate::Source>(
    id: AdapterId, input: Input,
) -> Result<Evidence, Error> {
    let input = crate::seam::SourceInput::from(input);
    let ctx = source_ctx(&id, &input);
    A::extract(&crate::WasiModel, &ctx, &input).await.map(Into::into).map_err(Into::into)
}

fn source_ctx<'a>(id: &'a str, input: &'a crate::seam::SourceInput) -> crate::seam::Context<'a> {
    match &input.content {
        crate::seam::SourceContent::Workspace(view) => {
            crate::seam::Context::guest(id).lending(view.root.clone())
        }
        crate::seam::SourceContent::Value(_) => crate::seam::Context::guest(id).without_lend(),
    }
}

/// Wire a [`crate::Source`] implementor into the component exports.
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

        struct HttpGuest;
        $crate::wasip3::http::service::export!(HttpGuest);

        impl $crate::wasip3::exports::http::handler::Guest for HttpGuest {
            async fn handle(
                request: $crate::wasip3::http::types::Request,
            ) -> Result<
                $crate::wasip3::http::types::Response,
                $crate::wasip3::http::types::ErrorCode,
            > {
                let (name, version) = <$adapter as $crate::Source>::IDENTITY
                    .split_once('@')
                    .expect("IDENTITY is name@version");
                $crate::references::serve(
                    name,
                    version,
                    <$adapter as $crate::Source>::docs(),
                    request,
                )
                .await
            }
        }
    };
}
