//! Target wave manifests at
//! `.emery/change/targets/<target>/waves/<digest>.yaml` — a frozen
//! same-target member antichain (RFC-96 D7), `target.wave.opened`.

use std::path::{Path, PathBuf};

use diagnostics::digest::sha256_hex;
use error::Error;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::build_record::BuildRecord;
use crate::config::Layout;
use crate::journal::{self, Event, EventKind, FactEpochRef};
use crate::name::SliceName;
use crate::snapshot::SnapshotId;

mod accepted;

pub use accepted::{accepted_cid, wave_base};

/// Fact-log identity of a `plan.execute.started` authorization epoch
/// (`writer` + 1-based `sequence` of that line in the union).
pub type EpochRef = FactEpochRef;

/// Exact inputs one wave member consumed when the wave opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MemberInputs {
    /// Refinement digest of the member's covered manifest (`sha256:…`).
    pub refinement: SnapshotId,
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

/// Immutable target-wave manifest (RFC-86 D9, RFC-96 D7).
///
/// On disk under `.emery/change/targets/<target>/waves/<digest>.yaml` where
/// `digest` is the bare hex of the canonical YAML bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Wave {
    /// Target key (current project name in the in-place cut).
    pub target: String,
    /// Target-base tree identity selected when the wave opened
    /// (the current accepted CID, or `plan.yaml.targets[].cid`).
    pub base: SnapshotId,
    /// Ordered member set, frozen before claims and builds — never
    /// shrunk (RFC-96 D7).
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
    /// Build a wave over a frozen member antichain (RFC-96 D7).
    #[must_use]
    pub fn new(
        target: impl Into<String>, base: SnapshotId, members: Vec<Member>,
        depends_on: Vec<SliceName>, build_authorization: EpochRef,
    ) -> Self {
        Self {
            target: target.into(),
            base,
            members,
            depends_on,
            build_authorization,
        }
    }

    /// [`Self::new`] over a single member — the cap-one shape.
    #[must_use]
    pub fn one_member(
        target: impl Into<String>, base: SnapshotId, slice: SliceName, refinement: SnapshotId,
        depends_on: Vec<SliceName>, build_authorization: EpochRef,
    ) -> Self {
        Self::new(
            target,
            base,
            vec![Member {
                slice,
                inputs: MemberInputs { refinement },
            }],
            depends_on,
            build_authorization,
        )
    }

    /// Refuse an empty member set — a wave freezes at least one member.
    fn enforce_members(&self) -> Result<(), Error> {
        if self.members.is_empty() {
            return Err(Error::Diag {
                code: "target-wave-member-count",
                detail: format!("target wave for `{}` must have at least one member", self.target),
            });
        }
        Ok(())
    }

    /// Whether every member's live refinement manifest still matches
    /// the digest frozen into this wave. A re-refined, dropped, or
    /// archived member retracts the whole uncommitted wave — no
    /// prefix is authoritative (RFC-96 D7).
    ///
    /// # Errors
    ///
    /// Manifest read failures other than absence.
    pub fn members_fresh(&self, layout: Layout<'_>) -> Result<bool, Error> {
        for member in &self.members {
            let live = crate::refinement::file_digest(&layout.slice_dir(member.slice.as_str()))?;
            if live.as_ref() != Some(&member.inputs.refinement) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Whether every member holds a build record for this wave — the
    /// commit precondition (a wave commits only after every frozen
    /// member passes).
    ///
    /// # Errors
    ///
    /// Record read failures; `slice-build-record-ambiguous` on
    /// duplicates.
    pub fn records_complete(&self, layout: Layout<'_>) -> Result<bool, Error> {
        let digest = self.digest()?;
        for member in &self.members {
            let slice_dir = layout.slice_dir(member.slice.as_str());
            if BuildRecord::find_for_wave(&slice_dir, &digest)?.is_none() {
                return Ok(false);
            }
        }
        Ok(true)
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
        self.enforce_members()?;
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

    /// Persist this wave and append `target.wave.opened` carrying the
    /// frozen member list.
    ///
    /// # Errors
    ///
    /// Member-count gate, write failures, or journal append failures.
    pub fn open(&self, layout: Layout<'_>, now: Timestamp) -> Result<Opened, Error> {
        let digest = self.write(layout)?;
        journal::append_one(
            layout,
            &Event::new(
                now,
                EventKind::TargetWaveOpened {
                    target: self.target.clone(),
                    digest: digest.as_str().to_string(),
                    members: self.members.iter().map(|member| member.slice.clone()).collect(),
                },
            ),
        )?;
        Ok(Opened {
            path: layout.target_wave_path(&self.target, digest.digest()),
            digest,
        })
    }

    /// Revalidate this manifest against a build record at merge time
    /// (RFC-86 D9): `slice` is a named member, content digest matching
    /// `record.wave`, and `base` matching the recorded build base.
    ///
    /// # Errors
    ///
    /// `target-wave-member-mismatch`, `target-wave-digest-mismatch`, or
    /// `target-wave-base-mismatch`.
    pub fn revalidate(&self, slice: &str, record: &BuildRecord) -> Result<(), Error> {
        if !self.members.iter().any(|member| member.slice.as_str() == slice) {
            return Err(Error::Diag {
                code: "target-wave-member-mismatch",
                detail: format!(
                    "wave for target `{}` does not name `{slice}` as a member",
                    self.target
                ),
            });
        }
        let digest = self.digest()?;
        if digest != record.wave {
            return Err(Error::Diag {
                code: "target-wave-digest-mismatch",
                detail: format!(
                    "wave digest `{digest}` does not match build record wave `{}`",
                    record.wave
                ),
            });
        }
        if self.base != record.base {
            return Err(Error::Diag {
                code: "target-wave-base-mismatch",
                detail: format!(
                    "wave base `{}` does not match build record base `{}`",
                    self.base, record.base
                ),
            });
        }
        Ok(())
    }

    /// Load the wave named by a build record under `layout`'s target key.
    ///
    /// # Errors
    ///
    /// Propagates [`Self::load`] / [`Self::revalidate`].
    pub fn load_for_merge(
        layout: Layout<'_>, target: &str, slice: &str, record: &BuildRecord,
    ) -> Result<Self, Error> {
        let wave = Self::load(layout, target, record.wave.as_str())?;
        wave.revalidate(slice, record)?;
        Ok(wave)
    }

    /// Load each member's build record for this wave, in stable member
    /// order. Refuses when any member result is missing.
    ///
    /// # Errors
    ///
    /// `slice-build-record-missing` (or `-ambiguous`) per member;
    /// `target-wave-base-mismatch` when a record's base is not this
    /// wave's base.
    pub fn load_member_records(
        &self, layout: Layout<'_>,
    ) -> Result<Vec<(Member, BuildRecord)>, Error> {
        let digest = self.digest()?;
        let mut out = Vec::with_capacity(self.members.len());
        for member in &self.members {
            let slice_dir = layout.slice_dir(member.slice.as_str());
            let record = BuildRecord::load_for_wave(&slice_dir, &digest)?;
            if self.base != record.base {
                return Err(Error::Diag {
                    code: "target-wave-base-mismatch",
                    detail: format!(
                        "wave base `{}` does not match build record base `{}` for member `{}`",
                        self.base, record.base, member.slice
                    ),
                });
            }
            out.push((member.clone(), record));
        }
        Ok(out)
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
