//! Architecture projections (RFC-104 D5): deterministic views over
//! one named `system.yaml` state, stamped with that state's exact
//! digest. Projections are never authority; stale ones fail validation.

pub mod diagram;

use std::fmt::Write as _;
use std::path::Path;

use error::Error;
use project::snapshot::SnapshotId;

use crate::layout::Layout;
use crate::model::{Element, ElementKind, Model, Relationship, State, Status};

/// The stamp line a projection carries; validation parses it back.
const DIGEST_PREFIX: &str = "Digest: ";

/// Project one named state: the Markdown document plus the diagram
/// source and its rendered SVG, all stamped with the state's digest.
///
/// # Errors
///
/// Digest serialization and atomic-write failures.
pub fn project(layout: &Layout<'_>, name: &str, state: &State) -> Result<(), Error> {
    let digest = state.digest()?;
    artifacts::atomic::bytes_write(
        &layout.state_doc_path(name),
        markdown(name, &digest, state).as_bytes(),
    )?;
    artifacts::atomic::bytes_write(
        &layout.diagram_source_path(name),
        diagram::source(name, &digest, state).as_bytes(),
    )?;
    artifacts::atomic::bytes_write(
        &layout.diagram_svg_path(name),
        diagram::svg(name, &digest, state).as_bytes(),
    )?;
    Ok(())
}

/// Check every committed projection against the live model.
///
/// A projection whose named state no longer exists, or whose stamped
/// digest differs from the live state's, is stale. An absent
/// `architecture/` directory is valid (nothing projected yet).
///
/// # Errors
///
/// - `system-projection-stale` naming each stale view.
/// - I/O failures reading the directory.
pub fn validate(layout: &Layout<'_>, model: &Model) -> Result<(), Error> {
    let mut stale = Vec::new();
    let dirs = [
        (layout.architecture_dir(), ""),
        (layout.architecture_dir().join("transitions"), "transitions/"),
        (layout.diagrams_dir(), "diagrams/"),
    ];
    for (dir, prefix) in dirs {
        scan(&dir, prefix, model, &mut stale)?;
    }
    stale.sort();
    stale.dedup();
    if stale.is_empty() {
        Ok(())
    } else {
        Err(Error::validation_failed(
            "system-projection-stale",
            "projections match their state digest",
            format!(
                "stale architecture projections: {}; re-run the stage that writes them",
                stale.join(", ")
            ),
        ))
    }
}

/// Check one projection directory's files, pushing stale view names.
fn scan(dir: &Path, prefix: &str, model: &Model, stale: &mut Vec<String>) -> Result<(), Error> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(Error::Io(err)),
    };
    for entry in entries {
        let path = entry.map_err(Error::Io)?.path();
        let Some(name) = projected_state(&path) else {
            continue;
        };
        let text = std::fs::read_to_string(&path).map_err(Error::Io)?;
        let fresh = match model.state(&name) {
            None => false,
            Some(live) => stamped_digest(&text) == Some(live.digest()?.as_str().to_string()),
        };
        if !fresh {
            stale.push(format!("{prefix}{}", display_name(&path)));
        }
    }
    Ok(())
}

/// The state a projection file claims to view, from its filename.
fn projected_state(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    matches!(path.extension()?.to_str()?, "md" | "svg" | "source").then(|| stem.to_string())
}

/// A projection file's name for the staleness report.
fn display_name(path: &Path) -> String {
    path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
}

/// The stamped digest inside a projection body — the `Digest: ` line,
/// wherever the format's comment syntax places it.
fn stamped_digest(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let start = line.find(DIGEST_PREFIX)?;
        Some(line[start + DIGEST_PREFIX.len()..].trim().to_string())
    })
}

