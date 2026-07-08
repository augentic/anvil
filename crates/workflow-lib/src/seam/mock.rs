//! Native scripted providers for the seam capability traits.
//!
//! Mirrors the scripted judgment `MockModel` in the test binaries:
//! replies are served in FIFO order per operation and every call is
//! recorded for assertion.

use std::collections::VecDeque;
use std::sync::Mutex;

use super::{Error, Evidence, Input, Lead, SourceSeam, TargetSeam, WorkingTree};
use crate::slice::BuildReport;

/// One recorded [`MockSourceSeam`] call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceCall {
    /// A `survey(id)` dispatch.
    Survey {
        /// The routed adapter id (e.g. `source:typescript`).
        id: String,
    },
    /// An `extract(id, lead)` dispatch.
    Extract {
        /// The routed adapter id.
        id: String,
        /// The lead the extract named.
        lead: Lead,
    },
}

/// Scripted [`SourceSeam`] provider for native tests.
#[derive(Debug, Default)]
pub struct MockSourceSeam {
    surveys: Mutex<VecDeque<Result<Vec<Lead>, Error>>>,
    extracts: Mutex<VecDeque<Result<Evidence, Error>>>,
    calls: Mutex<Vec<SourceCall>>,
}

impl MockSourceSeam {
    /// A mock answering each `survey` / `extract` call with the next
    /// scripted result of the matching operation.
    #[must_use]
    pub fn scripted(
        surveys: impl IntoIterator<Item = Result<Vec<Lead>, Error>>,
        extracts: impl IntoIterator<Item = Result<Evidence, Error>>,
    ) -> Self {
        Self {
            surveys: Mutex::new(surveys.into_iter().collect()),
            extracts: Mutex::new(extracts.into_iter().collect()),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Every call the mock has served, in call order.
    ///
    /// # Panics
    ///
    /// Panics when the interior lock is poisoned — only possible after
    /// a prior panic in the same test.
    #[must_use]
    pub fn calls(&self) -> Vec<SourceCall> {
        self.calls.lock().expect("mock lock").clone()
    }
}

impl SourceSeam for MockSourceSeam {
    async fn survey(&self, id: String) -> Result<Vec<Lead>, Error> {
        self.calls.lock().expect("mock lock").push(SourceCall::Survey { id });
        self.surveys
            .lock()
            .expect("mock lock")
            .pop_front()
            .expect("MockSourceSeam exhausted: script more survey replies")
    }

    async fn extract(&self, id: String, lead: Lead) -> Result<Evidence, Error> {
        self.calls.lock().expect("mock lock").push(SourceCall::Extract { id, lead });
        self.extracts
            .lock()
            .expect("mock lock")
            .pop_front()
            .expect("MockSourceSeam exhausted: script more extract replies")
    }
}

/// One recorded [`MockTargetSeam`] call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetCall {
    /// A `guidance(id)` dispatch.
    Guidance {
        /// The routed adapter id (e.g. `target:omnia`).
        id: String,
    },
    /// A `build(id, slice, inputs, tree)` dispatch.
    Build {
        /// The routed adapter id.
        id: String,
        /// The slice the build named.
        slice: String,
        /// The artifact inputs handed across the seam.
        inputs: Vec<Input>,
        /// The working tree the build named.
        tree: WorkingTree,
    },
}

/// Scripted [`TargetSeam`] provider for native tests.
#[derive(Debug, Default)]
pub struct MockTargetSeam {
    guidances: Mutex<VecDeque<Result<String, Error>>>,
    builds: Mutex<VecDeque<Result<BuildReport, Error>>>,
    calls: Mutex<Vec<TargetCall>>,
}

impl MockTargetSeam {
    /// A mock answering each `guidance` / `build` call with the next
    /// scripted result of the matching operation.
    #[must_use]
    pub fn scripted(
        guidances: impl IntoIterator<Item = Result<String, Error>>,
        builds: impl IntoIterator<Item = Result<BuildReport, Error>>,
    ) -> Self {
        Self {
            guidances: Mutex::new(guidances.into_iter().collect()),
            builds: Mutex::new(builds.into_iter().collect()),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Every call the mock has served, in call order.
    ///
    /// # Panics
    ///
    /// Panics when the interior lock is poisoned — only possible after
    /// a prior panic in the same test.
    #[must_use]
    pub fn calls(&self) -> Vec<TargetCall> {
        self.calls.lock().expect("mock lock").clone()
    }
}

impl TargetSeam for MockTargetSeam {
    async fn guidance(&self, id: String) -> Result<String, Error> {
        self.calls.lock().expect("mock lock").push(TargetCall::Guidance { id });
        self.guidances
            .lock()
            .expect("mock lock")
            .pop_front()
            .expect("MockTargetSeam exhausted: script more guidance replies")
    }

    async fn build(
        &self, id: String, slice: String, inputs: Vec<Input>, tree: WorkingTree,
    ) -> Result<BuildReport, Error> {
        self.calls.lock().expect("mock lock").push(TargetCall::Build {
            id,
            slice,
            inputs,
            tree,
        });
        self.builds
            .lock()
            .expect("mock lock")
            .pop_front()
            .expect("MockTargetSeam exhausted: script more build replies")
    }
}
