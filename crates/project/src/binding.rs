//! Locator grammar, bounded-read policy, and CID ingestion kernel.
//!
//! Git clone and HTTPS fetch are host-side (`launcher::ingest`); this
//! module is wasm-clean.

mod https;
mod ingest;
mod locator;
mod meter;
mod policy;

pub use https::{check as check_https, is_private, raw_github};
pub use ingest::{Cache, Resolved, Session, Staged, roots, view};
pub use locator::{Location, Locator, Origin};
pub use meter::Meter;
pub use policy::Policy;
