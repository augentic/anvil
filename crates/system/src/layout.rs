//! `Layout<'a>` — the typed home for every definition-home path.
//! Deliberately separate from `project::config::Layout`: a definition
//! home has no `.emery/` tree and no `project.yaml`.

use std::path::{Path, PathBuf};

/// Typed view over one definition-home root (RFC-104 D1).
///
/// The operator creates the root and the declared files (`scope.yaml`,
/// `coverage.yaml`, `decisions/`); the engine writes only the
/// generated layout (`evidence/`, `architecture/`, `handoffs/`,
/// `events/`) beneath it. Events live at `<system>/events/`, not
/// `.emery/events/`.
#[derive(Debug, Clone, Copy)]
pub struct Layout<'a> {
    root: &'a Path,
}

impl<'a> Layout<'a> {
    /// Wrap `root` as the typed definition home for path lookups.
    #[must_use]
    pub const fn new(root: &'a Path) -> Self {
        Self { root }
    }

    /// The definition-home root the layout is anchored at.
    #[must_use]
    pub const fn root(&self) -> &'a Path {
        self.root
    }

    /// `<system>/scope.yaml` — the declared boundary (operator-owned).
    #[must_use]
    pub fn scope_path(&self) -> PathBuf {
        self.root.join("scope.yaml")
    }

    /// `<system>/coverage.yaml` — one row per declared candidate
    /// (declared fields operator-owned; observed fields survey-owned).
    #[must_use]
    pub fn coverage_path(&self) -> PathBuf {
        self.root.join("coverage.yaml")
    }

    /// `<system>/system.yaml` — declared identities plus named
    /// architecture states (`as-is`, `target`, `transition-*`).
    #[must_use]
    pub fn system_path(&self) -> PathBuf {
        self.root.join("system.yaml")
    }

    /// `<system>/migration.yaml` — inlined modernization dispositions
    /// and migration waves (operator-owned once written).
    #[must_use]
    pub fn migration_path(&self) -> PathBuf {
        self.root.join("migration.yaml")
    }

    /// `<system>/evidence/` — survey-written Evidence, one document
    /// per `(source, lead)`.
    #[must_use]
    pub fn evidence_dir(&self) -> PathBuf {
        self.root.join("evidence")
    }

    /// `<system>/evidence/<source>/` — one included source's Evidence.
    #[must_use]
    pub fn source_evidence_dir(&self, source: &str) -> PathBuf {
        self.evidence_dir().join(source)
    }

    /// `<system>/evidence/<source>/<lead>.yaml` — one lead's Evidence.
    #[must_use]
    pub fn evidence_path(&self, source: &str, lead: &str) -> PathBuf {
        self.source_evidence_dir(source).join(format!("{lead}.yaml"))
    }

    /// `<system>/architecture/` — generated document and diagram
    /// projections (never authority).
    #[must_use]
    pub fn architecture_dir(&self) -> PathBuf {
        self.root.join("architecture")
    }

    /// `<system>/architecture/<name>.md` for `as-is` / `target`,
    /// `<system>/architecture/transitions/<name>.md` for
    /// `transition-*` states.
    #[must_use]
    pub fn state_doc_path(&self, name: &str) -> PathBuf {
        let dir = if name.starts_with("transition-") {
            self.architecture_dir().join("transitions")
        } else {
            self.architecture_dir()
        };
        dir.join(format!("{name}.md"))
    }

    /// `<system>/architecture/diagrams/` — committed diagram source
    /// beside its rendered form.
    #[must_use]
    pub fn diagrams_dir(&self) -> PathBuf {
        self.architecture_dir().join("diagrams")
    }

    /// `<system>/architecture/diagrams/<view>.source` — deterministic
    /// textual diagram notation.
    #[must_use]
    pub fn diagram_source_path(&self, view: &str) -> PathBuf {
        self.diagrams_dir().join(format!("{view}.source"))
    }

    /// `<system>/architecture/diagrams/<view>.svg` — the rendered view
    /// beside its committed source.
    #[must_use]
    pub fn diagram_svg_path(&self, view: &str) -> PathBuf {
        self.diagrams_dir().join(format!("{view}.svg"))
    }

    /// `<system>/handoffs/` — canonical wave handoffs named by digest;
    /// historical handoffs are never deleted.
    #[must_use]
    pub fn handoffs_dir(&self) -> PathBuf {
        self.root.join("handoffs")
    }

    /// `<system>/handoffs/<digest>.yaml` where `digest` is the bare
    /// 64-hex content address (no `sha256:` scheme).
    #[must_use]
    pub fn handoff_path(&self, digest: &str) -> PathBuf {
        self.handoffs_dir().join(format!("{digest}.yaml"))
    }

    /// `<system>/decisions/` — operator-authored definition decision
    /// records. The engine never writes this directory.
    #[must_use]
    pub fn decisions_dir(&self) -> PathBuf {
        self.root.join("decisions")
    }

    /// `<system>/decisions/<id>.yaml` — one definition decision.
    #[must_use]
    pub fn decision_path(&self, id: &str) -> PathBuf {
        self.decisions_dir().join(format!("{id}.yaml"))
    }

    /// `<system>/events/` — per-writer append-only fact logs, kept
    /// separate from any change home's `.emery/events/`.
    #[must_use]
    pub fn events_dir(&self) -> PathBuf {
        self.root.join("events")
    }

    /// `<system>/events/<writer>.jsonl` — one writer's event log.
    #[must_use]
    pub fn writer_events_path(&self, writer: &str) -> PathBuf {
        self.events_dir().join(format!("{writer}.jsonl"))
    }
}
