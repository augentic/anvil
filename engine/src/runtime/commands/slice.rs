//! Native-only `specify slice` residue: the `slice build` handler,
//! which owns the manifest-driven prepare-hook dispatch
//! (`extension::run_captured`) and the two-phase agent envelope. Every
//! other slice verb is shared and lives in
//! `specify_dispatch::commands::slice`.

pub mod build;
