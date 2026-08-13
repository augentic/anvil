//! Closed `decomposition.yaml` DTO, canonical digest, and compiled caps.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use artifacts::atomic::yaml_write;
use diagnostics::digest::sha256_hex;
use error::Error;
use serde::{Deserialize, Serialize};

use crate::name::SliceName;
use crate::profile::{Profile, Thresholds, Weights};
use crate::snapshot::SnapshotId;

/// Wire version stamped into every decomposition document.
pub const VERSION: u32 = 1;

/// Maximum containment depth (root is 1). Declared starting value.
pub const MAX_DEPTH: usize = 8;

/// Maximum node count. Declared starting value.
pub const MAX_NODES: usize = 64;

/// Maximum split/leaf judgment dispatches, including focused-survey
/// requeue. Enforced by the authoring loop, not this DTO.
pub const MAX_JUDGMENTS: usize = 128;

/// Conflict-domain hierarchy plus the inputs the leaf projector reads.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Decomposition {
    /// Wire version ([`VERSION`]).
    pub version: u32,
    /// Canonical digest of the bound `leads.md` revision.
    pub leads_digest: SnapshotId,
    /// Per-target closed profile body plus its canonical digest.
    #[serde(rename = "model-capability-profiles")]
    pub profiles: BTreeMap<String, BoundProfile>,
    /// Stable id of the root node.
    pub root: String,
    /// Hierarchy keyed by stable node id.
    pub nodes: BTreeMap<String, Node>,
}

/// Profile body recorded beside its digest. The digest covers
/// [`Profile`] only — it is not a field of that hashed body.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BoundProfile {
    /// Profile id.
    pub id: String,
    /// Canonical digest of the closed [`Profile`] body.
    pub digest: SnapshotId,
    /// Wire version of the closed body.
    pub version: u32,
    /// Per-dimension weights.
    pub weights: Weights,
    /// Operation thresholds.
    pub thresholds: Thresholds,
}

/// One conflict-domain or terminal node.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Node {
    /// Child node ids. Empty on a leaf.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<String>,
    /// Parent node id. Absent on the root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Contributing `(source, lead)` scopes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Scope>,
    /// Singular target binding (leaves; single-target domains).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Multi-target binding on an internal domain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<String>,
    /// Ownership envelope (path globs). Required on leaves.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ownership: Vec<String>,
    /// Domain or leaf predecessors (`depends-on`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// Explicit split/leaf kind. Inferred from children when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<Kind>,
    /// Terminal slice name. Required on leaves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice: Option<SliceName>,
    /// Reviewable acceptance boundary. Required on leaves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<String>,
}

/// One contributing `(source, lead)` pair.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Scope {
    /// Plan source key.
    pub source: String,
    /// Catalog lead id within that source.
    pub lead: String,
}

/// Local gate kind recorded on a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// Internal partition.
    Split,
    /// Terminal domain.
    Leaf,
}

impl BoundProfile {
    /// Record `profile`'s closed body plus [`Profile::digest`].
    ///
    /// # Errors
    ///
    /// YAML serialization failures from [`Profile::digest`].
    pub fn capture(profile: &Profile) -> Result<Self, Error> {
        Ok(Self {
            id: profile.id.clone(),
            digest: profile.digest()?,
            version: profile.version,
            weights: profile.weights,
            thresholds: profile.thresholds,
        })
    }

    /// Closed body this record hashes.
    #[must_use]
    pub fn body(&self) -> Profile {
        Profile {
            id: self.id.clone(),
            version: self.version,
            weights: self.weights,
            thresholds: self.thresholds,
        }
    }
}

impl Node {
    /// An internal node with `children` and no terminal fields.
    #[must_use]
    pub fn split(children: impl Into<Vec<String>>) -> Self {
        Self {
            children: children.into(),
            kind: Some(Kind::Split),
            ..Self::default()
        }
    }

    /// A terminal node bound to `target` and mapped to `slice`.
    #[must_use]
    pub fn leaf(target: impl Into<String>, slice: impl Into<SliceName>) -> Self {
        Self {
            target: Some(target.into()),
            slice: Some(slice.into()),
            kind: Some(Kind::Leaf),
            ..Self::default()
        }
    }

    /// Whether this node has no children.
    #[must_use]
    pub const fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Union of [`Self::target`] and [`Self::targets`].
    #[must_use]
    pub fn target_set(&self) -> BTreeSet<&str> {
        let mut set = BTreeSet::new();
        if let Some(target) = &self.target {
            set.insert(target.as_str());
        }
        set.extend(self.targets.iter().map(String::as_str));
        set
    }
}

impl Scope {
    /// Structured `(source, lead)` pair.
    #[must_use]
    pub fn new(source: impl Into<String>, lead: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            lead: lead.into(),
        }
    }
}

