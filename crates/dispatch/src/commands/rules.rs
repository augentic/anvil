//! `specify rules *` — codex resolution surface.
//!
//! `export` is the read-only `ResolvedRules` export contract entry
//! point; it is pure filesystem over the codex trees, so the handler
//! runs on both sides of the seam (native and in-guest, RFC-65
//! move 1). The old `rules sync` verb retired into `adapters sync`,
//! which re-materializes the embedded codex packs on every run.

pub mod cli;
pub mod export;
