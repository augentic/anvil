//! `specify rules *` — codex resolution surface.
//!
//! `export` is the read-only `ResolvedRules` export contract entry
//! point; it is pure filesystem over the codex trees, so the handler
//! runs on both sides of the seam (native and in-guest, RFC-65
//! move 1). `sync` materializes the embedded codex packs into the
//! per-project cache; its handler stays in the binary crate until the
//! `adapters sync` integration retires the verb (RFC-66) — only its
//! grammar lives here.

pub mod cli;
pub mod export;
