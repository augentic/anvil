//! Temporary scratch probe (delete after use).

use project::plan::decomposition::{Decomposition, Kind, Node, Scope};

const TREE: &str = include_str!(
    "/Users/andrewweston/github.com/augentic/emery-adapters/sandbox/omnia-r9k/decomposition.yaml"
);

fn base() -> Decomposition {
    Decomposition::parse(TREE).expect("parse")
}

fn keep(tree: &Decomposition, item: &diagnostics::Diagnostic) -> bool {
    let named = |id: Option<&str>| id.and_then(|id| tree.nodes.get(id));
    match item.rule_id.as_deref() {
        Some("decomposition-leaf-incomplete") => {
            named(item.slice.as_deref()).is_some_and(|node| node.kind == Some(Kind::Leaf))
        }
        Some("decomposition-non-reducing") => {
            named(item.slice.as_deref()).is_none_or(|node| node.kind.is_some())
        }
        _ => true,
    }
}

fn report(label: &str, tree: &Decomposition) {
    let all = project::plan::decomposition::findings(tree);
    let kept: Vec<String> = all
        .iter()
        .filter(|item| keep(tree, item))
        .map(|item| format!("{}: {}", item.rule_id.clone().unwrap_or_default(), item.impact))
        .collect();
    println!("=== {label} === kept={}", kept.len());
    for row in &kept {
        println!("    {row}");
    }
    println!("=== {label} === total={} check={:?}", all.len(), tree.check().is_ok());
}

/// The exact answer under consideration.
fn answer_leaf() -> Node {
    Node {
        parent: Some("root".into()),
        sources: vec![Scope::new("intent", "intent")],
        target: Some("service".into()),
        ownership: vec![
            "crates/gtfs-stop-catalog/src/refresh.rs".into(),
            "crates/gtfs-stop-catalog/tests/refresh.rs".into(),
        ],
        depends_on: vec!["gtfs-stop-catalog".into()],
        kind: Some(Kind::Leaf),
        slice: Some("gtfs-stop-refresh".into()),
        acceptance: Some("acceptance".into()),
        ..Node::default()
    }
}

#[test]
fn probe() {
    // Gate 1: the in-flight tree (siblings still open, no ownership).
    let mut tree = base();
    tree.nodes.insert("gtfs-stop-refresh".into(), answer_leaf());
    report("gate1 in-flight / intent-only 2 paths", &tree);

    // Gate 2: the complete tree, every sibling closed with its own
    // envelope; ours intent-only.
    let owners = [
        ("service-foundation", "crates/service-foundation/**"),
        ("confluent-secret-provider", "crates/confluent-secrets/**"),
        ("azure-system-token-provider", "crates/azure-token/**"),
        ("confluent-secret-rotation", "crates/confluent-rotation/**"),
        ("block-allocation-lookup", "crates/block-allocation/**"),
        ("gtfs-stop-catalog", "crates/gtfs-stop-catalog/**"),
        ("r9k-update-consumer", "crates/r9k-consumer/**"),
        ("smartrak-position-publisher", "crates/smartrak-publisher/**"),
        ("status-check-surface", "crates/status-check/**"),
    ];
    let mut tree = base();
    for (id, path) in owners {
        let node = tree.nodes.get_mut(id).expect("sibling");
        node.ownership = vec![path.into()];
        node.kind = Some(Kind::Leaf);
        node.slice = Some(id.into());
        node.acceptance = Some("acceptance".into());
    }
    let mut complete = tree.clone();
    complete.nodes.insert("gtfs-stop-refresh".into(), answer_leaf());
    report("gate2 complete / intent-only", &complete);

    // Same complete tree, but ours keeping both scopes — shows whether
    // the tie is only an in-flight artifact.
    let mut both = tree.clone();
    let mut node = answer_leaf();
    node.sources.push(Scope::new("legacy", "gtfs-stop-catalog-refresh"));
    both.nodes.insert("gtfs-stop-refresh".into(), node);
    report("gate2 complete / both scopes", &both);
}
