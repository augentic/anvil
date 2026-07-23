//! Host-only session helpers over the offline native provider.
//!
//! A [`Session`] is a throw-away project tree plus an offline
//! [`native::Provider`] over the full mock
//! [`crate::registry::catalog`] and `omnia-testkit`'s FIFO `Scripted`
//! model double. Reference mode is always [`ReferenceMode::Offline`],
//! so native tests start no listeners; the recording model handle is
//! held beside the provider (the provider exposes no model accessor).

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
        let (tmp, root) = owned_tree();
        let locations = Locations::explicit(
            root.join("adapter-store"),
            CachePlacement::Parent(root.join("project-cache")),
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

    /// A minimal initialised project (`.specify/project.yaml`) bound to
    /// `target_adapter`, with `answers` behind the judgment legs.
    ///
    /// # Panics
    ///
    /// Panics when the tempdir or project scaffold cannot be written.
    #[must_use]
    pub fn scripted(target_adapter: &str, answers: Vec<String>) -> Self {
        let session = Self::bare(answers);
        std::fs::create_dir_all(session.root.join(".specify")).expect("mkdir .specify");
        std::fs::write(
            session.root.join(".specify/project.yaml"),
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

    /// The caller-held recording model handle — for `requests()` and
    /// `assert_exhausted`.
    #[must_use]
    pub const fn model(&self) -> &Scripted {
        &self.model
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
