//! Registry topology, shape validation, and workspace materialisation
//! for Specify.

pub mod branch;
pub mod catalog;
pub mod gitignore;
pub mod identity;
pub mod topology;
pub mod validate;
pub mod workspace;

pub use catalog::{ContractRoles, GreenfieldSeed, Registry, RegistryProject};
pub use gitignore::ensure_gitignore_entries;
pub use topology::{Surface, TopologyLock, TopologyProject, cache_staleness};
