//! The source adapter contract
//!
//! The agreement between the Emery engine and every source adapter: the
//! `emery:adapter/source` WIT world, the Rust types that mirror its records,
//! the rules a claim must satisfy, and the [`Source`] capability the engine
//! calls adapters through.
//!
//! Both sides depend on this one crate so they cannot drift apart. The engine
//! consumes it directly; adapters receive it re-exported through the
//! `emery-adapter` SDK.

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
