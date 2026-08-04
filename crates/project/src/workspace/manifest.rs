//! Canonical tree manifest: the content-addressed description of one
//! complete product-code tree.
//!
//! A manifest is a sorted, line-oriented text document — one line per
//! file or symlink, directories implicit — whose SHA-256 digest is the
//! tree's [`SnapshotId`](crate::snapshot::SnapshotId). The encoding is
//! canonical (sorted paths, fixed field order), so equal trees always
//! hash equal.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use error::Error;

/// One manifest entry, keyed by its `/`-separated relative path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// A regular file: the blob digest of its content plus the
    /// executable bit — the one mode distinction the contract
    /// round-trips (the Git precedent).
    File { exec: bool, blob: String },
    /// A symbolic link: the blob digest of its target path bytes.
    Link { blob: String },
}

/// A parsed tree manifest: sorted relative paths to entries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    pub entries: BTreeMap<String, Entry>,
}

impl Manifest {
    /// Canonical encoding: one `blob <mode> <digest> <path>` or
    /// `link <digest> <path>` line per entry, in path order.
    pub fn encode(&self) -> String {
        let mut out = String::new();
        for (path, entry) in &self.entries {
            let line = match entry {
                Entry::File { exec, blob } => {
                    let mode = if *exec { "100755" } else { "100644" };
                    writeln!(out, "blob {mode} {blob} {path}")
                }
                Entry::Link { blob } => writeln!(out, "link {blob} {path}"),
            };
            line.expect("writing to a String cannot fail");
        }
        out
    }

    /// Parse the canonical encoding.
    ///
    /// # Errors
    ///
    /// `snapshot-manifest-malformed` on any line that does not match
    /// the canonical shape.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut entries = BTreeMap::new();
        for line in text.lines() {
            let (path, entry) = parse_line(line)?;
            entries.insert(path, entry);
        }
        Ok(Self { entries })
    }

    /// Paths whose entries differ between `self` and `other` —
    /// additions, removals, and content / mode / kind changes —
    /// sorted.
    pub fn diff(&self, other: &Self) -> Vec<String> {
        let mut touched: Vec<String> = self
            .entries
            .iter()
            .filter(|(path, entry)| other.entries.get(*path) != Some(entry))
            .map(|(path, _)| path.clone())
            .collect();
        touched
            .extend(other.entries.keys().filter(|path| !self.entries.contains_key(*path)).cloned());
        touched.sort();
        touched
    }
}

fn parse_line(line: &str) -> Result<(String, Entry), Error> {
    let malformed = || Error::Diag {
        code: "snapshot-manifest-malformed",
        detail: format!("manifest line `{line}` is not canonical"),
    };
    let (kind, rest) = line.split_once(' ').ok_or_else(malformed)?;
    match kind {
        "blob" => {
            let (mode, rest) = rest.split_once(' ').ok_or_else(malformed)?;
            let exec = match mode {
                "100644" => false,
                "100755" => true,
                _ => return Err(malformed()),
            };
            let (blob, path) = rest.split_once(' ').ok_or_else(malformed)?;
            check_digest(blob).ok_or_else(malformed)?;
            Ok((
                path.to_string(),
                Entry::File {
                    exec,
                    blob: blob.to_string(),
                },
            ))
        }
        "link" => {
            let (blob, path) = rest.split_once(' ').ok_or_else(malformed)?;
            check_digest(blob).ok_or_else(malformed)?;
            Ok((
                path.to_string(),
                Entry::Link {
                    blob: blob.to_string(),
                },
            ))
        }
        _ => Err(malformed()),
    }
}

fn check_digest(hex: &str) -> Option<()> {
    (hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))).then_some(())
}