/// Render the deterministic Markdown view of one state.
#[must_use]
pub fn markdown(name: &str, digest: &SnapshotId, state: &State) -> String {
    let mut out = String::new();
    let w = &mut out;
    line(w, "<!-- Generated projection — not architecture authority. Do not edit. -->");
    line(w, &format!("# Architecture — {name}"));
    line(w, "");
    line(w, &format!("State: {name}"));
    line(w, &format!("{DIGEST_PREFIX}{}", digest.as_str()));
    line(w, "");

    line(w, "## Elements");
    if state.elements.is_empty() {
        line(w, "");
        line(w, "None recovered.");
    }
    for kind in KINDS {
        let mut members: Vec<&Element> =
            state.elements.iter().filter(|element| element.kind == *kind).collect();
        if members.is_empty() {
            continue;
        }
        members.sort_by(|a, b| a.id.cmp(&b.id));
        line(w, "");
        line(w, &format!("### {}", kind_heading(*kind)));
        line(w, "");
        for element in members {
            line(w, &element_row(element));
            for (key, value) in &element.attributes {
                line(w, &format!("  - {key}: {value}"));
            }
        }
    }
    line(w, "");

    line(w, "## Relationships");
    line(w, "");
    if state.relationships.is_empty() {
        line(w, "None recovered.");
    } else {
        line(w, "| id | kind | from | to | status |");
        line(w, "| --- | --- | --- | --- | --- |");
        let mut relationships: Vec<&Relationship> = state.relationships.iter().collect();
        relationships.sort_by(|a, b| a.id.cmp(&b.id));
        for relationship in relationships {
            line(w, &relationship_row(relationship));
        }
    }
    line(w, "");

    line(w, "## Gaps and conflicts");
    line(w, "");
    let flagged: Vec<String> = records(state)
        .filter(|(_, status)| matches!(status, Status::Unknown | Status::Conflict))
        .map(|(id, status)| format!("- `{id}` — {}", status_label(status)))
        .collect();
    if flagged.is_empty() {
        line(w, "None.");
    } else {
        for row in flagged {
            line(w, &row);
        }
    }
    out
}

/// Every record's `(id, status)`, elements first, in model order.
fn records(state: &State) -> impl Iterator<Item = (&str, Status)> {
    state.elements.iter().map(|element| (element.id.as_str(), element.status)).chain(
        state
            .relationships
            .iter()
            .map(|relationship| (relationship.id.as_str(), relationship.status)),
    )
}

fn element_row(element: &Element) -> String {
    let context = if element.context_only { ", context-only" } else { "" };
    let mut row = format!("- **{}** — {}{context}", element.id, status_label(element.status));
    if let Some(decision) = &element.decision {
        let _infallible = write!(row, " (decision: {decision})");
    }
    if !element.claims.is_empty() {
        let cited: Vec<String> =
            element.claims.iter().map(|claim| format!("{}#{}", claim.source, claim.id)).collect();
        let _infallible = write!(row, " [{}]", cited.join(", "));
    }
    row
}

fn relationship_row(relationship: &Relationship) -> String {
    format!(
        "| {} | {} | {} | {} | {} |",
        relationship.id,
        kind_word(relationship),
        relationship.from,
        relationship.to,
        status_label(relationship.status)
    )
}

fn line(out: &mut String, text: &str) {
    // Writing to a String cannot fail.
    let _infallible = writeln!(out, "{text}");
}

/// Element kinds in projection order.
const KINDS: &[ElementKind] = &[
    ElementKind::System,
    ElementKind::Service,
    ElementKind::Repository,
    ElementKind::Interface,
    ElementKind::DataStore,
    ElementKind::Queue,
    ElementKind::ScheduledJob,
    ElementKind::DeploymentUnit,
    ElementKind::Environment,
    ElementKind::ExternalActor,
    ElementKind::OwningGroup,
];

const fn kind_heading(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::System => "Systems",
        ElementKind::Service => "Services",
        ElementKind::Repository => "Repositories",
        ElementKind::Interface => "Interfaces",
        ElementKind::DataStore => "Data stores",
        ElementKind::Queue => "Queues and topics",
        ElementKind::ScheduledJob => "Scheduled jobs",
        ElementKind::DeploymentUnit => "Deployment units",
        ElementKind::Environment => "Environments",
        ElementKind::ExternalActor => "External actors",
        ElementKind::OwningGroup => "Owning groups",
    }
}

const fn kind_word(relationship: &Relationship) -> &'static str {
    use crate::model::RelationshipKind as K;
    match relationship.kind {
        K::Containment => "containment",
        K::Deployment => "deployment",
        K::Invocation => "invocation",
        K::Publication => "publication",
        K::Consumption => "consumption",
        K::Read => "read",
        K::Write => "write",
        K::Dependency => "dependency",
        K::Ownership => "ownership",
    }
}

const fn status_label(status: Status) -> &'static str {
    match status {
        Status::Evidenced => "evidenced",
        Status::Inferred => "inferred",
        Status::Conflict => "conflict",
        Status::Unknown => "unknown",
        Status::Decided => "decided",
    }
}
