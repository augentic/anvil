//! Linked-adapter catalog: a validated, typed vtable over the per-axis
//! operations traits.
//!
//! Consumers declare their linked adapters once
//! (`Catalog::builder().source::<A>()…target::<B>().build()?`); each
//! registration monomorphizes the implementor's operation legs into fn
//! pointers at [`DynModel`], so dispatch stays compile-checked trait
//! calls while the catalog itself is plain data the provider routes
//! ids over. `build()` validates identities, per-axis duplicates, and
//! reference-shelf coherence; same-name source and target entries
//! remain legal (dispatch is always axis-qualified).
//!
//! Publicly the catalog is construction plus read-only inventory;
//! operation dispatch is crate-private and reachable only through the
//! [`crate::Provider`] seam.

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use adapter::registry::Doc;
use adapter::seam::{self as aseam, Context};
use adapter::{Source, Target, references};
use project::adapter::Axis;
use project::adapter::metadata::Metadata;

use crate::convert;
use crate::error::Error;
use crate::model::DynModel;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

type SurveyFn = for<'a> fn(
    &'a DynModel,
    &'a Context<'a>,
) -> BoxFuture<'a, Result<Vec<aseam::Lead>, aseam::Error>>;
type GuidanceFn =
    for<'a> fn(&'a DynModel, &'a Context<'a>) -> BoxFuture<'a, Result<String, aseam::Error>>;
type ExtractFn = for<'a> fn(
    &'a DynModel,
    &'a Context<'a>,
    &'a aseam::Lead,
) -> BoxFuture<'a, Result<aseam::Evidence, aseam::Error>>;
type BuildFn = for<'a> fn(
    &'a DynModel,
    &'a Context<'a>,
    &'a str,
    &'a [aseam::Input],
    &'a aseam::BuildContext,
    &'a aseam::WorkingTree,
) -> BoxFuture<'a, Result<aseam::Report, aseam::Error>>;
type MergeFn = for<'a> fn(
    &'a DynModel,
    &'a Context<'a>,
    &'a str,
    aseam::MergePhase,
    &'a aseam::WorkingTree,
) -> BoxFuture<'a, Result<aseam::Report, aseam::Error>>;

/// The monomorphized operation legs of one linked adapter.
#[derive(Clone, Copy)]
enum Ops {
    Source { survey: SurveyFn, extract: ExtractFn },
    Target { guidance: GuidanceFn, build: BuildFn, merge: MergeFn },
}

/// One Rust adapter crate linked into the host.
#[derive(Clone, Copy)]
pub struct Entry {
    axis: Axis,
    name: &'static str,
    version: &'static str,
    server_name: &'static str,
    metadata: fn() -> Metadata,
    docs: fn() -> &'static [Doc],
    ops: Ops,
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

    /// Resolve a routed adapter id against the compiled entries: axis
    /// and name must match, and a versioned id
    /// (`<axis>:<name>@<version>`) additionally requires the exact
    /// compiled version. An unversioned id matches the sole compiled
    /// identity for that `(axis, name)`.
    pub(crate) fn find(&self, id: &str) -> Option<&Entry> {
        let routed = project::adapter::RoutedId::parse(id).ok()?;
        self.entries.iter().find(|entry| {
            entry.axis == routed.axis
                && entry.name == routed.name
                && routed.version.as_ref().is_none_or(|version| {
                    semver::Version::parse(entry.version).is_ok_and(|linked| linked == *version)
                })
        })
    }

    /// Dispatch `guidance` to the linked target adapter behind `id`.
    ///
    /// # Errors
    ///
    /// Returns the adapter's failure, or `invalid-request` when `id`
    /// routes to no linked target.
    pub(crate) async fn guidance(
        &self, model: &DynModel, ctx: &Context<'_>, id: &str,
    ) -> Result<String, aseam::Error> {
        match self.find(id).map(|entry| entry.ops) {
            Some(Ops::Target { guidance, .. }) => guidance(model, ctx).await,
            _ => Err(unlinked(id)),
        }
    }

    /// Dispatch `survey` to the linked source adapter behind `id`.
    ///
    /// # Errors
    ///
    /// Returns the adapter's failure, or `invalid-request` when `id`
    /// routes to no linked source.
    pub(crate) async fn survey(
        &self, model: &DynModel, ctx: &Context<'_>, id: &str,
    ) -> Result<Vec<aseam::Lead>, aseam::Error> {
        match self.find(id).map(|entry| entry.ops) {
            Some(Ops::Source { survey, .. }) => survey(model, ctx).await,
            _ => Err(unlinked(id)),
        }
    }

