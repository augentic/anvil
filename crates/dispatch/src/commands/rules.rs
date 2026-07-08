//! `specify rules *` — codex resolution surface.
//!
//! `export` is the read-only `ResolvedRules` export contract entry
//! point; it is pure filesystem over the codex trees, so the handler
//! runs on both sides of the seam (native and in-guest, guest routing).
//! Codex-pack materialization lives in `adapters sync`.

pub mod cli;
pub mod export;
