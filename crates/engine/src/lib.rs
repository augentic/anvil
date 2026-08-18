//! The spec-generator engine (remediation Phase 3): the one loop —
//! extract → synthesise → emit `spec.md` / `design.md` — behind the
//! generation-pointer output home (ADR-0001 Option C, ADR-0009).

pub mod extract;
pub mod handler;
pub mod home;
pub mod init;
pub mod project;
pub mod resolve;
pub mod specify;
pub mod synthesise;

/// The embedded synthesis-prose registry (the reviewed v1 port under
/// `prose/`, walked and link-checked at build time).
mod prose {
    adapter::registry!();
}
