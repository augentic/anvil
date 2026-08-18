//! Linked-adapter catalog: validated identity, metadata, and embedded
//! docs per linked source implementor. Lookup is always axis-qualified.

use std::fmt;

use adapter::registry::Doc;
use adapter::{Source, references};
use project::adapter::Axis;
use project::adapter::metadata::Metadata;

use crate::convert;
use crate::error::Error;

/// One Rust adapter crate linked into the host.
#[derive(Clone, Copy)]
pub struct Entry {
    axis: Axis,
    name: &'static str,
    version: &'static str,
    server_name: &'static str,
    metadata: fn() -> Metadata,
    docs: fn() -> &'static [Doc],
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field("axis", &self.axis)
            .field("name", &self.name)
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}

impl Entry {
    /// Adapter axis.
    #[must_use]
    pub const fn axis(&self) -> Axis {
        self.axis
    }

    /// Axis-local adapter name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Exact compiled adapter version (`AdapterIdentity.version`).
    #[must_use]
    pub const fn version(&self) -> &'static str {
        self.version
    }

    /// MCP server name.
    #[must_use]
    pub const fn server_name(&self) -> &'static str {
        self.server_name
    }

    /// Exact routed adapter id (`<axis>:<name>@<version>`) — the
    /// identity a resolved native adapter dispatches by.
    #[must_use]
    pub fn id(&self) -> String {
        format!("{}:{}@{}", self.axis, self.name, self.version)
    }

    /// Adapter metadata projected onto the engine shape.
    #[must_use]
    pub fn metadata(&self) -> Metadata {
        (self.metadata)()
    }

    /// Embedded prose documents.
    #[must_use]
    pub fn docs(&self) -> &'static [Doc] {
        (self.docs)()
    }
}

/// The linked adapters behind one host instantiation.
#[derive(Clone)]
pub struct Catalog {
    entries: Vec<Entry>,
}

impl fmt::Debug for Catalog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Catalog").field("entries", &self.entries).finish()
    }
}

impl Catalog {
    /// An empty catalog builder.
    #[must_use]
    pub const fn builder() -> Builder {
        Builder { entries: Vec::new() }
    }

    /// Every linked adapter, in declaration order — the read-only
    /// inventory for diagnostics and build information.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Look up a linked adapter by axis and name.
    ///
    /// # Errors
    ///
    /// Returns `adapter-not-linked` when the catalog has no matching
    /// entry, naming the available identities on that axis.
    pub fn get(&self, axis: Axis, name: &str) -> Result<Entry, error::Error> {
        self.entries
            .iter()
            .copied()
            .find(|entry| entry.axis == axis && entry.name == name)
            .ok_or_else(|| error::Error::Diag {
                code: "adapter-not-linked",
                detail: format!(
                    "adapter `{name}` (axis `{axis}`) is not linked into this host; linked \
                     identities on that axis: [{}]",
                    self.axis_inventory(axis),
                ),
            })
    }

    /// The `name@version` inventory of one axis, for refusal details.
    #[must_use]
    pub(crate) fn axis_inventory(&self, axis: Axis) -> String {
        let names: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| entry.axis == axis)
            .map(|entry| format!("{}@{}", entry.name, entry.version))
            .collect();
        names.join(", ")
    }
}

/// Accumulates linked adapters into a validated [`Catalog`].
pub struct Builder {
    entries: Vec<Entry>,
}

impl fmt::Debug for Builder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Builder").field("entries", &self.entries).finish()
    }
}

impl Builder {
    /// Link one source implementor.
    #[must_use]
    pub fn source<A: Source + 'static>(mut self) -> Self {
        self.entries.push(Entry {
            axis: Axis::Source,
            name: A::IDENTITY.name,
            version: A::IDENTITY.version,
            server_name: references::server_name(A::IDENTITY.name),
            metadata: || convert::source_metadata(A::metadata()),
            docs: A::docs,
        });
        self
    }

    /// The finished, validated catalog.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Catalog`] for a malformed identity name, a
    /// version that is not exact SemVer, or a per-axis duplicate.
    pub fn build(self) -> Result<Catalog, Error> {
        for (index, entry) in self.entries.iter().enumerate() {
            validate_identity(entry)?;
            for earlier in &self.entries[..index] {
                if earlier.name == entry.name && earlier.axis == entry.axis {
                    return Err(Error::Catalog {
                        detail: format!("duplicate `{}` entry `{}`", entry.axis, entry.name),
                    });
                }
            }
        }
        Ok(Catalog {
            entries: self.entries,
        })
    }
}

fn validate_identity(entry: &Entry) -> Result<(), Error> {
    if !is_kebab_name(entry.name) {
        return Err(Error::Catalog {
            detail: format!(
                "adapter identity `{}` (axis `{}`) is not a kebab-case name",
                entry.name, entry.axis
            ),
        });
    }
    if semver::Version::parse(entry.version).is_err() {
        return Err(Error::Catalog {
            detail: format!(
                "adapter identity `{}` (axis `{}`) carries version `{}`, which is not exact \
                 SemVer",
                entry.name, entry.axis, entry.version
            ),
        });
    }
    Ok(())
}

/// `^[a-z][a-z0-9-]*$` — a kebab-case adapter name.
fn is_kebab_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
