//! Mock source adapter: extract calls the host model over a greeting brief.
#![cfg(target_arch = "wasm32")]

use emery_adapter::answers::{evidence_schema, evidence_tail};
use emery_adapter::registry::Doc;
use emery_adapter::types::{Context, Error, Evidence, SourceContent, SourceInput, SourceMetadata};
use emery_adapter::{Model, SourceAdapter, repaired};

// This development-only identity must never match a release pin.
#[derive(Clone, Copy, Debug)]
struct Mock;

emery_adapter::source!(Mock);

const EXTRACT: &str = r#"# source.extract

Emit one `Evidence` document from the bound greeting brief.

Read `reference.md` via `read_doc` before answering.

## Claim kinds

| Kind | Required body field | When to emit |
|---|---|---|
| `requirement` | `statement` | The one greeting behaviour the brief (or the reference) states. |

Do not invent a `criterion`. A `requirement` without `statement` fails the run closed.

## Output contract

```json
{
  "authority": "documentation",
  "claims": [
    {
      "kind": "requirement",
      "id": "greeting.behaviour",
      "statement": "GET /greeting returns the static string 'hello'."
    }
  ]
}
```

The caller persists the document; do not write it yourself.
"#;

const REFERENCE: &str = "\
# Greeting source

The mock source documents one behaviour: GET /greeting returns the \
static string 'hello'. Requirement ids stay `greeting.behaviour`.
";

const DOCS: &[Doc] = &[
    Doc {
        path: "prompts/extract.md",
        body: EXTRACT,
    },
    Doc {
        path: "reference.md",
        body: REFERENCE,
    },
];

impl SourceAdapter for Mock {
    const IDENTITY: &str = "source@0.1.0";

    fn metadata() -> SourceMetadata {
        SourceMetadata { emery_floor: None }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn extract<P: Model>(
        model: &P, ctx: &Context<'_>, input: &SourceInput,
    ) -> Result<Evidence, Error> {
        let system = EXTRACT.to_string();
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
            "`$SOURCE_DIR` is the read-only view at `{}`. Prefer the bound tree; \
             fall back to `reference.md` when the tree does not state a greeting.",
            view.root
        )),
    }
}
