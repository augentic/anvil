//! Emery's specification-generation engine.

pub mod extract;
pub mod handler;
pub mod home;
pub mod resolve;
pub mod show;
pub mod sources;
pub mod specify;
mod storage;
pub mod synthesise;

// Generated from the link-checked synthesis corpus at build time.
mod prose {
    emery_adapter::registry!();
}
