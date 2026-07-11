//! Native-only catalog of adapter crates linked into `specify-dev`.
//!
//! [`linked!`] is the single declarative table: one `<axis> <name> =>
//! <crate>;` line per adapter generates the catalog [`entries`] (name,
//! MCP server, metadata projection, embedded docs) *and* the
//! per-operation dispatch functions the seam provider calls — adding
//! an adapter is that one line plus its Cargo path dependency.

use adapter::registry::Doc;
use adapter::seam::{self as aseam, Context};
use error::Error;
use omnia_guest::Model;
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

/// The axis token of one table line.
macro_rules! axis_of {
    (source) => {
        Axis::Source
    };
    (target) => {
        Axis::Target
    };
}

/// The metadata projection thunk for one table line, by axis.
macro_rules! metadata_of {
    (source, $krate:ident) => {
        || source_metadata($krate::operations::metadata())
    };
    (target, $krate:ident) => {
        || target_metadata($krate::operations::metadata())
    };
}

/// One source-operation dispatch leg; expands to nothing for targets.
macro_rules! source_leg {
    (source, $name:ident, $krate:ident, $op:ident, ($($arg:expr),+), $id:expr) => {
        if $id == concat!("source:", stringify!($name)) {
            return $krate::operations::$op($($arg),+).await;
        }
    };
    (target, $name:ident, $krate:ident, $op:ident, ($($arg:expr),+), $id:expr) => {};
}

/// One `guidance` dispatch leg; expands to nothing for sources.
macro_rules! guidance_leg {
    (target, $name:ident, $krate:ident, $id:expr) => {
        if $id == concat!("target:", stringify!($name)) {
            return Ok($krate::operations::guidance());
        }
    };
    (source, $name:ident, $krate:ident, $id:expr) => {};
}

/// One `build` dispatch leg; expands to nothing for sources.
macro_rules! build_leg {
    (target, $name:ident, $krate:ident, ($($arg:expr),+), $id:expr) => {
        if $id == concat!("target:", stringify!($name)) {
            return $krate::operations::build($($arg),+).await;
        }
    };
    (source, $name:ident, $krate:ident, ($($arg:expr),+), $id:expr) => {};
}

/// The declarative linked-adapter table: generates [`entries`] and the
/// dispatch functions from one line per adapter.
macro_rules! linked {
    ($( $axis:ident $name:ident => $krate:ident; )+) => {
        /// Every adapter linked into the native shim.
        #[must_use]
        pub fn entries() -> &'static [Entry] {
            static ENTRIES: &[Entry] = &[
                $(
                    Entry {
                        axis: axis_of!($axis),
                        name: stringify!($name),
                        server_name: concat!(stringify!($name), "-references"),
                        metadata: metadata_of!($axis, $krate),
                        docs: $krate::registry::docs,
                    },
                )+
            ];
            ENTRIES
        }

        /// Dispatch `survey` to the linked source adapter behind `id`.
        pub(crate) async fn survey<M: Model>(
            model: &M, ctx: &Context<'_>, id: &str,
        ) -> Result<Vec<aseam::Lead>, aseam::Error> {
            $( source_leg!($axis, $name, $krate, survey, (model, ctx), id); )+
            Err(unlinked(id))
        }

        /// Dispatch `extract` to the linked source adapter behind `id`.
        pub(crate) async fn extract<M: Model>(
            model: &M, ctx: &Context<'_>, id: &str, lead: &aseam::Lead,
        ) -> Result<aseam::Evidence, aseam::Error> {
            $( source_leg!($axis, $name, $krate, extract, (model, ctx, lead), id); )+
            Err(unlinked(id))
        }

        /// Serve the linked target adapter's embedded guidance prompt.
        pub(crate) fn guidance(id: &str) -> Result<&'static str, aseam::Error> {
            $( guidance_leg!($axis, $name, $krate, id); )+
            Err(unlinked(id))
        }

        /// Dispatch `build` to the linked target adapter behind `id`.
        pub(crate) async fn build<M: Model>(
            model: &M, ctx: &Context<'_>, id: &str, slice: &str,
            inputs: &[aseam::Input], tree: &aseam::WorkingTree,
        ) -> Result<aseam::Report, aseam::Error> {
            $( build_leg!($axis, $name, $krate, (model, ctx, slice, inputs, tree), id); )+
            Err(unlinked(id))
        }
    };
}

linked! {
    source captures => captures;
    target contracts => contracts;
    source documentation => documentation;
    source intent => intent;
    target omnia => omnia_target;
    source screenshots => screenshots;
    source typescript => typescript;
    target vectis => vectis;
}

/// A dispatch to an adapter id this shim does not link.
fn unlinked(id: &str) -> aseam::Error {
    aseam::Error::InvalidRequest(format!("adapter `{id}` is not linked into the native shim"))
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

fn source_metadata(record: adapter::seam::SourceMetadata) -> Metadata {
    Metadata {
        specify_floor: record.specify_floor,
        inputs: Vec::new(),
        platforms: None,
    }
}

fn target_metadata(record: adapter::seam::TargetMetadata) -> Metadata {
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
