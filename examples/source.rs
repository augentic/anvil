//! Deterministic mock source component for static runtime examples.
#![cfg(target_arch = "wasm32")]

use std::future::Future;

use emery_adapter::registry::Doc;
use emery_adapter::seam::{
    Authority, Backing, Claim, ClaimKind, Context, Error, Evidence, SourceInput, SourceMetadata,
};
use emery_adapter::{Model, SourceAdapter};

// This development-only identity must never match a release pin.
#[derive(Clone, Copy, Debug)]
struct Mock;

emery_adapter::source!(Mock);

const STATEMENT: &str = "GET /greeting returns the static string 'hello'.";
const DOCS: &[Doc] = &[Doc {
    path: "reference.md",
    body: "# Adapter Reference\n\nThe mock source adapter returns one deterministic requirement.\n",
}];

impl SourceAdapter for Mock {
    const IDENTITY: &str = "source@0.1.0";

    fn metadata() -> SourceMetadata {
        SourceMetadata { emery_floor: None }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    fn extract<P: Model>(
        _model: &P, _ctx: &Context<'_>, _input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send {
        let mut extras = serde_json::Map::new();
        extras.insert("statement".to_string(), serde_json::Value::String(STATEMENT.to_string()));
        std::future::ready(Ok(Evidence {
            authority: Authority::Documentation,
            claims: vec![Claim {
                kind: ClaimKind::Requirement,
                id: Some("greeting.behaviour".to_string()),
                path: None,
                synopsis: Some("Greeting behaviour".to_string()),
                backing: Some(Backing::Payload(STATEMENT.to_string())),
                extras,
            }],
        }))
    }
}
