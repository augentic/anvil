//! Linked-adapter catalog: a typed vtable over the per-axis operations
//! traits.
//!
//! Consumers declare their linked adapters once through [`Builder`]
//! (`Catalog::builder().source::<A>()...target::<B>().build()`); each
//! call monomorphizes the implementor's operation legs into fn pointers,
//! so dispatch stays compile-checked trait calls while the catalog
//! itself is plain data the provider routes ids over.

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use adapter::registry::Doc;
use adapter::seam::{self as aseam, Context};
use adapter::{Source, Target, references};
use error::Error;
use omnia_guest::Model;
#[doc(hidden)]
pub use omnia_guest::Model as CatalogModel;
use project::adapter::Axis;
use project::adapter::metadata::Metadata;

use crate::convert;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

type SurveyFn<M> =
    for<'a> fn(&'a M, &'a Context<'a>) -> BoxFuture<'a, Result<Vec<aseam::Lead>, aseam::Error>>;
type GuidanceFn<M> =
    for<'a> fn(&'a M, &'a Context<'a>) -> BoxFuture<'a, Result<String, aseam::Error>>;
type ExtractFn<M> = for<'a> fn(
    &'a M,
    &'a Context<'a>,
    &'a aseam::Lead,
) -> BoxFuture<'a, Result<aseam::Evidence, aseam::Error>>;
type BuildFn<M> = for<'a> fn(
    &'a M,
    &'a Context<'a>,
    &'a str,
    &'a [aseam::Input],
    &'a aseam::WorkingTree,
) -> BoxFuture<'a, Result<aseam::Report, aseam::Error>>;
type MergeFn<M> = for<'a> fn(
    &'a M,
    &'a Context<'a>,
    &'a str,
    aseam::MergePhase,
    &'a aseam::WorkingTree,
) -> BoxFuture<'a, Result<aseam::Report, aseam::Error>>;

/// One repository's linked-adapter declaration.
///
/// The single hook a wrapper implements to bind its concrete adapters
/// into the shared harness (the engine's eval crate binds the fixture
/// registry; `engine` binds the first-party adapters). Generic
/// over the model backend so one declaration serves every provider
/// shape the trial, scenario, command, and HTTP entrypoints construct.
pub trait Binding {
    /// The linked-adapter catalog over model backend `M`.
    fn catalog<M: Model>() -> Catalog<M>;
}

/// The monomorphized operation legs of one linked adapter.
enum Ops<M> {
    Source { survey: SurveyFn<M>, extract: ExtractFn<M> },
    Target { guidance: GuidanceFn<M>, build: BuildFn<M>, merge: MergeFn<M> },
}

impl<M> Clone for Ops<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M> Copy for Ops<M> {}

/// One Rust adapter crate linked into the native harness.
pub struct Entry<M> {
    axis: Axis,
    name: &'static str,
    server_name: &'static str,
    metadata: fn() -> Metadata,
    docs: fn() -> &'static [Doc],
    ops: Ops<M>,
}

impl<M> Clone for Entry<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M> Copy for Entry<M> {}

impl<M> fmt::Debug for Entry<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field("axis", &self.axis)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl<M> Entry<M> {
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

    /// MCP server name.
    #[must_use]
    pub const fn server_name(&self) -> &'static str {
        self.server_name
    }

    /// Routed adapter id (`<axis>:<name>`).
    #[must_use]
    pub fn id(&self) -> String {
        format!("{}:{}", self.axis, self.name)
    }

    /// Adapter metadata projected onto the workflow shape.
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

/// The linked adapters behind one harness instantiation, generic over
/// the model backend the operation legs receive.
pub struct Catalog<M> {
    entries: Vec<Entry<M>>,
}

impl<M> Clone for Catalog<M> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
        }
    }
}

impl<M> fmt::Debug for Catalog<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Catalog").field("entries", &self.entries).finish()
    }
}

impl<M> Catalog<M> {
    /// An empty catalog builder.
    #[must_use]
    pub const fn builder() -> Builder<M> {
        Builder { entries: Vec::new() }
    }

    /// Every linked adapter, in declaration order.
    #[must_use]
    pub fn entries(&self) -> &[Entry<M>] {
        &self.entries
    }

    /// Look up a linked adapter by axis and name.
    ///
    /// # Errors
    ///
    /// Returns `adapter-not-found` when the catalog has no matching entry.
    pub fn get(&self, axis: Axis, name: &str) -> Result<Entry<M>, Error> {
        self.entries
            .iter()
            .copied()
            .find(|entry| entry.axis == axis && entry.name == name)
            .ok_or_else(|| Error::Diag {
                code: "adapter-not-found",
                detail: format!(
                    "adapter `{name}` (axis `{axis}`) is not linked into the native harness"
                ),
            })
    }

