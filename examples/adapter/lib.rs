//! Mock source adapter
//!
//! The smallest complete source adapter: it reads a greeting fixture and
//! asks the model to describe it as claims. It exists so the engine can be
//! exercised end to end on the live journey without depending on a
//! first-party adapter from the adapters repository, and so the SDK's wasm32
//! export side is linted by `make wasm`.
//!
//! It is also the reference shape for adapter authors: one `SourceAdapter`
//! implementation, an embedded prose tree, and a single `source!` export.

#[cfg(target_arch = "wasm32")]
mod guest {
    emery_adapter::source!(crate::Adapter);
}

use emery_adapter::types::{Context, Error, Evidence, SourceContent, SourceInput};
use emery_adapter::{Model, SourceAdapter, content_note, evidence};
use emery_prose::registry::{self, Doc};

static DOCS: &[Doc] = &[
    Doc {
        path: "prompts/extract.md",
        body: include_str!("prose/prompts/extract.md"),
    },
    Doc {
        path: "references/greeting.md",
        body: include_str!("prose/references/greeting.md"),
    },
];

/// Extracts the greeting fixture into structured claims.
#[derive(Clone, Copy, Debug)]
pub struct Adapter;

impl SourceAdapter for Adapter {
    // Development-only: must never match a release pin.
    const IDENTITY: &str = concat!("source@", env!("CARGO_PKG_VERSION"));

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn extract<P: Model>(
        model: &P, ctx: &Context<'_>, input: &SourceInput,
    ) -> Result<Evidence, Error> {
        let system = registry::body(DOCS, "prompts/extract.md").to_string();
        let user = format!(
            "Extract the claim set of the greeting source bound to adapter `{id}` \
             (source key `{key}`).\n\n\
             {content}\n\n\
             Answer with one JSON object matching the gated schema: the Evidence body \
             (`authority`, `claims`) the prompt describes. The caller persists the \
             document; do not write it yourself.",
            id = ctx.adapter_id,
            key = input.key,
            content = greeting_note(input)?,
        );
        evidence(model, ctx, system, user).await
    }
}

// The shared note, plus the greeting fallback and the empty-brief refusal.
fn greeting_note(input: &SourceInput) -> Result<String, Error> {
    match &input.content {
        SourceContent::Value(value) if value.trim().is_empty() => {
            Err(Error::InvalidRequest("the bound greeting brief is empty".to_string()))
        }
        SourceContent::Value(_) => Ok(content_note(input, "")),
        SourceContent::Workspace(_) => Ok(format!(
            "{} Prefer the bound tree; fall back to `references/greeting.md` when the tree \
             does not state a greeting.",
            content_note(input, "the greeting tree")
        )),
    }
}
