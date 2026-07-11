//! Native-only catalog of adapter crates linked into `specify-dev`.

use adapter::registry::Doc;
use error::Error;
use workflow::adapter::metadata::Metadata;
use workflow::adapter::{Axis, BuildInputDeclaration, PlatformsCapability};

/// One Rust adapter crate linked into the native shim.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    axis: Axis,
    name: &'static str,
    server_name: &'static str,
    metadata: fn() -> Metadata,
    docs: fn() -> &'static [Doc],
}

impl Entry {
    /// Adapter axis.
    #[must_use]
    pub const fn axis(self) -> Axis {
        self.axis
    }

    /// Axis-local adapter name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// MCP server name.
    #[must_use]
    pub const fn server_name(self) -> &'static str {
        self.server_name
    }

    /// Routed adapter id (`<axis>:<name>`).
    #[must_use]
    pub fn id(self) -> String {
        format!("{}:{}", self.axis.dir_segment().trim_end_matches('s'), self.name)
    }

    /// Adapter metadata projected onto the workflow shape.
    #[must_use]
    pub fn metadata(self) -> Metadata {
        (self.metadata)()
    }

    /// Embedded prose documents.
    #[must_use]
    pub fn docs(self) -> &'static [Doc] {
        (self.docs)()
    }
}

/// Every adapter linked into the native shim.
#[must_use]
pub fn entries() -> &'static [Entry] {
    static ENTRIES: &[Entry] = &[
        Entry {
            axis: Axis::Source,
            name: "captures",
            server_name: "captures-references",
            metadata: captures_metadata,
            docs: captures::registry::docs,
        },
        Entry {
            axis: Axis::Target,
            name: "contracts",
            server_name: "contracts-references",
            metadata: contracts_metadata,
            docs: contracts::registry::docs,
        },
        Entry {
            axis: Axis::Source,
            name: "documentation",
            server_name: "documentation-references",
            metadata: documentation_metadata,
            docs: documentation::registry::docs,
        },
        Entry {
            axis: Axis::Source,
            name: "intent",
            server_name: "intent-references",
            metadata: intent_metadata,
            docs: intent::registry::docs,
        },
        Entry {
            axis: Axis::Target,
            name: "omnia",
            server_name: "omnia-references",
            metadata: omnia_metadata,
            docs: omnia_target::registry::docs,
        },
        Entry {
            axis: Axis::Source,
            name: "screenshots",
            server_name: "screenshots-references",
            metadata: screenshots_metadata,
            docs: screenshots::registry::docs,
        },
        Entry {
            axis: Axis::Source,
            name: "typescript",
            server_name: "typescript-references",
            metadata: typescript_metadata,
            docs: typescript::registry::docs,
        },
        Entry {
            axis: Axis::Target,
            name: "vectis",
            server_name: "vectis-references",
            metadata: vectis_metadata,
            docs: vectis::registry::docs,
        },
    ];
    ENTRIES
}

/// Look up a linked adapter by axis and name.
///
/// # Errors
///
/// Returns `adapter-not-found` when the catalog has no matching entry.
pub fn get(axis: Axis, name: &str) -> Result<Entry, Error> {
    entries().iter().copied().find(|entry| entry.axis == axis && entry.name == name).ok_or_else(
        || Error::Diag {
            code: "adapter-not-found",
            detail: format!("adapter `{name}` (axis `{axis}`) is not linked into specify-dev"),
        },
    )
}

fn source(record: adapter::seam::SourceMetadata) -> Metadata {
    Metadata {
        specify_floor: record.specify_floor,
        inputs: Vec::new(),
        platforms: None,
    }
}

fn target(record: adapter::seam::TargetMetadata) -> Metadata {
    Metadata {
        specify_floor: record.specify_floor,
        inputs: record
            .inputs
            .into_iter()
            .map(|input| BuildInputDeclaration {
                path: input.path,
                required: input.required,
            })
            .collect(),
        platforms: record.platforms.map(|capability| PlatformsCapability {
            required: capability.required,
            allowed: capability.allowed.into_iter().map(platform).collect(),
            default: capability.default.into_iter().map(platform).collect(),
        }),
    }
}

const fn platform(platform: adapter::seam::Platform) -> workflow::platform::Platform {
    use workflow::platform::Platform;
    match platform {
        adapter::seam::Platform::Core => Platform::Core,
        adapter::seam::Platform::Ios => Platform::Ios,
        adapter::seam::Platform::Android => Platform::Android,
        adapter::seam::Platform::Web => Platform::Web,
        adapter::seam::Platform::Desktop => Platform::Desktop,
    }
}

fn captures_metadata() -> Metadata {
    source(captures::operations::metadata())
}
fn contracts_metadata() -> Metadata {
    target(contracts::operations::metadata())
}
fn documentation_metadata() -> Metadata {
    source(documentation::operations::metadata())
}
fn intent_metadata() -> Metadata {
    source(intent::operations::metadata())
}
fn omnia_metadata() -> Metadata {
    target(omnia_target::operations::metadata())
}
fn screenshots_metadata() -> Metadata {
    source(screenshots::operations::metadata())
}
fn typescript_metadata() -> Metadata {
    source(typescript::operations::metadata())
}
fn vectis_metadata() -> Metadata {
    target(vectis::operations::metadata())
}
