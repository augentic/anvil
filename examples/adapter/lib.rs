//! Mock greeting source adapter.

#[cfg(target_arch = "wasm32")]
mod guest {
    emery_adapter::source!(crate::Adapter);
}

use emery_adapter::answers::{evidence_schema, evidence_tail};
use emery_adapter::registry::{self, Doc};
use emery_adapter::types::{Context, Error, Evidence, SourceContent, SourceInput, SourceMetadata};
use emery_adapter::{Model, SourceAdapter, repaired};

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

    fn metadata() -> SourceMetadata {
        SourceMetadata { emery_floor: None }
    }

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
            content = content_note(input)?,
        );
        let schema = evidence_schema();
        repaired(model, ctx, system, user, "evidence", &schema, evidence_tail).await
    }
}

fn content_note(input: &SourceInput) -> Result<String, Error> {
    match &input.content {
        SourceContent::Value(value) => {
            if value.trim().is_empty() {
                return Err(Error::InvalidRequest("the bound greeting brief is empty".to_string()));
            }
            Ok(format!(
                "The bound material is this inline value; no `$SOURCE_DIR` is lent:\n\n{value}\n\n\
                 Nothing else is reachable; extract works only from this value."
            ))
        }
        SourceContent::Workspace(view) => Ok(format!(
            "`$SOURCE_DIR` is the read-only view at `{}` — the greeting tree the \
             prompt walks. Prefer the bound tree; fall back to `references/greeting.md` \
             when the tree does not state a greeting. Nothing outside it is reachable; \
             extract mines only this source.",
            view.root
        )),
    }
}
