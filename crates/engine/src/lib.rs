//! Emery's specification-generation engine.

pub mod cli;
mod extract;
mod handler;
pub mod home;
mod resolve;
mod show;
mod sources;
mod spec;
mod specify;
mod storage;
mod synthesise;

// Generated from the link-checked synthesis corpus at build time.
mod prose {
    emery_adapter::registry!();
}
