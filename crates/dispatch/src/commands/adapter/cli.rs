//! Clap derive surface for `specify adapter *` (RFC-48 D6/D10). The
//! umbrella `cli.rs` re-exports `AdapterAction`.

use std::path::PathBuf;

use clap::Subcommand;

/// Verbs under `specify adapter`.
#[derive(Debug, Subcommand)]
pub enum AdapterAction {
    /// Build a self-contained adapter artifact: dereference the in-repo
    /// `adapters/shared/` symlinks into real bytes, exclude the Rust
    /// source trees and the dev/VCS cruft, and pack the tree into a
    /// byte-deterministic layer (RFC-48 D1/D9/D12). Reports the layer's
    /// content digest and entry count.
    Build {
        /// Adapter directory holding `adapter.yaml` (defaults to the
        /// current directory).
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Pack the adapter, publish it as an immutable, content-addressed
    /// single-layer OCI artifact under `reference`, pull it back, and
    /// verify the recorded digest (RFC-48 D4/D6). Refuses to re-publish
    /// an existing `(name, version)` with different bytes.
    Publish {
        /// Adapter directory holding `adapter.yaml` (defaults to the
        /// current directory).
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Immutable OCI reference `<registry>/<repo>:<version>` to
        /// publish under (e.g. `ghcr.io/augentic/omnia:1.2.0`).
        #[arg(long)]
        reference: String,
    },
}
