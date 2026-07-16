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
use project::adapter::metadata::{Metadata, Request as MetadataRequest};
use project::adapter::{Axis, BuildInputDeclaration, PlatformsCapability};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

type SurveyFn<M> =
    for<'a> fn(&'a M, &'a Context<'a>) -> BoxFuture<'a, Result<Vec<aseam::Lead>, aseam::Error>>;
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
/// into the shared harness (the engine's eval crate binds the testkit
/// fixture; `engine` binds the first-party adapters). Generic
/// over the model backend so one declaration serves every provider
/// shape the trial, scenario, command, and HTTP entrypoints construct.
pub trait Binding {
    /// The linked-adapter catalog over model backend `M`.
    fn catalog<M: Model>() -> Catalog<M>;
}

/// The monomorphized operation legs of one linked adapter.
enum Ops<M> {
    Source { survey: SurveyFn<M>, extract: ExtractFn<M> },
    Target { guidance: fn() -> &'static str, build: BuildFn<M>, merge: MergeFn<M> },
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
        format!("{}:{}", self.axis.dir_segment().trim_end_matches('s'), self.name)
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

    /// Serve the linked target adapter's embedded guidance prompt.
    ///
    /// # Errors
    ///
    /// Returns `invalid-request` when `id` routes to no linked target.
    pub fn guidance(&self, id: &str) -> Result<&'static str, aseam::Error> {
        match self.find(id).map(|entry| entry.ops) {
            Some(Ops::Target { guidance, .. }) => Ok(guidance()),
            _ => Err(unlinked(id)),
        }
    }

    /// In-process metadata dispatch used by seam-level parity tests.
    ///
    /// # Errors
    ///
    /// `adapter-metadata-failed` when the request names an adapter this
    /// catalog does not link.
    pub fn metadata(&self, request: &MetadataRequest<'_>) -> Result<Metadata, Error> {
        let name = request.adapter_id.split_once(':').map(|(_, name)| name).unwrap_or_default();
        self.get(request.axis, name).map(|entry| entry.metadata()).map_err(|_error| Error::Diag {
            code: "adapter-metadata-failed",
            detail: format!(
                "adapter `{}` is not linked into the native harness",
                request.adapter_id
            ),
        })
    }

    fn find(&self, id: &str) -> Option<&Entry<M>> {
        self.entries.iter().find(|entry| entry.id() == id)
    }
}

impl<M: Model> Catalog<M> {
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
            metadata: || source_metadata(A::metadata()),
            docs: A::docs,
            ops: Ops::Source {
                survey: survey_leg::<A, M>,
                extract: extract_leg::<A, M>,
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
            metadata: || target_metadata(A::metadata()),
            docs: A::docs,
            ops: Ops::Target {
                guidance: A::guidance,
                build: build_leg::<A, M>,
                merge: merge_leg::<A, M>,
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

fn survey_leg<'a, A: Source + 'static, M: Model>(
    model: &'a M, ctx: &'a Context<'a>,
) -> BoxFuture<'a, Result<Vec<aseam::Lead>, aseam::Error>> {
    Box::pin(A::survey(model, ctx))
}

fn extract_leg<'a, A: Source + 'static, M: Model>(
    model: &'a M, ctx: &'a Context<'a>, lead: &'a aseam::Lead,
) -> BoxFuture<'a, Result<aseam::Evidence, aseam::Error>> {
    Box::pin(A::extract(model, ctx, lead))
}

fn build_leg<'a, A: Target + 'static, M: Model>(
    model: &'a M, ctx: &'a Context<'a>, slice: &'a str, inputs: &'a [aseam::Input],
    tree: &'a aseam::WorkingTree,
) -> BoxFuture<'a, Result<aseam::Report, aseam::Error>> {
    Box::pin(A::build(model, ctx, slice, inputs, tree))
}

fn merge_leg<'a, A: Target + 'static, M: Model>(
    model: &'a M, ctx: &'a Context<'a>, slice: &'a str, phase: aseam::MergePhase,
    tree: &'a aseam::WorkingTree,
) -> BoxFuture<'a, Result<aseam::Report, aseam::Error>> {
    Box::pin(A::merge(model, ctx, slice, phase, tree))
}

fn unlinked(id: &str) -> aseam::Error {
    aseam::Error::InvalidRequest(format!("adapter `{id}` is not linked into the native shim"))
}

fn source_metadata(record: aseam::SourceMetadata) -> Metadata {
    Metadata {
        specify_floor: record.specify_floor,
        inputs: Vec::new(),
        platforms: None,
    }
}

fn target_metadata(record: aseam::TargetMetadata) -> Metadata {
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

const fn platform(platform: aseam::Platform) -> project::platform::Platform {
    use project::platform::Platform;
    match platform {
        aseam::Platform::Core => Platform::Core,
        aseam::Platform::Ios => Platform::Ios,
        aseam::Platform::Android => Platform::Android,
        aseam::Platform::Web => Platform::Web,
        aseam::Platform::Desktop => Platform::Desktop,
    }
}
