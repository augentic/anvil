//! Discovery surface — the `## Lead inventory` blocks in `discovery.md`.
//!
//! [`lead::validate_leads`] re-checks blocks before the merge into
//! `discovery.md`; the whole-document model lives in [`document`].

pub mod document;
pub mod lead;

pub use document::{Discovery, ResolveError as DiscoveryResolveError};
pub use lead::{Lead, validate_leads};
