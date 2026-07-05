//! Native-only `specify plan` residue: the subprocess-spawning
//! `plan lock` handler. Every other plan verb is shared and lives in
//! `specify_dispatch::commands::plan`.

pub mod lock;
