//! Host-only session helpers over the offline native provider.
//!
//! Reference mode is always [`ReferenceMode::Offline`], so native tests start
//! no listeners; the recording model handle is held beside the provider.

use std::path::{Path, PathBuf};

use native::{DynModel, Provider, ReferenceMode};
use omnia_testkit::model::{Harness, Scripted as ScriptedModel};
use project::handler::{CachePlacement, ExecutionPaths, Locations};

/// The recording model shape the scripted suites hold beside the
/// provider: a request-recording [`Harness`] over FIFO `Scripted`
/// behind the judgment legs.
pub type Scripted = Harness<ScriptedModel>;

/// A throw-away project tree plus the scripted mock provider.
///
/// The tempdir and the session's isolated store and cache roots live
/// exactly as long as the session, so adapter writes stay hermetic —
/// the provider carries the explicit layout as [`ExecutionPaths`]; no
/// process environment is read or mutated.
#[derive(Debug)]
pub struct Session {
    root: PathBuf,
    provider: Provider,
    model: Scripted,
    _tmp: tempfile::TempDir,
}

impl Session {
    /// A bare tree — nothing scaffolded, for suites whose first
    /// operation is `Init`.
    ///
    /// # Panics
    ///
    /// Panics when the tempdir cannot be created.
    #[must_use]
    pub fn bare(answers: Vec<String>) -> Self {
        let (tmp, base) = owned_tree();
        // Production shape: the layout roots (store, cache, snapshot
        // store, workspaces) live outside the project root, so a
        // product-tree freeze never captures them.
        let root = base.join("project");
        std::fs::create_dir_all(&root).expect("mkdir project root");
        let locations = Locations::explicit(
            base.join("adapter-store"),
            CachePlacement::Parent(base.join("project-cache")),
        );
        let paths = ExecutionPaths::new(&root, locations);
        let model = Harness::answering(answers);
        let provider = Provider::new(
            paths,
            DynModel::new(model.clone()),
            crate::catalog(),
            ReferenceMode::Offline,
        );
        Self {
            root,
            provider,
            model,
            _tmp: tmp,
        }
    }

    /// A detached change home (no `project.yaml`) with `answers`
    /// behind the judgment legs.
    ///
    /// # Panics
    ///
    /// Panics when the tempdir cannot be created.
    #[must_use]
    pub fn detached(answers: Vec<String>) -> Self {
        let (tmp, base) = owned_tree();
        let root = base.join("change");
        std::fs::create_dir_all(&root).expect("mkdir change home");
        let locations = Locations::explicit(
            base.join("adapter-store"),
            CachePlacement::Parent(base.join("project-cache")),
        );
        let paths = ExecutionPaths::detached(&root, locations);
        let model = Harness::answering(answers);
        let provider = Provider::new(
            paths,
            DynModel::new(model.clone()),
            crate::catalog(),
            ReferenceMode::Offline,
        );
        Self {
            root,
            provider,
            model,
            _tmp: tmp,
        }
    }

    /// A minimal initialised project (`.emery/project.yaml`) bound to
    /// `target_adapter`, with `answers` behind the judgment legs.
    ///
    /// # Panics
    ///
    /// Panics when the tempdir or project scaffold cannot be written.
    #[must_use]
    pub fn scripted(target_adapter: &str, answers: Vec<String>) -> Self {
        let session = Self::bare(answers);
        std::fs::create_dir_all(session.root.join(".emery")).expect("mkdir .emery");
        std::fs::write(
            session.root.join(".emery/project.yaml"),
            format!("name: demo\nadapter: {target_adapter}\nrules: {{}}\n"),
        )
        .expect("write project.yaml");
        session
    }

    /// The project root every project-scoped verb anchors at.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The scripted mock provider.
    #[must_use]
    pub const fn provider(&self) -> &Provider {
        &self.provider
    }

    /// Content-addressed snapshot store this session's provider writes to.
    ///
    /// # Panics
    ///
    /// Panics when the session layout is missing the temp-home parent
    /// of the project root.
    #[must_use]
    pub fn store(&self) -> project::workspace::Store<project::workspace::FsObjects> {
        let home = self.root.parent().expect("session layout: project sits under the temp home");
        project::workspace::Store::new(home.join("snapshots"))
    }

    /// Materialize this target's current accepted CID into a tempdir.
    ///
    /// Merge no longer writes the operator checkout; tests inspect
    /// folded baselines and build outputs from this tree.
    ///
    /// # Panics
    ///
    /// Panics when the journal cannot be read, the accepted-CID chain
    /// is broken, no CID exists yet, or materialization fails.
    pub async fn materialize_accepted(&self, target: &str) -> tempfile::TempDir {
        let dest = tempfile::TempDir::new().expect("accepted tree");
        let layout = project::config::Layout::new(&self.root);
        let events = project::journal::read_union(layout).expect("union");
        let cid = project::wave::accepted_cid(layout, &events, target)
            .expect("accepted-CID projection")
            .expect("target has an accepted CID");
        self.store().materialize(&cid, dest.path()).await.expect("materialize accepted CID");
        dest
    }

    /// The caller-held recording model handle — for `requests()` and
    /// `assert_exhausted`.
    #[must_use]
    pub const fn model(&self) -> &Scripted {
        &self.model
    }

    /// Replace the compiled model-capability profile table.
    #[must_use]
    pub fn with_profiles(mut self, table: project::profile::Table) -> Self {
        let provider = self.provider;
        self.provider = provider.with_profiles(table);
        self
    }
}

// A fresh tempdir; the session's cache parent lives inside it.
fn owned_tree() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical tempdir");
    (tmp, root)
}

/// RAII current-directory guard for the few scaffold tests that
/// require CWD; restores the previous directory on drop.
#[derive(Debug)]
pub struct Cwd {
    prev: PathBuf,
}

impl Cwd {
    /// Enter `dir`, restoring the current directory on drop.
    ///
    /// # Panics
    ///
    /// Panics when the current directory cannot be read or changed.
    #[must_use]
    pub fn enter(dir: &Path) -> Self {
        let prev = std::env::current_dir().expect("read current directory");
        std::env::set_current_dir(dir).expect("enter directory");
        Self { prev }
    }
}

impl Drop for Cwd {
    fn drop(&mut self) {
        // Best effort: the previous directory may be a removed tempdir.
        drop(std::env::set_current_dir(&self.prev));
    }
}
