//! The self-assembling deployment launcher inside the shipped
//! `specify` binary (RFC-70 Stage 1).
//!
//! For each invocation the launcher derives the component closure —
//! the engine guest plus every adapter the command could dispatch —
//! from the anchored project's `project.yaml`, the active `plan.yaml`,
//! and each materialised workspace slot's own `project.yaml` (each leg
//! joined only when the routed verb's `ClosureScope` can reach it, so
//! read-only verbs deploy the engine alone), plus the adapter
//! selectors argv itself
//! carries (projected through the shared transport grammar). Closure
//! misses are hydrated into the well-known locations (the global
//! adapter store for package pins, the per-project cache for local
//! components — both derived from one carried `Locations` value),
//! every store-resolved entry is digest-verified fail closed as it
//! resolves (one digest pass per component, inside hydration), and
//! the result is a typed [`Deployment`] the binary maps onto an Omnia
//! manifest and runs. Nothing starts when verification fails.
//!
//! `SPECIFY_HOME` remains a relocation override only — the Cargo
//! model: everything anchors at the user home or the project root by
//! default, and one invocation captures the layout exactly once
//! ([`prepare`] → `Locations::from_env`).

use std::future::Future;
use std::path::Path;

use error::Error;
use project::handler::{ExecutionPaths, Locations};
use transport::command::selectors::{CommandSelectors, Projection, from_argv};

mod anchor;
mod closure;
mod deployment;
mod hydrate;

pub use deployment::{Deployment, Guest, Mount};

/// The engine identity the composition root supplies.
///
/// The engine guest is versioned by the binary, so the binary's own
/// `CARGO_PKG_VERSION` is the pin — injected here rather than read
/// from the launcher crate's, which is a different package. A release
/// build additionally carries the engine component itself
/// (`SPECIFY_ENGINE_WASM` at build time): the embedded bytes seed the
/// store entry on first launch, removing the first-launch network
/// dependency; `None` falls back to registry hydration.
#[derive(Debug, Clone, Copy)]
pub struct Engine {
    /// Exact semver of the engine guest — the binary's own version.
    pub version: &'static str,
    /// The embedded engine component, when the build carried one.
    pub bytes: Option<&'static [u8]>,
}

/// What the binary should do with one argv.
#[derive(Debug)]
pub enum Outcome {
    /// The closure resolved and verified: start the runtime over this
    /// deployment.
    Run(Deployment),
    /// Nothing was started: write `stderr` and exit with `code`
    /// (grammar rejections render clap's own diagnostic; closure,
    /// hydration, and verification failures render the standard
    /// failure envelope in the invocation's `--format`).
    Exit {
        /// Rendered diagnostic for the stderr channel.
        stderr: Vec<u8>,
        /// Process exit code.
        code: u8,
    },
}

/// Prepare the deployment for one invocation.
///
/// Captures the artifact layout (`Locations::from_env`) exactly once,
/// projects selectors from argv, derives the closure, hydrates misses
/// (the embedded engine seeds the store; registry over HTTPS for the
/// rest), verifies, and assembles.
#[must_use]
pub fn prepare(invoked_dir: &Path, argv: &[String], engine: Engine) -> Outcome {
    prepare_with(invoked_dir, argv, engine, Locations::from_env(), hydrate::http_fetch)
}

/// [`prepare`] with caller-supplied artifact locations and registry
/// byte transport.
///
/// The same explicit-layout and fetch-closure seams the ensure kernels
/// use, so deployments and tests bind their own layout and transport
/// without touching the environment.
pub fn prepare_with<F, Fut>(
    invoked_dir: &Path, argv: &[String], engine: Engine, locations: Locations, fetch: F,
) -> Outcome
where
    F: Fn(String) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Vec<u8>, Error>> + Send,
{
    let selectors = match from_argv(argv) {
        Projection::Rejected { rendered } => {
            return Outcome::Exit {
                stderr: rendered.into_bytes(),
                code: 2,
            };
        }
        Projection::Forward(selectors) => selectors,
    };
    let format = selectors.format;
    match assemble(invoked_dir, &selectors, engine, locations, fetch) {
        Ok(deployment) => Outcome::Run(deployment),
        Err(error) => {
            let (stderr, code) = transport::command::render_failure(format, &error);
            Outcome::Exit { stderr, code }
        }
    }
}

fn assemble<F, Fut>(
    invoked_dir: &Path, selectors: &CommandSelectors, engine: Engine, locations: Locations,
    fetch: F,
) -> Result<Deployment, Error>
where
    F: Fn(String) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Vec<u8>, Error>> + Send,
{
    let root = anchor::project_root(invoked_dir, selectors);
    let paths = ExecutionPaths::new(root, locations);
    seed(selectors, &paths)?;
    let closure = closure::compute(paths.project_root(), selectors, engine);
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let resolved = runtime.block_on(hydrate::resolve(&paths, closure, fetch))?;
    deployment::assemble(&paths, resolved)
}

/// Perform an `adapter add` cache seed host-side, before the runtime
/// starts: the operator's component path may live anywhere on the
/// host, while the engine guest sees only its mounts. The guest verb
/// then reports over the already-seeded entry.
fn seed(selectors: &CommandSelectors, paths: &ExecutionPaths) -> Result<(), Error> {
    let Some(request) = &selectors.seed else {
        return Ok(());
    };
    // Relative component paths anchor at the selected project
    // directory — the kernel's own contract.
    project::adapter::ensure::seed(&request.component, paths, jiff::Timestamp::now()).map(drop)
}
