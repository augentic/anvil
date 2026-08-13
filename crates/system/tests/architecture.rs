//! Architecture projections: deterministic Markdown from one state
//! digest, and staleness validation over the committed views.

use std::collections::BTreeMap;

use system::architecture::{self, diagram};
use system::layout::Layout;
use system::model::overlay::persist_as_is;
use system::model::{
    ClaimRef, Element, ElementKind, Model, Relationship, RelationshipKind, State, Status,
};

fn state(id: &str) -> State {
    State {
        elements: vec![Element {
            id: id.to_string(),
            kind: ElementKind::Service,
            status: Status::Evidenced,
            claims: vec![ClaimRef {
                source: "orders-code".to_string(),
                id: "orders.api".to_string(),
            }],
            decision: None,
            context_only: false,
            attributes: BTreeMap::from([("ownership".to_string(), "order state".to_string())]),
        }],
        relationships: Vec::new(),
    }
}

/// Two elements joined by a `write` relationship.
fn linked_state() -> State {
    let mut view = state("orders");
    view.elements.push(Element {
        id: "orders-store".to_string(),
        kind: ElementKind::DataStore,
        status: Status::Inferred,
        claims: Vec::new(),
        decision: None,
        context_only: false,
        attributes: BTreeMap::new(),
    });
    view.relationships.push(Relationship {
        id: "orders-writes-store".to_string(),
        kind: RelationshipKind::Write,
        from: "orders".to_string(),
        to: "orders-store".to_string(),
        status: Status::Evidenced,
        claims: vec![ClaimRef {
            source: "orders-code".to_string(),
            id: "orders.store-write".to_string(),
        }],
        decision: None,
        context_only: false,
        attributes: BTreeMap::new(),
    });
    view
}

#[test]
fn deterministic_markdown() {
    let view = state("orders");
    let digest = view.digest().expect("digest");
    let first = architecture::markdown("as-is", &digest, &view);
    assert_eq!(first, architecture::markdown("as-is", &digest, &view), "byte-identical");
    assert!(first.contains(&format!("Digest: {}", digest.as_str())), "{first}");
    assert!(first.contains("### Services"), "{first}");
    assert!(first.contains("ownership: order state"), "{first}");
    assert!(first.contains("[orders-code#orders.api]"), "claims cited: {first}");
}

#[test]
fn fresh_validates() {
    let home = tempfile::tempdir().expect("tempdir");
    let layout = Layout::new(home.path());
    let model = persist_as_is(&layout.system_path(), state("orders"), &[]).expect("persist");
    architecture::project(&layout, "as-is", &model.as_is).expect("project");
    architecture::validate(&layout, &model).expect("fresh projection validates");
}

#[test]
fn stale_fails() {
    // The model moved on after the view was written: validation names
    // the stale projection instead of letting it survive as current.
    let home = tempfile::tempdir().expect("tempdir");
    let layout = Layout::new(home.path());
    let model = persist_as_is(&layout.system_path(), state("orders"), &[]).expect("persist");
    architecture::project(&layout, "as-is", &model.as_is).expect("project");

    let moved = persist_as_is(&layout.system_path(), state("billing"), &[]).expect("re-persist");
    let err = architecture::validate(&layout, &moved).expect_err("stale projection");
    assert!(err.to_string().contains("as-is"), "{err}");
}

#[test]
fn edits_fail_visibly() {
    // Hand-editing a projection cannot change authority — the stamped
    // digest no longer matches and validation refuses it.
    let home = tempfile::tempdir().expect("tempdir");
    let layout = Layout::new(home.path());
    let model = persist_as_is(&layout.system_path(), state("orders"), &[]).expect("persist");
    architecture::project(&layout, "as-is", &model.as_is).expect("project");

    let path = layout.architecture_dir().join("as-is.md");
    let edited = std::fs::read_to_string(&path).expect("view").replace(
        &format!("Digest: {}", model.as_is.digest().expect("digest").as_str()),
        "Digest: sha256:0000",
    );
    std::fs::write(&path, edited).expect("edit");
    architecture::validate(&layout, &model).expect_err("edited digest is stale");
}

