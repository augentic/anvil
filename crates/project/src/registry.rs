//! Registry topology and shape validation for Specify.

pub(crate) mod catalog;
mod gitignore;
pub mod handlers;
pub(crate) mod identity;
pub mod topology;
mod validate;
pub(crate) mod workspace;

pub use catalog::{ContractRoles, GreenfieldSeed, Registry, RegistryProject};
pub(crate) use gitignore::ensure_gitignore;
pub(crate) use topology::cache_staleness;