impl Decomposition {
    /// Parse YAML, reject unknown fields, and enforce the wire version.
    ///
    /// # Errors
    ///
    /// `decomposition-malformed` on YAML/unknown-field failures;
    /// `decomposition-version` on a wire-version mismatch.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let tree: Self = serde_saphyr::from_str(text).map_err(|err| Error::Diag {
            code: "decomposition-malformed",
            detail: err.to_string(),
        })?;
        if tree.version != VERSION {
            return Err(Error::Diag {
                code: "decomposition-version",
                detail: format!("decomposition version `{}` is not `{VERSION}`", tree.version),
            });
        }
        Ok(tree)
    }

    /// Load and parse a decomposition file.
    ///
    /// # Errors
    ///
    /// Filesystem failures; the same closed-shape errors as [`Self::parse`].
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = std::fs::read_to_string(path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Canonical YAML bytes (trailing newline, stable field order).
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

    /// Atomic write of the canonical document.
    ///
    /// # Errors
    ///
    /// YAML serialization or filesystem failures.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        yaml_write(path, self)
    }

    /// Load when the file exists; `Ok(None)` when it is absent.
    ///
    /// # Errors
    ///
    /// Filesystem failures other than absence; [`Self::parse`] errors.
    pub fn load_opt(path: &Path) -> Result<Option<Self>, Error> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text).map(Some),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(Error::Filesystem {
                op: "read",
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// Blocking findings, or `Ok(())` when the tree is complete.
    ///
    /// # Errors
    ///
    /// `Error::Validation` carrying every blocking rule id's detail.
    pub fn check(&self) -> Result<(), Error> {
        let findings = super::validate::findings(self);
        let Some(first) = findings.first() else {
            return Ok(());
        };
        let detail = findings.iter().map(|item| item.impact.clone()).collect::<Vec<_>>().join("; ");
        Err(Error::Validation {
            code: first.rule_id.clone().unwrap_or_default().into(),
            detail,
        })
    }

    /// Node `id`, or `decomposition-node-unknown`.
    ///
    /// # Errors
    ///
    /// `decomposition-node-unknown` when `id` is absent.
    pub fn node(&self, id: &str) -> Result<&Node, Error> {
        self.nodes.get(id).ok_or_else(|| Error::Diag {
            code: "decomposition-node-unknown",
            detail: format!("no decomposition node `{id}`"),
        })
    }

    /// Slice name of a leaf node. Falls back to the node id.
    ///
    /// # Errors
    ///
    /// `decomposition-node-unknown` when `id` is absent;
    /// `decomposition-not-leaf` when the node has children.
    pub fn leaf_slice(&self, id: &str) -> Result<SliceName, Error> {
        let node = self.node(id)?;
        if !node.is_leaf() {
            return Err(Error::Diag {
                code: "decomposition-not-leaf",
                detail: format!("node `{id}` is not a leaf"),
            });
        }
        Ok(node.slice.clone().unwrap_or_else(|| SliceName::new(id)))
    }

    /// Node id whose `slice` mapping is `name`.
    ///
    /// # Errors
    ///
    /// `decomposition-slice-unknown` when no leaf maps to `name`.
    pub fn leaf_id(&self, name: &str) -> Result<&str, Error> {
        self.nodes
            .iter()
            .find(|(id, node)| {
                node.is_leaf()
                    && node
                        .slice
                        .as_ref()
                        .map_or_else(|| id.as_str() == name, |slice| slice.as_str() == name)
            })
            .map(|(id, _)| id.as_str())
            .ok_or_else(|| Error::Diag {
                code: "decomposition-slice-unknown",
                detail: format!("no leaf maps to slice `{name}`"),
            })
    }

    /// Parent-chain from root to `id`, excluding `id` itself.
    ///
    /// # Errors
    ///
    /// `decomposition-node-unknown` on a broken parent pointer.
    pub fn ancestry(&self, id: &str) -> Result<Vec<String>, Error> {
        let mut chain = Vec::new();
        let mut cursor = self.node(id)?.parent.as_deref();
        while let Some(parent) = cursor {
            chain.push(parent.to_string());
            cursor = self.node(parent)?.parent.as_deref();
            if chain.len() > MAX_DEPTH {
                break;
            }
        }
        chain.reverse();
        Ok(chain)
    }

    /// Terminal descendants of `id` (or `id` itself when it is a leaf).
    ///
    /// # Errors
    ///
    /// `decomposition-node-unknown` when a child id is absent.
    pub fn terminals(&self, id: &str) -> Result<Vec<String>, Error> {
        let mut out = Vec::new();
        self.collect_terminals(id, &mut out)?;
        Ok(out)
    }

    fn collect_terminals(&self, id: &str, out: &mut Vec<String>) -> Result<(), Error> {
        let node = self.node(id)?;
        if node.is_leaf() {
            out.push(id.to_string());
            return Ok(());
        }
        for child in &node.children {
            self.collect_terminals(child, out)?;
        }
        Ok(())
    }

    /// Depth of `id` (root is 1).
    ///
    /// # Errors
    ///
    /// `decomposition-node-unknown` on a broken parent pointer.
    pub fn depth(&self, id: &str) -> Result<usize, Error> {
        Ok(self.ancestry(id)?.len() + 1)
    }
}
