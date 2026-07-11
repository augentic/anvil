//! Verb-level operations on a Specify slice directory.
//!
//! Create, transition, scan, overlap detection, archive, and discard.

pub mod archive;
pub mod create;
pub mod discard;
pub mod io;
pub mod overlap;
pub mod prune;
pub mod scan;
pub mod transition;

pub use archive::archive;
pub use create::{CreateIfExists, Created, create, validate_name};
pub use discard::discard;
pub use io::move_atomic;
pub use overlap::{Overlap, overlap};
pub use prune::Retention;
pub use scan::{scan_touched, touched_from_rendered, write_touched};
pub use transition::transition;
