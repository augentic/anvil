//! Kernel-level operations on a Emery slice directory, driven by the
//! refine / build / merge orchestrations and the drop / prune verbs.
//!
//! Create, transition, touched-spec classification, archive, and
//! discard.

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
