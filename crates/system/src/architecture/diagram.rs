//! Diagram projections: deterministic DOT source and its rendered
//! SVG, both stamped with the projected state's exact digest. The
//! renderer is the wasm-clean `layout` crate; a sort shim over the
//! SVG `<style>` block keeps the rendered bytes deterministic.

use std::collections::BTreeMap;

use ::layout::backends::svg::SVGWriter;
use ::layout::core::base::Orientation;
use ::layout::core::geometry::Point;
use ::layout::core::style::StyleAttr;
use ::layout::std_shapes::shapes::{Arrow, Element as Node, ShapeKind};
use ::layout::topo::layout::VisualGraph;
use project::snapshot::SnapshotId;

use super::{DIGEST_PREFIX, kind_word, line};
use crate::model::{Element, ElementKind, Relationship, State, Status};

/// Render the deterministic DOT source of one state.
#[must_use]
pub fn source(name: &str, digest: &SnapshotId, state: &State) -> String {
    let mut out = String::new();
    let w = &mut out;
    line(w, "// Generated projection — not architecture authority. Do not edit.");
    line(w, &format!("// State: {name}"));
    line(w, &format!("// {DIGEST_PREFIX}{}", digest.as_str()));
    line(w, &format!("digraph {} {{", quote(name)));
    line(w, "  rankdir=TB;");
    for element in ordered_elements(state) {
        let mut label = format!("{}\\n({})", element.id, kind_slug(element.kind));
        match element.status {
            Status::Conflict => label.push_str("\\n[conflict]"),
            Status::Unknown => label.push_str("\\n[unknown]"),
            _ => {}
        }
        let style = if element.context_only { ", style=dashed" } else { "" };
        line(
            w,
            &format!(
                "  {} [shape={}, label={}{style}];",
                quote(&element.id),
                kind_shape(element.kind),
                quote(&label),
            ),
        );
    }
    for relationship in ordered_relationships(state) {
        line(
            w,
            &format!(
                "  {} -> {} [label={}];",
                quote(&relationship.from),
                quote(&relationship.to),
                quote(kind_word(relationship)),
            ),
        );
    }
    line(w, "}");
    out
}

/// Render the deterministic SVG view of one state.
#[must_use]
pub fn svg(name: &str, digest: &SnapshotId, state: &State) -> String {
    let body = if state.elements.is_empty() {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n<svg \
             xmlns=\"http://www.w3.org/2000/svg\" width=\"360\" height=\"48\">\n<text x=\"12\" \
             y=\"28\" font-size=\"14\">{name}: no elements recovered</text>\n</svg>\n"
        )
    } else {
        sort_style_block(&render(state))
    };
    stamp(name, digest, &body)
}

/// Lay the state out as a graph and render it.
fn render(state: &State) -> String {
    let mut graph = VisualGraph::new(Orientation::TopToBottom);
    let mut handles = BTreeMap::new();
    for element in ordered_elements(state) {
        let label = format!("{} ({})", element.id, kind_slug(element.kind));
        let node = Node::create(
            ShapeKind::new_box(&label),
            StyleAttr::simple(),
            Orientation::TopToBottom,
            Point::new(140., 40.),
        );
        handles.insert(element.id.as_str(), graph.add_node(node));
    }
    for relationship in ordered_relationships(state) {
        // Endpoint resolution is guaranteed by model validation.
        let (Some(from), Some(to)) =
            (handles.get(relationship.from.as_str()), handles.get(relationship.to.as_str()))
        else {
            continue;
        };
        graph.add_edge(Arrow::simple(kind_word(relationship)), *from, *to);
    }
    let mut writer = SVGWriter::new();
    graph.do_it(false, false, false, &mut writer);
    writer.finalize()
}

/// Elements sorted by id — the projection's stable order.
fn ordered_elements(state: &State) -> Vec<&Element> {
    let mut elements: Vec<&Element> = state.elements.iter().collect();
    elements.sort_by(|a, b| a.id.cmp(&b.id));
    elements
}

/// Relationships sorted by id — the projection's stable order.
fn ordered_relationships(state: &State) -> Vec<&Relationship> {
    let mut relationships: Vec<&Relationship> = state.relationships.iter().collect();
    relationships.sort_by(|a, b| a.id.cmp(&b.id));
    relationships
}

/// Prepend the digest stamp as an XML comment, after any declaration.
fn stamp(name: &str, digest: &SnapshotId, svg: &str) -> String {
    let comment = format!(
        "<!--\nGenerated projection — not architecture authority. Do not edit.\nState: \
         {name}\n{DIGEST_PREFIX}{}\n-->",
        digest.as_str()
    );
    match svg.split_once('\n') {
        Some((first, rest)) if first.starts_with("<?xml") => {
            format!("{first}\n{comment}\n{rest}")
        }
        _ => format!("{comment}\n{svg}"),
    }
}

/// Sort the `<style>` block's lines: the renderer emits font classes
/// in hash order, the one nondeterminism in its output.
fn sort_style_block(svg: &str) -> String {
    let (Some(open), Some(close)) = (svg.find("<style>"), svg.find("</style>")) else {
        return svg.to_string();
    };
    let body_start = open + "<style>".len();
    if close < body_start {
        return svg.to_string();
    }
    let mut lines: Vec<&str> =
        svg[body_start..close].lines().filter(|line| !line.trim().is_empty()).collect();
    lines.sort_unstable();
    format!("{}\n{}\n{}", &svg[..body_start], lines.join("\n"), &svg[close..])
}

/// Escape and quote a DOT identifier or label.
fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The wire kind word shown in labels.
const fn kind_slug(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::System => "system",
        ElementKind::Service => "service",
        ElementKind::Repository => "repository",
        ElementKind::Interface => "interface",
        ElementKind::DataStore => "data-store",
        ElementKind::Queue => "queue",
        ElementKind::ScheduledJob => "scheduled-job",
        ElementKind::DeploymentUnit => "deployment-unit",
        ElementKind::Environment => "environment",
        ElementKind::ExternalActor => "external-actor",
        ElementKind::OwningGroup => "owning-group",
    }
}

/// The DOT node shape for one element kind.
const fn kind_shape(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::System => "box3d",
        ElementKind::Service | ElementKind::DeploymentUnit => "box",
        ElementKind::Repository => "folder",
        ElementKind::Interface => "component",
        ElementKind::DataStore => "cylinder",
        ElementKind::Queue => "cds",
        ElementKind::ScheduledJob => "oval",
        ElementKind::Environment => "house",
        ElementKind::ExternalActor => "ellipse",
        ElementKind::OwningGroup => "note",
    }
}
