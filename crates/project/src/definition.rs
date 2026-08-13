//! RFC-104 handoff contract: closed DTO, canonical digest, and
//! definition-home read of the current reviewed wave projection.

mod handoff;
mod resolve;

use std::path::{Path, PathBuf};

pub use handoff::{Architecture, Handoff, INTENT, Mapping, Ref, Scope, Target, VERSION, Wave};
pub use resolve::{Reviewed, resolve};

use crate::snapshot::SnapshotId;

/// A definition home: `handoffs/` plus a separate `events/` root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Home<'a> {
    root: &'a Path,
}

impl<'a> Home<'a> {
    /// Wrap `root` as a definition home. No marker file is required.
    #[must_use]
    pub const fn new(root: &'a Path) -> Self {
        Self { root }
    }

    /// Absolute definition-home directory.
    #[must_use]
    pub const fn root(&self) -> &'a Path {
        self.root
    }

    /// `<root>/handoffs/`.
    #[must_use]
    pub fn handoffs_dir(&self) -> PathBuf {
        self.root.join("handoffs")
    }

    /// `<root>/events/`.
    #[must_use]
    pub fn events_dir(&self) -> PathBuf {
        self.root.join("events")
    }

    /// Content-addressed handoff path: `handoffs/<hex>.yaml`.
    #[must_use]
    pub fn handoff_path(&self, digest: &SnapshotId) -> PathBuf {
        self.handoffs_dir().join(format!("{}.yaml", digest.digest()))
    }
}
