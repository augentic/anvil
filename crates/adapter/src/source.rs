//! `source-adapter` WIT bindings and the `source!` export macro.
//!
//! One `wit_bindgen::generate!` here; leaf crates wire a [`crate::Source`]
//! implementor with `adapter::source!(…)`.

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

impl From<crate::seam::Lead> for Lead {
    fn from(lead: crate::seam::Lead) -> Self {
        Self {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
            parent: lead.parent,
            focus: lead.focus,
        }
    }
}

impl From<Lead> for crate::seam::Lead {
    fn from(lead: Lead) -> Self {
        Self {
            lead: lead.lead,
            synopsis: lead.synopsis,
            topics: lead.topics,
            parent: lead.parent,
            focus: lead.focus,
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
            focus: input.focus.map(Into::into),
        }
    }
}

impl From<Input> for crate::seam::SourceInput {
    fn from(input: Input) -> Self {
        Self {
            key: input.key,
            content: input.content.into(),
            focus: input.focus.map(Into::into),
        }
    }
}

impl From<crate::seam::SurveyResult> for SurveyResult {
    fn from(result: crate::seam::SurveyResult) -> Self {
        Self {
            leads: result.leads.into_iter().map(Into::into).collect(),
            children: result.children.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<SurveyResult> for crate::seam::SurveyResult {
    fn from(result: SurveyResult) -> Self {
        Self {
            leads: result.leads.into_iter().map(Into::into).collect(),
            children: result.children.into_iter().map(Into::into).collect(),
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
        Self {
            kind: claim.kind.into(),
            id: claim.id,
            path: claim.path,
            synopsis: claim.synopsis,
            backing: claim.backing.map(Into::into),
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

/// Map [`crate::Source::metadata`] onto the WIT record.
#[must_use]
pub fn dispatch_metadata<A: crate::Source>() -> AdapterMetadata {
    A::metadata().into()
}

/// # Errors
///
/// As the implementor's [`survey`](crate::Source::survey).
pub async fn dispatch_survey<A: crate::Source>(
    id: AdapterId, input: Input,
) -> Result<SurveyResult, Error> {
    let input = crate::seam::SourceInput::from(input);
    let ctx = source_ctx(&id, &input);
    A::survey(&crate::WasiModel, &ctx, &input).await.map(Into::into).map_err(Into::into)
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
/// adapter::source!(crate::Captures);
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

            async fn survey(
                id: $crate::source::AdapterId,
                input: $crate::source::Input,
            ) -> Result<$crate::source::SurveyResult, $crate::source::Error> {
                $crate::source::dispatch_survey::<$adapter>(id, input).await
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
                $crate::references::serve(
                    <$adapter as $crate::Source>::IDENTITY.name,
                    <$adapter as $crate::Source>::IDENTITY.version,
                    <$adapter as $crate::Source>::docs(),
                    request,
                )
                .await
            }
        }
    };
}
