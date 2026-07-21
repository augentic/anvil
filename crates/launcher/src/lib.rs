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
//! Two argv shapes never assemble a deployment: help and version
//! displays render host-side from the shared grammar (byte-identical
//! to the guest's), and `adapter add` — a deterministic cache copy
//! whose input may live outside any guest mount — seeds and reports
//! host-side.
//!
//! `SPECIFY_HOME` remains a relocation override only — the Cargo
//! model: everything anchors at the user home or the project root by
//! default, and one invocation captures the layout exactly once
//! ([`prepare`] → `Locations::from_env`).

use std::future::Future;
use std::path::Path;

use error::Error;
use project::handler::{ExecutionPaths, Locations};
use transport::command::Format;
use transport::command::selectors::{CommandSelectors, Projection, SeedRequest, from_argv};

mod anchor;
mod closure;
mod deployment;
mod hydrate;

pub use deployment::{Deployment, Guest, Mount};

/// The engine identity the composition root supplies.
///
/// The engine guest is versioned by the binary, so the binary's own
/// `CARGO_PKG_VERSION` is the pin — injected here rather than read
/// from the launcher crate's, which is a different package. The
/// launcher hydrates `specify:engine@<version>` from the registry on
/// a store miss (the same path as every other package pin).
#[derive(Debug, Clone, Copy)]
pub struct Engine {
    /// Exact semver of the engine guest — the binary's own version.
    pub version: &'static str,
}

/// What the binary should do with one argv.
#[derive(Debug)]
pub enum Outcome {
    /// The closure resolved and verified: start the runtime over this
    /// deployment.
    Run(Deployment),
    /// The invocation completed (or failed) host-side, no runtime
    /// needed: write both channels and exit with `code`. Help and
    /// version displays and the `adapter add` report land on `stdout`;
    /// grammar rejections render clap's own diagnostic and failures
    /// render the standard envelope in the invocation's `--format`,
    /// both on `stderr`.
    Done {
        /// Rendered success output for the stdout channel.
        stdout: Vec<u8>,
        /// Rendered diagnostic for the stderr channel.
        stderr: Vec<u8>,
        /// Process exit code.
        code: u8,
    },
}

impl Outcome {
    fn failure(format: Format, error: &Error) -> Self {
        let (stderr, code) = transport::command::render_failure(format, error);
        Self::Done {
            stdout: Vec::new(),
            stderr,
            code,
        }
    }
}

/// Prepare the deployment for one invocation.
///
/// Captures the artifact layout (`Locations::from_env`) exactly once,
/// projects selectors from argv, derives the closure, hydrates store
/// misses from the registry over HTTPS, verifies, and assembles. Help
/// and version displays and the deterministic `adapter add` seed
/// complete host-side without a deployment.
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
        Projection::Display { rendered } => {
            return Outcome::Done {
                stdout: rendered.into_bytes(),
                stderr: Vec::new(),
                code: 0,
            };
        }
        Projection::Rejected { rendered } => {
            return Outcome::Done {
                stdout: Vec::new(),
                stderr: rendered.into_bytes(),
                code: 2,
            };
        }
        Projection::Forward(selectors) => selectors,
    };
    let format = selectors.format;
    let root = anchor::project_root(invoked_dir, &selectors);
    let paths = ExecutionPaths::new(root, locations);
    if let Some(request) = &selectors.seed {
        return seed(request, &paths, format);
    }
    match assemble(&paths, &selectors, engine, fetch) {
        Ok(deployment) => Outcome::Run(deployment),
        Err(error) => Outcome::failure(format, &error),
    }
}

fn assemble<F, Fut>(
    paths: &ExecutionPaths, selectors: &CommandSelectors, engine: Engine, fetch: F,
) -> Result<Deployment, Error>
where
    F: Fn(String) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Vec<u8>, Error>> + Send,
{
    let closure = closure::compute(paths.project_root(), selectors, engine);
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let resolved = runtime.block_on(hydrate::resolve(paths, closure, fetch))?;
    deployment::assemble(paths, resolved)
}

/// Run `adapter add` host-side, end to end: the operator's component
/// path may live anywhere on the host, while the engine guest sees
/// only its mounts — and the copy is deterministic engine-free work,
/// so no runtime is started. Renders the same success envelope the
/// guest verb produces on a native host.
fn seed(request: &SeedRequest, paths: &ExecutionPaths, format: Format) -> Outcome {
    // Relative component paths anchor at the selected project
    // directory — the kernel's own contract.
    match project::adapter::ensure::seed(&request.component, paths, jiff::Timestamp::now()) {
        Ok(seeded) => {
            let body = project::adapter::handlers::AddBody::from(seeded);
            match transport::command::render_success(format, &body) {
                Ok(stdout) => Outcome::Done {
                    stdout,
                    stderr: Vec::new(),
                    code: 0,
                },
                Err(error) => Outcome::failure(format, &error),
            }
        }
        Err(error) => Outcome::failure(format, &error),
    }
}
