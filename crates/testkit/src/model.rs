//! Model doubles: re-exports of `omnia-testkit`'s scripted and replay
//! backends plus the suite record-and-regenerate flow behind
//! `REGENERATE_FIXTURES=1`.

use std::path::Path;

use omnia_guest::Model;
use omnia_guest::model::{Error, Reply, Request};
pub use omnia_testkit::model::{Harness, Recorder, Replay, Scripted};

/// The env var that flips a replay suite into record mode: run the
/// suite once with `REGENERATE_FIXTURES=1` to rewrite its committed
/// fixture directory from the scripted answers, then commit the rows.
pub const REGENERATE_FIXTURES: &str = "REGENERATE_FIXTURES";

/// The model behind a replay-fixture suite: [`Replay`] over the
/// committed directory normally; a [`Recorder`] around the scripted
/// source of truth when regenerating.
#[derive(Clone, Debug)]
pub enum SuiteModel {
    /// Serving the committed fixture rows.
    Replay(Replay),
    /// Rewriting them from the scripted answers.
    Record(Recorder<Scripted>),
}

impl Model for SuiteModel {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        match self {
            Self::Replay(replay) => replay.create(request).await,
            Self::Record(recorder) => recorder.create(request).await,
        }
    }
}

/// The suite model over the committed `fixtures` directory — or, when
/// [`REGENERATE_FIXTURES`] is set, a recorder that clears the stale
/// rows and rewrites the directory from `answers` as the suite runs.
///
/// # Panics
///
/// Panics when a suite with scripted answers has no fixture directory,
/// the fixture directory cannot be read, or a committed row is malformed.
#[must_use]
pub fn suite_model(fixtures: &Path, answers: Vec<String>) -> SuiteModel {
    if std::env::var_os(REGENERATE_FIXTURES).is_some() {
        clear_rows(fixtures);
        SuiteModel::Record(Recorder::new(Scripted::answers(answers), fixtures))
    } else if answers.is_empty() && !fixtures.exists() {
        SuiteModel::Replay(Replay::new([]).expect("empty replay fixtures load"))
    } else {
        SuiteModel::Replay(Replay::from_dir(fixtures).expect("replay fixtures load"))
    }
}

// Remove stale fixture rows so a regeneration run leaves no orphans.
fn clear_rows(fixtures: &Path) {
    let Ok(entries) = std::fs::read_dir(fixtures) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            std::fs::remove_file(&path).expect("remove stale fixture row");
        }
    }
}