    /// Dispatch `extract` to the linked source adapter behind `id`.
    ///
    /// # Errors
    ///
    /// Returns the adapter's failure, or `invalid-request` when `id`
    /// routes to no linked source.
    pub(crate) async fn extract(
        &self, model: &DynModel, ctx: &Context<'_>, id: &str, lead: &aseam::Lead,
    ) -> Result<aseam::Evidence, aseam::Error> {
        match self.find(id).map(|entry| entry.ops) {
            Some(Ops::Source { extract, .. }) => extract(model, ctx, lead).await,
            _ => Err(unlinked(id)),
        }
    }

    /// Dispatch `build` to the linked target adapter behind `id`.
    ///
    /// # Errors
    ///
    /// Returns the adapter's failure, or `invalid-request` when `id`
    /// routes to no linked target.
    #[expect(
        clippy::too_many_arguments,
        reason = "internal dispatch shim mirroring the seam build signature; one call site"
    )]
    pub(crate) async fn build(
        &self, model: &DynModel, ctx: &Context<'_>, id: &str, slice: &str, inputs: &[aseam::Input],
        context: &aseam::BuildContext, tree: &aseam::WorkingTree,
    ) -> Result<aseam::Report, aseam::Error> {
        match self.find(id).map(|entry| entry.ops) {
            Some(Ops::Target { build, .. }) => {
                build(model, ctx, slice, inputs, context, tree).await
            }
            _ => Err(unlinked(id)),
        }
    }

    /// Dispatch one `merge` gate to the linked target adapter behind `id`.
    ///
    /// # Errors
    ///
    /// Returns the adapter's failure, or `invalid-request` when `id`
    /// routes to no linked target.
    pub(crate) async fn merge(
        &self, model: &DynModel, ctx: &Context<'_>, id: &str, slice: &str,
        phase: aseam::MergePhase, tree: &aseam::WorkingTree,
    ) -> Result<aseam::Report, aseam::Error> {
        match self.find(id).map(|entry| entry.ops) {
            Some(Ops::Target { merge, .. }) => merge(model, ctx, slice, phase, tree).await,
            _ => Err(unlinked(id)),
        }
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
            ops: Ops::Source {
                survey: |model, ctx| Box::pin(A::survey(model, ctx)),
                extract: |model, ctx, lead| Box::pin(A::extract(model, ctx, lead)),
            },
        });
        self
    }

    /// Link one target implementor.
    #[must_use]
    pub fn target<A: Target + 'static>(mut self) -> Self {
        self.entries.push(Entry {
            axis: Axis::Target,
            name: A::IDENTITY.name,
            version: A::IDENTITY.version,
            server_name: references::server_name(A::IDENTITY.name),
            metadata: || convert::target_metadata(A::metadata()),
            docs: A::docs,
            ops: Ops::Target {
                guidance: |model, ctx| Box::pin(A::guidance(model, ctx)),
                build: |model, ctx, slice, inputs, context, tree| {
                    Box::pin(A::build(model, ctx, slice, inputs, context, tree))
                },
                merge: |model, ctx, slice, phase, tree| {
                    Box::pin(A::merge(model, ctx, slice, phase, tree))
                },
            },
        });
        self
    }

    /// The finished, validated catalog.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Catalog`] for a malformed identity name, a
    /// version that is not exact SemVer, a per-axis duplicate, or two
    /// same-name entries with conflicting reference-shelf identities
    /// (dual-axis registrations must share one version and one
    /// embedded docs registry).
    pub fn build(self) -> Result<Catalog, Error> {
        for (index, entry) in self.entries.iter().enumerate() {
            validate_identity(entry)?;
            for earlier in &self.entries[..index] {
                if earlier.name != entry.name {
                    continue;
                }
                if earlier.axis == entry.axis {
                    return Err(Error::Catalog {
                        detail: format!("duplicate `{}` entry `{}`", entry.axis, entry.name),
                    });
                }
                if earlier.version != entry.version || !std::ptr::eq(earlier.docs(), entry.docs()) {
                    return Err(Error::Catalog {
                        detail: format!(
                            "conflicting reference-shelf identities for `{}`: dual-axis \
                             registrations must share one version and one embedded docs registry",
                            entry.name
                        ),
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

fn unlinked(id: &str) -> aseam::Error {
    aseam::Error::InvalidRequest(format!("adapter `{id}` is not linked into this host"))
}
