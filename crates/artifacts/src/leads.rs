//! Authoritative `leads.md` catalog — one block per `(source, lead)`.
//!
//! [`lead::validate_leads`] re-checks a survey set before merge;
//! the document model and canonical digest live in [`document`].

pub mod document;
pub mod lead;

pub use document::{Leads, ResolveError as LeadsResolveError};
pub use lead::{Lead, validate_leads};
