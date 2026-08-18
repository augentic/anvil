//! Embeds every markdown document under `prose/` (the reviewed v1
//! synthesis-prompt port) as the sorted `DOCS` table the engine's
//! registry module includes; relative links are checked at build time.

fn main() {
    prose::emit("prose");
}
