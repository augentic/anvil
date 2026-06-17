//! Verb-level operations on a Specify slice directory.
//!
//! Create, transition, scan, overlap detection, archive, discard, and
//! outcome stamping — each verb is a free function so the CLI dispatches one
//! per import.

pub mod archive;
pub mod create;
pub mod discard;
pub mod io;
pub mod outcome;
pub mod overlap;
pub mod prune;
pub mod scan;
pub mod transition;

pub use archive::archive;
pub use create::{CreateIfExists, Created, create, validate_name};
pub use discard::discard;
pub use io::move_atomic;
pub use outcome::stamp_outcome;
pub use overlap::{Overlap, overlap};
pub use prune::{ArchivedSlice, Retention};
pub use scan::{scan_touched, write_touched};
pub use transition::transition;
