//! Embeds every markdown document under the adapter's `prose/` tree as
//! the sorted `DOCS` table `src/registry.rs` includes; symlinks resolve
//! at build time.

fn main() {
    emery_prose::emit("prose");
}
