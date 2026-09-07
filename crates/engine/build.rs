//! Engine build script
//!
//! Embeds the synthesis prompt corpus under `prose/` into the engine at
//! compile time and checks its internal links, so a prompt that references a
//! missing document is a build failure rather than a run-time surprise.

fn main() {
    emery_prose::emit("prose");
}
