//! Compile-time embedded schemas from the repository's `schemas/` directory.

use include_dir::{Dir, include_dir};

/// All schemas bundled at compile time from `schemas/`.
pub static EMBEDDED_SCHEMAS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/schemas");
