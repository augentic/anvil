//! Scratch probe: which partition answers for `omnia-crate-scaffold`
//! clear the deterministic validators against the live sandbox tree.

use project::plan::decomposition::{Decomposition, Kind, Node, Scope, findings};

const TREE: &str = include_str!(
    "/Users/andrewweston/github.com/augentic/emery-adapters/sandbox/omnia-r9k/decomposition.yaml"
);

fn base() -> Decomposition {
    Decomposition::parse(TREE).expect("sandbox tree parses")
}

/// Mirror of `change::orchestrate::decompose::keep_progress`.
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
    let found: Vec<_> = findings(tree).into_iter().filter(|item| keep(tree, item)).collect();
    println!("=== {label}: {} blocking finding(s)", found.len());
    for item in &found {
        println!("    [{}] {}", item.rule_id.clone().unwrap_or_default(), item.impact);
    }
}

fn leaf(tree: &mut Decomposition, sources: Vec<Scope>, paths: usize) {
    let node = tree.node_mut("omnia-crate-scaffold").expect("node");
    node.sources = sources;
    node.kind = Some(Kind::Leaf);
    node.slice = Some(project::name::SliceName::new("omnia-crate-scaffold"));
    node.acceptance = Some("scaffold builds".into());
    node.target = Some("service".into());
    node.ownership = (0..paths).map(|index| format!("path{index}/**")).collect();
}

#[test]
fn probe_candidates() {
    let mut tree = base();
    report("open (no answer)", &tree);

    let both =
        vec![Scope::new("intent", "intent"), Scope::new("legacy", "legacy")];

    leaf(&mut tree, both.clone(), 19);
    report("leaf, both leads, 19 paths", &tree);

    let mut tree = base();
    leaf(&mut tree, both.clone(), 6);
    report("leaf, both leads, 6 paths", &tree);

    let mut tree = base();
    leaf(&mut tree, vec![Scope::new("legacy", "legacy")], 12);
    report("leaf, legacy lead only", &tree);

    let mut tree = base();
    leaf(&mut tree, vec![Scope::new("intent", "intent")], 12);
    report("leaf, intent lead only", &tree);

    let mut tree = base();
    leaf(&mut tree, both.clone(), 12);
    tree.node_mut("omnia-crate-scaffold").expect("node").depends_on = Vec::new();
    report("leaf, both leads, 12 paths", &tree);

    // split into two children, neither carrying ownership yet
    let mut tree = base();
    for (id, sources) in [
        ("omnia-workspace-shell", vec![Scope::new("intent", "intent")]),
        (
            "omnia-provider-seam",
            vec![Scope::new("intent", "intent"), Scope::new("legacy", "legacy")],
        ),
    ] {
        tree.nodes.insert(
            id.into(),
            Node {
                parent: Some("omnia-crate-scaffold".into()),
                sources,
                target: Some("service".into()),
                ..Node::default()
            },
        );
    }
    {
        let node = tree.node_mut("omnia-crate-scaffold").expect("node");
        node.kind = Some(Kind::Split);
        node.children = vec!["omnia-workspace-shell".into(), "omnia-provider-seam".into()];
    }
    report("split into two open children", &tree);
}
