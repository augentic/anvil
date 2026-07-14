//! Prompt pinning: canonicalize recorded judgment requests and gate
//! them against committed goldens.
//!
//! The retired replay fixtures pinned the assembled prompts implicitly
//! (the canonical request was the fixture key). The scripted double
//! answers regardless of the request, so representative suites restore
//! that signal explicitly: canonicalize the [`Harness`] request log
//! and compare it to a committed golden, regenerated with
//! `REGENERATE_GOLDENS=1`. Ordered-log semantics compose with repair
//! loops, which a request-keyed store could not express.
//!
//! [`Harness`]: crate::model::Harness

use std::path::Path;

use omnia_guest::model::{Format, Request, Role, Tool};
use serde_json::{Value, json};

/// Assert the recorded request log matches the golden at `path`.
///
/// `REGENERATE_GOLDENS=1` rewrites the golden instead; `git diff` it
/// and review the prompt-side changes like any other golden.
///
/// # Panics
///
/// Panics on a golden mismatch, an unreadable golden, or a failed
/// regeneration write.
pub fn assert_requests(path: &Path, requests: &[Request]) {
    let log: Vec<Value> = requests.iter().map(canonical_request).collect();
    let mut actual = serde_json::to_string_pretty(&log).expect("canonical log serialises");
    actual.push('\n');
    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        std::fs::create_dir_all(path.parent().expect("goldens dir")).expect("create goldens dir");
        std::fs::write(path, &actual).expect("regenerate request golden");
    }
    let expected = std::fs::read_to_string(path).unwrap_or_else(|err| {
        panic!(
            "read request golden {} (regenerate with REGENERATE_GOLDENS=1): {err}",
            path.display()
        )
    });
    assert_eq!(actual, expected, "request golden mismatch: {}", path.display());
}

// The canonical JSON one request reduces to — the prompt-affecting
// fields only, mirroring the shape the replay fixture key carried.
fn canonical_request(request: &Request) -> Value {
    json!({
        "model": request.model,
        "system": request.system,
        "messages": request.messages.iter().map(|message| json!({
            "role": role(message.role),
            "content": message.content,
        })).collect::<Vec<_>>(),
        "generation": request.generation.as_ref().map(|generation| json!({
            "temperature": generation.temperature,
            "top_p": generation.top_p,
            "max_tokens": generation.max_tokens,
            "stop": generation.stop,
            "seed": generation.seed,
            "effort": generation.effort.map(|effort| format!("{effort:?}").to_lowercase()),
        })),
        "format": format_value(&request.format),
        "tools": request.tools.iter().map(tool_value).collect::<Vec<_>>(),
        "references": request.references,
        "verify": request.verify,
        "lend-workspace": request.lend_workspace,
    })
}

const fn role(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

fn format_value(format: &Format) -> Value {
    match format {
        Format::Text => json!({ "kind": "text" }),
        Format::Json => json!({ "kind": "json" }),
        Format::Schema(spec) => json!({
            "kind": "schema",
            "schema": { "name": spec.name, "schema": spec.schema },
        }),
    }
}

fn tool_value(tool: &Tool) -> Value {
    match tool {
        Tool::Function(function) => json!({
            "function": {
                "name": function.name,
                "description": function.description,
                "parameters": function.parameters,
            },
        }),
        Tool::Mcp(mcp) => json!({
            "mcp": { "name": mcp.name, "tools": mcp.tools, "url": mcp.url },
        }),
    }
}
