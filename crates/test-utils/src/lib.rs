//! Harness for the root component suite (`tests/component.rs`).
//!
//! The build script compiles the mock adapter example to a `wasm32-wasip2`
//! component and generates one `pub const` path per fixture. [`run`]
//! assembles by hand what `src/main.rs` declares through `omnia::runtime!`
//! — the engine guest, the source seam, a read-only `.` project mount, the
//! model/storage/otel hosts, and a `.`-rooted path loader — over scripted
//! backends, so a scenario observes exit status and storage handles rather
//! than stdout (which the runtime inherits).

#![cfg(not(target_arch = "wasm32"))]

mod model;

use std::fs;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use emery_adapter::SOURCE_INTERFACE;
use omnia::{
    Backend as _, DeploymentBuilder, ExitStatus, GuestEntry, Manifest, Mode, Mount, NoOptions,
    PathMounts, Plugins, Provides, Runtime, StoreCtx, WasiPlugins, serve_links,
};
use omnia_wasi_blobstore::{BlobstoreDefault, WasiBlobstore, WasiBlobstoreCtx};
use omnia_wasi_keyvalue::{KeyValueDefault, WasiKeyValue, WasiKeyValueCtx};
use omnia_wasi_model::{WasiModel, WasiModelCtx};
use omnia_wasi_otel::{OtelDefault, WasiOtel, WasiOtelCtx};
use tempfile::TempDir;

pub use self::model::{Exchange, ScriptedModel, Seen};

include!(concat!(env!("OUT_DIR"), "/gen.rs"));

/// The keyvalue bucket the engine guest's `StateStore` opens.
pub const STATE_BUCKET: &str = "cache";

/// The store's backend bundle: a scripted model over in-memory storage.
/// Every field is a shared handle, so a clone held by the scenario reads
/// state back after the run.
#[derive(Clone, Debug)]
pub struct Backends {
    /// The scripted model backend.
    pub model: ScriptedModel,
    /// The in-memory `wasi:keyvalue` backend.
    pub keyvalue: KeyValueDefault,
    /// The in-memory `wasi:blobstore` backend.
    pub blobstore: BlobstoreDefault,
    otel: OtelDefault,
}

impl Backends {
    /// A bundle answering with `model` over fresh in-memory storage.
    ///
    /// # Panics
    ///
    /// Panics if an in-memory default cannot be constructed.
    pub async fn scripted(model: ScriptedModel) -> Self {
        Self {
            model,
            keyvalue: KeyValueDefault::connect_with(NoOptions).await.expect("in-memory keyvalue"),
            blobstore: BlobstoreDefault::connect_with(NoOptions)
                .await
                .expect("in-memory blobstore"),
            otel: OtelDefault,
        }
    }

    /// Reads one committed object back through the blobstore handle.
    pub async fn blob(&self, container: &str, object: &str) -> Option<Vec<u8>> {
        let exists = self.blobstore.container_exists(container.to_owned()).await.ok()?;
        if !exists {
            return None;
        }
        let container = self.blobstore.get_container(container.to_owned()).await.ok()?;
        container.get_data(object.to_owned(), 0, u64::MAX).await.ok().flatten().map(Into::into)
    }

    /// Reads one state entry back through the keyvalue handle.
    pub async fn state(&self, key: &str) -> Option<Vec<u8>> {
        let bucket = self.keyvalue.open_bucket(STATE_BUCKET.to_owned()).await.ok()?;
        bucket.get(key.to_owned()).await.ok().flatten()
    }
}

impl Provides<WasiModel> for Backends {
    fn borrow(&mut self) -> &mut dyn WasiModelCtx {
        &mut self.model
    }
}

impl Provides<WasiKeyValue> for Backends {
    fn borrow(&mut self) -> &mut dyn WasiKeyValueCtx {
        &mut self.keyvalue
    }
}

impl Provides<WasiBlobstore> for Backends {
    fn borrow(&mut self) -> &mut dyn WasiBlobstoreCtx {
        &mut self.blobstore
    }
}

impl Provides<WasiOtel> for Backends {
    fn borrow(&mut self) -> &mut dyn WasiOtelCtx {
        &mut self.otel
    }
}

/// One command-mode deployment of the engine guest.
#[derive(Clone, Copy, Debug)]
pub struct Deployment<'a> {
    /// Path to the engine component.
    pub engine: &'a str,
    /// CLI arguments after the program name (`["specify", "greeting"]`).
    pub argv: &'a [&'a str],
    /// The project directory, mounted read-only as `.` and serving the
    /// path-load slot.
    pub project: &'a Scratch,
    /// Statically declared `(id, component path)` guests — bare-name
    /// adapters reachable by dispatch.
    pub guests: &'a [(&'a str, &'a str)],
}

/// Drive `deployment` once over `backends`, returning the guest's exit
/// status.
///
/// # Errors
///
/// Returns an error if the deployment cannot be built or linked, the path
/// loader cannot open the project, or the guest traps without exiting.
pub async fn run(deployment: Deployment<'_>, backends: Backends) -> Result<ExitStatus> {
    let mut manifest = Manifest::new()
        .plugins([SOURCE_INTERFACE])
        .guest(GuestEntry::new("emery", deployment.engine))
        .mounts([deployment.project.mount(false)]);
    for (id, wasm) in deployment.guests {
        manifest = manifest.guest(GuestEntry::new(*id, *wasm));
    }
    // The runtime supplies argv[0]; these are the operator's arguments.
    let args: Vec<String> = deployment.argv.iter().map(|arg| (*arg).to_owned()).collect();

    let mut built = DeploymentBuilder::new()
        .manifest(manifest)
        .mode(Mode::Command)
        .args(args)
        .build::<StoreCtx<Backends>>()
        .await
        .context("building deployment")?;
    built.host::<WasiOtel, Backends>()?;
    built.host::<WasiModel, Backends>()?;
    built.host::<WasiKeyValue, Backends>()?;
    built.host::<WasiBlobstore, Backends>()?;
    built.host::<WasiPlugins, Backends>()?;

    let mounts = built.mounts();
    let args = built.args().to_vec();
    let registry = Arc::new(built.into_registry().context("assembling registry")?);
    let runtime = Runtime::from_parts(registry, args, mounts, backends);

    let path = PathMounts::new([(".", deployment.project.path())])
        .context("opening the project as the path-load location")?;
    Plugins::install(&runtime, None, Some(Arc::new(path)))?;
    serve_links(&runtime).await.context("wiring host-mediated dispatch")?;

    let status = runtime.run_command().await;
    runtime.shutdown();
    status
}

/// A per-test project directory, removed on drop — including when the test
/// panics partway through.
#[derive(Debug)]
pub struct Scratch(TempDir);

impl Scratch {
    /// The directory path.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.0.path()
    }

    /// A [`Mount`] preopening this directory into the guest sandbox as `.`.
    #[must_use]
    pub fn mount(&self, writable: bool) -> Mount {
        Mount {
            name: ".".to_owned(),
            path: self.path().to_path_buf(),
            writable,
        }
    }

    /// Writes `contents` at `relative`, creating parent directories.
    ///
    /// # Panics
    ///
    /// Panics if the file cannot be written.
    pub fn write(&self, relative: &str, contents: impl AsRef<[u8]>) {
        let target = self.path().join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("creating scratch subdirectory");
        }
        fs::write(&target, contents).expect("writing scratch file");
    }
}

/// Create a fresh [`Scratch`] directory.
///
/// # Panics
///
/// Panics if the directory cannot be created.
#[must_use]
pub fn scratch() -> Scratch {
    Scratch(tempfile::tempdir().expect("creating scratch dir"))
}
