//! `specify rules {export, sync}` — codex resolution surface.
//!
//! `export` is the read-only `ResolvedRules` export contract CLI entry
//! point. It does not require an initialised `.specify/` tree:
//! callers supply `--rules-root` directly (or rely on the resolver's
//! probe against a monorepo or the distributed codex cache). No journal
//! events, no lifecycle transitions, no on-disk writes — the handler
//! streams a `ResolvedRules` JSON envelope to stdout and exits.
//!
//! `sync` distributes the shared codex into the out-of-tree `<project-cache>/codex/`
//! pinned to the project's adapter source/ref (RM-07), so consumer
//! projects resolve shared `UNI-*` rules without a co-located framework
//! checkout or a manual `--rules-root`.

pub mod cli;
pub mod export;
pub mod sync;
