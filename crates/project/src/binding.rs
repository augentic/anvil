//! Locator grammar, bounded-read policy, and CID ingestion kernel.
//!
//! Git clone and HTTPS fetch are host-side (`launcher::ingest`); this
//! module is wasm-clean.

mod https;
mod ingest;
mod locator;
mod meter;
mod policy;

#[cfg(not(target_arch = "wasm32"))]
mod git;
#[cfg(not(target_arch = "wasm32"))]
mod host;
#[cfg(not(target_arch = "wasm32"))]
mod https_fetch;

#[cfg(not(target_arch = "wasm32"))]
pub use git::checkout;
#[cfg(not(target_arch = "wasm32"))]
pub use host::{fetch as fetch_locator, resolve};
pub use https::{check as check_https, is_private, raw_github};
#[cfg(not(target_arch = "wasm32"))]
pub use https_fetch::fetch as fetch_https;
pub use ingest::{Cache, Resolved, Session, Staged, roots, view};
pub use locator::{Location, Locator, Origin};
pub use meter::Meter;
pub use policy::Policy;
