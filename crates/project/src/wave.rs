//! One-member target wave manifests (RFC-86 D9).
//!
//! Before build, the engine writes an immutable manifest at
//! `.emery/targets/<target>/waves/<digest>.yaml` naming the target
//! (project name in the in-place cut), pinned base, ordered member set
//! (exactly one member in this cut), exact member inputs, dependency
//! frontier, and build-authorization epoch, then appends
//! `target.wave.opened`. Full build orchestration wires this helper in
//! a later session; this module owns the schema, persistence, and open
//! fact.

use std::path::{Path, PathBuf};

use diagnostics::digest::sha256_hex;
use error::Error;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::config::Layout;
use crate::journal::{self, Event, EventKind};
use crate::name::SliceName;
use crate::snapshot::SnapshotId;

/// Fact-log identity of a `plan.execute.started` authorization epoch
/// (`actor` + 1-based `sequence` of that line in the union).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct EpochRef {
    /// Actor file that holds the epoch fact.
    pub actor: String,
    /// 1-based sequence of the epoch fact in that actor's file.
    pub sequence: u64,
}

/// Exact inputs one wave member consumed when the wave opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MemberInputs {
    /// Content digest of the member's `spec.md` (`sha256:…`).
    pub spec: SnapshotId,
}

/// One ordered member of a target wave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Member {
    /// Slice leaf this member builds.
    pub slice: SliceName,
    /// Exact member inputs bound at open time.
    pub inputs: MemberInputs,
}

/// Immutable target-wave manifest (RFC-86 D9).
///
/// On disk under `.emery/targets/<target>/waves/<digest>.yaml` where
/// `digest` is the bare hex of the canonical YAML bytes. This cut
/// accepts exactly one [`Member`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Wave {
    /// Target key (current project name in the in-place cut).
    pub target: String,
    /// Pinned target-base tree identity consumed by the wave.
    pub base: SnapshotId,
    /// Ordered member set — length must be 1 before open.
    pub members: Vec<Member>,
    /// Dependency frontier: slices whose accepted results this wave
    /// assumes (copied from the leaf's `depends-on` at open time).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<SliceName>,
    /// Build-authorization epoch this wave binds.
    pub build_authorization: EpochRef,
}

/// Result of persisting a wave and appending `target.wave.opened`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    /// Content digest of the written manifest (`sha256:…`).
    pub digest: SnapshotId,
    /// Absolute path of the written YAML file.
    pub path: PathBuf,
}

impl Wave {
    /// Build a one-member wave from the fields D9 requires.
    #[must_use]
    pub fn one_member(
        target: impl Into<String>, base: SnapshotId, slice: SliceName, spec: SnapshotId,
        depends_on: Vec<SliceName>, build_authorization: EpochRef,
    ) -> Self {
        Self {
            target: target.into(),
            base,
            members: vec![Member {
                slice,
                inputs: MemberInputs { spec },
            }],
            depends_on,
            build_authorization,
        }
    }

    /// Refuse anything other than a single member (this cut).
    ///
    /// # Errors
    ///
    /// `target-wave-member-count` when `members.len() != 1`.
    pub fn enforce_one_member(&self) -> Result<(), Error> {
        if self.members.len() == 1 {
            return Ok(());
        }
        Err(Error::Diag {
            code: "target-wave-member-count",
            detail: format!(
                "target wave for `{}` must have exactly one member; found {}",
                self.target,
                self.members.len()
            ),
        })
    }

    /// Canonical YAML bytes of this manifest (trailing newline).
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn canonical_yaml(&self) -> Result<String, Error> {
        artifacts::atomic::serialise_yaml(self)
    }

    /// Content digest of [`Self::canonical_yaml`] as a [`SnapshotId`].
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        Ok(SnapshotId::from_digest(&sha256_hex(self.canonical_yaml()?.as_bytes())))
    }

    /// Absolute path for this wave under `layout`, derived from its
    /// content digest.
    ///
    /// # Errors
    ///
    /// YAML serialization failures while computing the digest.
    pub fn path(&self, layout: Layout<'_>) -> Result<PathBuf, Error> {
        let digest = self.digest()?;
        Ok(layout.target_wave_path(&self.target, digest.digest()))
    }

    /// Atomically write this manifest at its content-addressed path.
    ///
    /// Write-once: an existing file with identical bytes is a no-op; a
    /// different payload at the same digest path is `target-wave-conflict`.
    ///
    /// # Errors
    ///
    /// Member-count gate, YAML / filesystem failures, digest collision.
    pub fn write(&self, layout: Layout<'_>) -> Result<SnapshotId, Error> {
        self.enforce_one_member()?;
        let yaml = self.canonical_yaml()?;
        let digest = SnapshotId::from_digest(&sha256_hex(yaml.as_bytes()));
        let path = layout.target_wave_path(&self.target, digest.digest());
        if path.is_file() {
            let existing = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
                op: "read",
                path: path.clone(),
                source,
            })?;
            if existing == yaml {
                return Ok(digest);
            }
            return Err(Error::Diag {
                code: "target-wave-conflict",
                detail: format!(
                    "wave manifest at `{}` already exists with different bytes",
                    path.display()
                ),
            });
        }
        artifacts::atomic::bytes_write(&path, yaml.as_bytes())?;
        Ok(digest)
    }

    /// Load a previously written wave by target key and digest.
    ///
    /// `digest` accepts bare hex or the `sha256:…` wire form.
    ///
    /// # Errors
    ///
    /// Filesystem / YAML parse failures; `snapshot-id-malformed` for a
    /// bad digest token.
    pub fn load(layout: Layout<'_>, target: &str, digest: &str) -> Result<Self, Error> {
        let id = parse_digest(digest)?;
        let path = layout.target_wave_path(target, id.digest());
        load_path(&path)
    }

    /// Persist this one-member wave and append `target.wave.opened`.
    ///
    /// # Errors
    ///
    /// Member-count gate, write failures, or journal append failures.
    pub fn open(&self, layout: Layout<'_>, now: Timestamp) -> Result<Opened, Error> {
        let digest = self.write(layout)?;
        let slice_name = self
            .members
            .first()
            .ok_or_else(|| Error::Diag {
                code: "target-wave-member-count",
                detail: format!(
                    "target wave for `{}` must have exactly one member; found 0",
                    self.target
                ),
            })?
            .slice
            .clone();
        journal::append_one(
            layout,
            &Event::new(
                now,
                EventKind::TargetWaveOpened {
                    target: self.target.clone(),
                    digest: digest.as_str().to_string(),
                    slice_name,
                },
            ),
        )?;
        Ok(Opened {
            path: layout.target_wave_path(&self.target, digest.digest()),
            digest,
        })
    }
}

fn parse_digest(digest: &str) -> Result<SnapshotId, Error> {
    if digest.starts_with("sha256:") {
        SnapshotId::parse(digest)
    } else {
        SnapshotId::parse(&format!("sha256:{digest}"))
    }
}

fn load_path(path: &Path) -> Result<Wave, Error> {
    let text = std::fs::read_to_string(path).map_err(|source| Error::Filesystem {
        op: "read",
        path: path.to_path_buf(),
        source,
    })?;
    Ok(serde_saphyr::from_str(&text)?)
}
