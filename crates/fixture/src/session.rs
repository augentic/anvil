//! Host-only session helpers over the harness default layer.
//!
//! A [`Session`] is a throw-away project tree plus a linked-only
//! [`harness::provider::Provider`] over the full fixture
//! [`crate::registry::catalog`] and `omnia-testkit`'s FIFO `Scripted`
//! model double. Construction goes through `Provider::new` — never
//! `Provider::bound` — so native tests start no listeners.

use std::path::{Path, PathBuf};

use harness::provider::Provider;
use omnia_testkit::model::Scripted as ScriptedModel;
use project::handler::ExecutionPaths;

use crate::model::Harness;

/// The provider shape the scripted suites run against: the fixture
/// catalog behind the seams, a request-recording [`Harness`] over
/// `omnia-testkit`'s FIFO script behind the judgment legs.
pub type Scripted = Provider<Harness<ScriptedModel>>;

/// A throw-away project tree plus the scripted fixture provider.
///
/// The tempdir and the session's isolated project-cache parent live
/// exactly as long as the session, so adapter cache writes stay
/// hermetic — the provider carries the cache placement as
/// [`ExecutionPaths`]; no process environment is mutated.
#[derive(Debug)]
pub struct Session {
    root: PathBuf,
    provider: Scripted,
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
        let paths = ExecutionPaths::isolated(&root, root.join("project-cache"));
        let provider = Provider::new(paths, Harness::answering(answers), crate::catalog());
        Self {
            root,
            provider,
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

    /// The scripted fixture provider.
    #[must_use]
    pub const fn provider(&self) -> &Scripted {
        &self.provider
    }

    /// The recording model backend — for `requests()` and
    /// `assert_exhausted`.
    #[must_use]
    pub const fn model(&self) -> &Harness<ScriptedModel> {
        self.provider.model()
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
