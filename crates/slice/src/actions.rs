//! Kernel-level operations on an Emery slice directory: create,
//! transition, touched-spec classification, archive, discard, and
//! prune.

pub mod archive;
pub mod create;
pub mod discard;
pub mod prune;
pub mod scan;
pub mod transition;

pub use archive::archive;
pub use create::{CreateIfExists, create};
pub use discard::discard;
pub use prune::Retention;
pub use scan::touched_from_rendered;
pub use transition::transition;