#[test]
fn unknown_state_is_stale() {
    let home = tempfile::tempdir().expect("tempdir");
    let layout = Layout::new(home.path());
    let model = persist_as_is(&layout.system_path(), state("orders"), &[]).expect("persist");
    std::fs::create_dir_all(layout.architecture_dir()).expect("dir");
    std::fs::write(layout.architecture_dir().join("transition-gone.md"), "Digest: sha256:0\n")
        .expect("orphan view");
    let err = architecture::validate(&layout, &model).expect_err("orphan projection");
    assert!(err.to_string().contains("transition-gone"), "{err}");
}

#[test]
fn absent_dir_validates() {
    let home = tempfile::tempdir().expect("tempdir");
    let layout = Layout::new(home.path());
    architecture::validate(&layout, &Model::empty()).expect("nothing projected is valid");
}

#[test]
fn deterministic_diagrams() {
    // AC4: re-running the projection over the same state digest is
    // byte-identical for the committed source and the rendered SVG.
    let view = linked_state();
    let digest = view.digest().expect("digest");

    let source = diagram::source("as-is", &digest, &view);
    assert_eq!(source, diagram::source("as-is", &digest, &view), "byte-identical source");
    assert!(source.contains(&format!("// Digest: {}", digest.as_str())), "{source}");
    assert!(source.contains("digraph \"as-is\""), "{source}");
    assert!(source.contains("shape=cylinder"), "data store shape: {source}");
    assert!(source.contains("\"orders\" -> \"orders-store\" [label=\"write\"];"), "{source}");

    let svg = diagram::svg("as-is", &digest, &view);
    assert_eq!(svg, diagram::svg("as-is", &digest, &view), "byte-identical svg");
    assert!(svg.contains(&format!("Digest: {}", digest.as_str())), "{svg}");
    assert!(svg.contains("<svg "), "{svg}");
    assert!(svg.contains("orders-store (data-store)"), "node label rendered: {svg}");
}

#[test]
fn empty_state_diagram() {
    let view = State::default();
    let digest = view.digest().expect("digest");
    let svg = diagram::svg("as-is", &digest, &view);
    assert_eq!(svg, diagram::svg("as-is", &digest, &view), "byte-identical svg");
    assert!(svg.contains("no elements recovered"), "{svg}");
    assert!(svg.contains(&format!("Digest: {}", digest.as_str())), "{svg}");
}

#[test]
fn writes_diagram_pair() {
    let home = tempfile::tempdir().expect("tempdir");
    let layout = Layout::new(home.path());
    let model = persist_as_is(&layout.system_path(), linked_state(), &[]).expect("persist");
    architecture::project(&layout, "as-is", &model.as_is).expect("project");
    assert!(layout.diagram_source_path("as-is").is_file(), "committed source");
    assert!(layout.diagram_svg_path("as-is").is_file(), "rendered view beside it");
    architecture::validate(&layout, &model).expect("fresh diagrams validate");
}

#[test]
fn stale_diagram_fails() {
    // An edited or outdated rendered view cannot survive as current.
    let home = tempfile::tempdir().expect("tempdir");
    let layout = Layout::new(home.path());
    let model = persist_as_is(&layout.system_path(), linked_state(), &[]).expect("persist");
    architecture::project(&layout, "as-is", &model.as_is).expect("project");

    let path = layout.diagram_svg_path("as-is");
    let digest = model.as_is.digest().expect("digest");
    let edited = std::fs::read_to_string(&path)
        .expect("view")
        .replace(&format!("Digest: {}", digest.as_str()), "Digest: sha256:0000");
    std::fs::write(&path, edited).expect("edit");

    let err = architecture::validate(&layout, &model).expect_err("stale rendered view");
    assert!(err.to_string().contains("diagrams/as-is.svg"), "{err}");
}
