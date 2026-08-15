//! Locator grammar, bounded-read policy, and CID ingestion kernel.
//! Wasm-clean: origin I/O runs through the seam's `Trees`, CID
//! minting through `Workspaces` ([`crate::vcs`] backs the native leg).

mod fetch;
mod https;
mod ingest;
mod locator;
mod meter;
mod policy;

#[cfg(not(target_arch = "wasm32"))]
mod git;
#[cfg(not(target_arch = "wasm32"))]
mod https_fetch;

pub use fetch::{fetch as fetch_locator, resolve};
#[cfg(not(target_arch = "wasm32"))]
pub use git::checkout;
pub use https::{check as check_https, is_private, raw_github};
#[cfg(not(target_arch = "wasm32"))]
pub use https_fetch::fetch as fetch_https;
pub use ingest::{Cache, Resolved, Session, Staged, roots, view};
pub use locator::{Location, Locator, Origin};
pub use meter::Meter;
pub use policy::Policy;
