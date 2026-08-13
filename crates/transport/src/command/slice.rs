//! Clap argument types for `emery slice *`. Each `*Args` type mirrors
//! its command's workflow wire input.

/// Arguments for `slice list` — none.
#[derive(Clone, Copy, Debug, clap::Args)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "clap's `Args` derive requires a braced struct"
)]
pub struct ListArgs {}

/// Arguments for `slice validate`.
#[derive(Debug, clap::Args)]
pub struct ValidateArgs {
    /// Slice name (under `.emery/change/slices/`)
    pub name: String,
}

/// Arguments for `slice provenance`.
#[derive(Debug, clap::Args)]
pub struct ProvenanceArgs {
    /// Slice name (under `.emery/change/slices/`)
    pub name: String,
}

/// Arguments for `slice model show`.
#[derive(Debug, clap::Args)]
pub struct ModelShowArgs {
    /// Slice name (under `.emery/change/slices/`)
    pub name: String,
}
