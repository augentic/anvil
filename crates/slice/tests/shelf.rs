//! The synthesis reference shelf (RFC-96 D9): the embedded corpus is
//! listable and readable over the MCP server surface, misses fail
//! typed, and the shelf identity matches the grant name.

use omnia_guest::mcp::{Content, McpServer as _};
use serde_json::json;
use slice::shelf::{PATH, SERVER, Shelf};

fn text(content: &[Content]) -> &str {
    match content.first().expect("one content block") {
        Content::Text { text } => text,
        other => panic!("unexpected content block: {other:?}"),
    }
}

#[test]
fn identity() {
    assert_eq!(PATH, "/mcp/engine/synthesis");
    let info = Shelf.info();
    assert_eq!(info.name, SERVER, "handshake identity matches the grant name");
}

// Every playbook document is listed, resourced, and readable in full;
// tool and resource reads return the same bodies.
#[test]
fn list_and_read() {
    let listed = Shelf.call_tool("list_docs", &json!({})).expect("list_docs");
    let paths: Vec<String> =
        serde_json::from_str(text(&listed.content)).expect("list_docs answers a JSON array");
    for expected in [
        "synthesize.md",
        "synthesis/substeps.md",
        "synthesis/boundary.md",
        "synthesis/requirement-block.md",
        "synthesis/authority.md",
        "synthesis/claim-reconciliation.md",
        "synthesis/tags.md",
        "synthesis/decisions.md",
        "synthesis/spec-format.md",
    ] {
        assert!(paths.contains(&expected.to_string()), "`{expected}` is shelved");
        let read =
            Shelf.call_tool("read_doc", &json!({ "path": expected })).expect("read_doc resolves");
        assert!(!text(&read.content).is_empty(), "`{expected}` reads non-empty");
        let resource = Shelf.read_resource(&format!("doc://{expected}")).expect("resource");
        assert_eq!(resource.text, text(&read.content), "tool and resource bodies agree");
    }
    assert_eq!(Shelf.resources().len(), paths.len(), "one resource per listed document");
}

#[test]
fn misses_fail_typed() {
    Shelf.call_tool("read_doc", &json!({ "path": "synthesis/missing.md" })).unwrap_err();
    Shelf.call_tool("unknown_tool", &json!({})).unwrap_err();
    Shelf.read_resource("doc://synthesis/missing.md").unwrap_err();
}
