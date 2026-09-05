//! The `emery:adapter/source` contract, shared by the engine and the
//! adapter SDK.
//!
//! Engine providers implement the import-side [`Source`] capability;
//! adapters reach the export side through `emery-adapter`.

pub mod claims;
mod dispatch;
mod identity;
pub mod types;

#[cfg(target_arch = "wasm32")]
pub mod wire;

/// The versioned `source` interface a deployment declares as its plugin
/// seam; must track the `emery:adapter` WIT package version.
pub const SOURCE_INTERFACE: &str = "emery:adapter/source@0.1.0";

pub use dispatch::{DispatchError, Source};
pub use identity::{AdapterIdentity, IdentityError};
