//! The fail-closed spec AST (A17, ADR-0009 §4) at its public surface:
//! the reviewable set parses; everything else is a typed error.

use artifacts::spec::ast::{Status, Tag, parse};

const REVIEWABLE: &str = "\
# Session handling

Scope: the session lifecycle.

### Requirement: Sessions expire after inactivity [divergence]

ID: REQ-001
Sources: [intent, docs]
Status: divergence

Sessions must expire after 30 minutes of inactivity.

> [divergence] docs say 30 minutes; behaviour shows 15. Intent wins.

### Requirement: Session renewal on activity

ID: REQ-002
Sources: [docs]
Status: agreed

Activity within the window renews the session.

### Requirement: Concurrent session limit [unknown]

ID: REQ-003
Sources: []
Status: unknown

[unknown] No source states a concurrent-session limit.
";

#[test]
fn reviewable_set_parses() {
    let spec = parse(REVIEWABLE).expect("the reviewable set parses");
    assert!(spec.preamble.starts_with("# Session handling"));
    assert_eq!(spec.requirements.len(), 3);

    let first = &spec.requirements[0];
    assert_eq!(first.id, "REQ-001");
    assert_eq!(first.name, "Sessions expire after inactivity");
    assert_eq!(first.tag, Some(Tag::Divergence));
    assert_eq!(first.status, Status::Divergence);
    assert_eq!(first.sources, ["intent", "docs"]);
    assert!(first.body.contains("Intent wins"));

    let third = &spec.requirements[2];
    assert_eq!(third.tag, Some(Tag::Unknown));
    assert!(third.sources.is_empty(), "unknown may cite no sources");
}

#[test]
fn violations_fail_typed() {
    // (document, expected finding fragment)
    let cases: &[(&str, &str)] = &[
        ("# Title only, no blocks\n", "no `### Requirement:` block"),
        ("### Requirement: No id\n\nSources: [a]\nStatus: agreed\n\nBody.\n", "no `ID:` line"),
        (
            "### Requirement: Bad id\n\nID: REQ-1\nSources: [a]\nStatus: agreed\n\nBody.\n",
            "malformed id `REQ-1`",
        ),
        (
            "### Requirement: No sources\n\nID: REQ-001\nStatus: agreed\n\nBody.\n",
            "no `Sources:` line",
        ),
        ("### Requirement: No status\n\nID: REQ-001\nSources: [a]\n\nBody.\n", "no `Status:` line"),
        (
            "### Requirement: Bad status\n\nID: REQ-001\nSources: [a]\nStatus: resolved\n\nBody.\n",
            "unrecognised `Status: resolved`",
        ),
        (
            "### Requirement: Untagged conflict\n\nID: REQ-001\nSources: [a, b]\nStatus: conflict\n\nBody.\n",
            "without the `[conflict]` heading tag",
        ),
        (
            "### Requirement: Mistagged [conflict]\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\nBody.\n",
            "disagrees with `Status: agreed`",
        ),
        (
            "### Requirement: Stray tag [wip]\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\nBody.\n",
            "unrecognised heading tag `[wip]`",
        ),
        (
            "### Requirement: Evidence-less but agreed\n\nID: REQ-001\nSources: []\nStatus: agreed\n\nBody.\n",
            "empty `Sources:` but not `Status: unknown`",
        ),
        (
            "### Requirement: Bad key\n\nID: REQ-001\nSources: [Docs!]\nStatus: agreed\n\nBody.\n",
            "malformed source key `Docs!`",
        ),
        (
            "### Requirement: One\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\nBody.\n\n\
             ### Requirement: Two\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\nBody.\n",
            "duplicate requirement id `REQ-001`",
        ),
        (
            "### Requirement: Doubled\n\nID: REQ-001\nID: REQ-002\nSources: [a]\nStatus: agreed\n\nBody.\n",
            "duplicate `ID:` line",
        ),
    ];
    for (text, fragment) in cases {
        let err = parse(text).expect_err(fragment);
        let message = err.to_string();
        assert!(message.contains("spec-invalid"), "typed code for {fragment}: {message}");
        assert!(message.contains(fragment), "expected `{fragment}` in: {message}");
    }
}
