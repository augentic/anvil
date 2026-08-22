//! The spec-generator engine: extract → synthesise → emit `spec.md` /
//! `design.md` behind the generation-pointer output home.

pub mod extract;
pub mod handler;
pub mod home;
pub mod init;
pub mod project;
pub mod resolve;
pub mod specify;
mod storage;
pub mod synthesise;

// The embedded synthesis-prose registry under `prose/`, walked and
// link-checked at build time.
mod prose {
    emery_adapter::registry!();
}
