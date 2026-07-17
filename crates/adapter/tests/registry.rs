//! The prose lookup helpers over a sorted doc table.

use adapter::registry::{Doc, body, find};

/// A sorted table, as the `prose` codegen emits.
static DOCS: &[Doc] = &[
    Doc {
        path: "prompts/build.md",
        body: "# build",
    },
    Doc {
        path: "prompts/guidance.md",
        body: "# guidance",
    },
    Doc {
        path: "references/verifier.md",
        body: "# verifier",
    },
];

#[test]
fn find_by_path() {
    assert_eq!(find(DOCS, "prompts/guidance.md").map(|doc| doc.body), Some("# guidance"));
    assert_eq!(find(DOCS, "references/verifier.md").map(|doc| doc.body), Some("# verifier"));
    assert!(find(DOCS, "prompts/missing.md").is_none());
}

#[test]
fn body_lookup() {
    assert_eq!(body(DOCS, "prompts/build.md"), "# build");
}

#[test]
#[should_panic(expected = "document `prompts/missing.md` is not embedded")]
fn body_miss_panics() {
    let _ = body(DOCS, "prompts/missing.md");
}
