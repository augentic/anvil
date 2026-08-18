//! The native seam provider: project anchoring, ensure/resolve over
//! the compiled catalog, and model access. Native ensure is a static
//! package match — misses fail `adapter-not-linked`.

use error::Error;
use omnia_guest::Model;
use omnia_guest::model::{Reply, Request};
use project::adapter::catalog::Catalog as Adapters;
use project::adapter::{
    AdapterSelector, Axis, FIRST_PARTY_NAMESPACE, Inventory, Origin, ResolvedSource,
    ResolvedTarget, Resolver,
};
use project::handler::ExecutionPaths;
use project::profile::{self, Profiles};

use crate::catalog::{Catalog, Entry};
use crate::model::DynModel;

/// The native host provider over a validated [`Catalog`] and an
/// erased [`DynModel`] backend.
///
/// Clones share the model and catalog — `Clone` supports router
/// invocation and shared capabilities, not concurrent independent
/// commands. The provider exposes no model accessor; backends with
/// post-run state hand out caller-held clones before erasure.
#[derive(Clone, Debug)]
pub struct Provider {
    paths: ExecutionPaths,
    model: DynModel,
    catalog: Catalog,
}

impl Provider {
    /// A provider anchored at `paths` over the given model backend
    /// and native adapter catalog.
    #[must_use]
    pub const fn new(paths: ExecutionPaths, model: DynModel, catalog: Catalog) -> Self {
        Self {
            paths,
            model,
            catalog,
        }
    }

    /// The native adapter catalog.
    #[must_use]
    pub const fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Match one selector against the compiled catalog (native
    /// ensure): bare by name, exact pin by `(name, version)`; anything
    /// else refuses as `adapter-not-linked`.
    fn matched(&self, axis: Axis, selector: &AdapterSelector) -> Result<Entry, Error> {
        match selector {
            AdapterSelector::Bare { name } => self.catalog.get(axis, name),
            AdapterSelector::Package {
                namespace,
                name,
                version,
            } => {
                if namespace != FIRST_PARTY_NAMESPACE {
                    return Err(not_linked(format!(
                        "adapter `{selector}` (axis `{axis}`) is not linked into this host: \
                         linked identities publish under the `{FIRST_PARTY_NAMESPACE}` namespace"
                    )));
                }
                let entry = self.catalog.get(axis, name)?;
                let linked = entry_version(&entry)?;
                // Recorded bindings are exact pins, including native
                // mock identities compiled at `0.0.0`. Match the
                // compiled version exactly.
                if linked == *version {
                    Ok(entry)
                } else {
                    Err(not_linked(format!(
                        "adapter `{selector}` (axis `{axis}`) does not match the linked \
                         `{name}@{}`; use a compatible native build or the Wasm deployment",
                        entry.version()
                    )))
                }
            }
            AdapterSelector::Component { path } => Err(not_linked(format!(
                "native execution does not load the supplied component `{}`; linked \
                 identities on axis `{axis}`: [{}]",
                path.display(),
                self.catalog.axis_inventory(axis),
            ))),
        }
    }
}

impl project::handler::Anchor for Provider {
    fn paths(&self) -> &ExecutionPaths {
        &self.paths
    }
}

impl Resolver for Provider {
    fn resolve_source(
        &self, selector: &AdapterSelector, _paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        let entry = self.matched(Axis::Source, selector)?;
        project::adapter::resolver::source(
            entry.name(),
            Some(entry_version(&entry)?),
            entry.metadata(),
            origin(&entry),
        )
    }

    fn resolve_target(
        &self, selector: &AdapterSelector, _paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        let entry = self.matched(Axis::Target, selector)?;
        project::adapter::resolver::target(
            entry.name(),
            Some(entry_version(&entry)?),
            entry.metadata(),
            origin(&entry),
        )
    }
}

impl Inventory for Provider {
    fn inventory(&self) -> &Adapters {
        static FIRST: std::sync::LazyLock<Adapters> =
            std::sync::LazyLock::new(Adapters::first_party);
        &FIRST
    }
}

impl Profiles for Provider {
    fn profiles(&self) -> &profile::Table {
        static FIRST: std::sync::LazyLock<profile::Table> =
            std::sync::LazyLock::new(profile::Table::compiled);
        &FIRST
    }
}

impl Model for Provider {
    async fn create(&self, request: Request) -> Result<Reply, omnia_guest::model::Error> {
        self.model.create(request).await
    }
}

/// The exact compiled version a catalog entry resolves as.
fn entry_version(entry: &Entry) -> Result<semver::Version, Error> {
    semver::Version::parse(entry.version()).map_err(|err| Error::Diag {
        code: "adapter-not-linked",
        detail: format!(
            "linked adapter `{}` carries an invalid version `{}`: {err}",
            entry.name(),
            entry.version()
        ),
    })
}

const fn not_linked(detail: String) -> Error {
    Error::Diag {
        code: "adapter-not-linked",
        detail,
    }
}

fn origin(entry: &Entry) -> Origin {
    Origin {
        label: "native".to_string(),
        reference: format!("rust:{}", entry.id()),
    }
}
