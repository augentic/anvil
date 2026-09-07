//! Emery's specification-generation engine.

mod extract;
pub mod preopen;
mod resolve;
pub mod show;
pub mod sources;
mod spec;
pub mod specify;
mod store;
mod synthesise;

use emery_source::Source;
use omnia_guest::{BlobStore, Model, Plugins, StateStore};
pub use resolve::AdapterSelector;

/// The capability set every operation can be dispatched over, as one
/// bound for the transports that bind a provider.
pub trait Provider:
    Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static
{
}

impl<P: Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static> Provider for P {}

// Generated from the link-checked synthesis corpus at build time.
mod prose {
    emery_prose::registry!();
}
