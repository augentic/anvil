//! Embed-time markdown corpus: every document under `prompts/` is
//! link-checked and embedded into the generated `DOCS` table (see the
//! `prose` build crate) — if it is in `prompts/`, it ships.

fn main() {
    prose::emit("prompts");
}