    fn find(&self, id: &str) -> Option<&Entry<M>> {
        self.entries.iter().find(|entry| entry.id() == id)
    }
}

impl<M: Model> Catalog<M> {
    /// Dispatch `guidance` to the linked target adapter behind `id`.
    ///
    /// # Errors
    ///
    /// Returns the adapter's failure, or `invalid-request` when `id`
    /// routes to no linked target.
    pub async fn guidance(
        &self, model: &M, ctx: &Context<'_>, id: &str,
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
    pub async fn survey(
        &self, model: &M, ctx: &Context<'_>, id: &str,
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
    pub async fn extract(
        &self, model: &M, ctx: &Context<'_>, id: &str, lead: &aseam::Lead,
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
    pub async fn build(
        &self, model: &M, ctx: &Context<'_>, id: &str, slice: &str, inputs: &[aseam::Input],
        tree: &aseam::WorkingTree,
    ) -> Result<aseam::Report, aseam::Error> {
        match self.find(id).map(|entry| entry.ops) {
            Some(Ops::Target { build, .. }) => build(model, ctx, slice, inputs, tree).await,
            _ => Err(unlinked(id)),
        }
    }

    /// Dispatch one `merge` gate to the linked target adapter behind `id`.
    ///
    /// # Errors
    ///
    /// Returns the adapter's failure, or `invalid-request` when `id`
    /// routes to no linked target.
    pub async fn merge(
        &self, model: &M, ctx: &Context<'_>, id: &str, slice: &str, phase: aseam::MergePhase,
        tree: &aseam::WorkingTree,
    ) -> Result<aseam::Report, aseam::Error> {
        match self.find(id).map(|entry| entry.ops) {
            Some(Ops::Target { merge, .. }) => merge(model, ctx, slice, phase, tree).await,
            _ => Err(unlinked(id)),
        }
    }
}

/// Accumulates linked adapters into a [`Catalog`].
pub struct Builder<M> {
    entries: Vec<Entry<M>>,
}

impl<M> fmt::Debug for Builder<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Builder").field("entries", &self.entries).finish()
    }
}

impl<M: Model> Builder<M> {
    /// Link one source implementor.
    #[must_use]
    pub fn source<A: Source + 'static>(mut self) -> Self {
        self.entries.push(Entry {
            axis: Axis::Source,
            name: A::NAME,
            server_name: references::server_name(A::NAME),
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
            name: A::NAME,
            server_name: references::server_name(A::NAME),
            metadata: || convert::target_metadata(A::metadata()),
            docs: A::docs,
            ops: Ops::Target {
                guidance: |model, ctx| Box::pin(A::guidance(model, ctx)),
                build: |model, ctx, slice, inputs, tree| {
                    Box::pin(A::build(model, ctx, slice, inputs, tree))
                },
                merge: |model, ctx, slice, phase, tree| {
                    Box::pin(A::merge(model, ctx, slice, phase, tree))
                },
            },
        });
        self
    }

    /// The finished catalog.
    #[must_use]
    pub fn build(self) -> Catalog<M> {
        Catalog {
            entries: self.entries,
        }
    }
}

fn unlinked(id: &str) -> aseam::Error {
    aseam::Error::InvalidRequest(format!("adapter `{id}` is not linked into the native shim"))
}

/// Declare the only repository-specific part of a native harness
/// binary: its statically linked adapters.
#[macro_export]
macro_rules! adapters {
    (
        $visibility:vis $name:ident {
            $($axis:ident $adapter:ty),+ $(,)?
        }
    ) => {
        #[doc = concat!("Linked adapter binding `", stringify!($name), "`.")]
        #[derive(Clone, Copy, Debug)]
        $visibility struct $name;

        impl $crate::catalog::Binding for $name {
            fn catalog<M: $crate::catalog::CatalogModel>() -> $crate::catalog::Catalog<M> {
                let builder = $crate::catalog::Catalog::builder();
                $(
                    let builder = $crate::adapters!(@register builder, $axis, $adapter);
                )+
                builder.build()
            }
        }
    };
    (@register $builder:ident, source, $adapter:ty) => {
        $builder.source::<$adapter>()
    };
    (@register $builder:ident, target, $adapter:ty) => {
        $builder.target::<$adapter>()
    };
}
