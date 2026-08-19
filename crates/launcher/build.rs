//! Generates the embedded first-party registry (ADR-0002 §2) from
//! `EMERY_EMBED_DIR` — a directory of built `<name>.wasm` components
//! staged by the release build (first-party adapters) and by the
//! journey rung (the mock component). Unset, missing, or empty, the
//! table is empty and resolution stays local (cache seed, store).

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=EMERY_EMBED_DIR");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let mut rows = String::new();
    for (name, path) in components() {
        #[expect(
            clippy::unnecessary_debug_formatting,
            reason = "Debug formatting emits the quoted, escaped path literal include_bytes! needs"
        )]
        writeln!(rows, "    (\"{name}\", include_bytes!({path:?}).as_slice()),")
            .expect("write to a String");
    }
    let table = format!(
        "/// First-party components embedded in the binary as default\n\
         /// registry entries (ADR-0002 \u{a7}2), staged from `EMERY_EMBED_DIR`\n\
         /// at build time.\n\
         const EMBEDDED: &[(&str, &[u8])] = &[\n{rows}];\n"
    );
    std::fs::write(out.join("embedded.rs"), table).expect("write the embedded registry table");
}

/// Every `<name>.wasm` under `EMERY_EMBED_DIR`, sorted by the adapter
/// name its file stem derives (the `emery_` artifact prefix stripped,
/// underscores folded to kebab dashes — mirroring
/// `engine::resolve::name_from_component`).
fn components() -> Vec<(String, PathBuf)> {
    let Some(dir) = std::env::var_os("EMERY_EMBED_DIR") else {
        return Vec::new();
    };
    let dir = PathBuf::from(dir);
    println!("cargo:rerun-if-changed={}", dir.display());
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut components: Vec<(String, PathBuf)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "wasm"))
        .filter_map(|path| {
            let stem = path.file_stem()?.to_str()?;
            let name = stem
                .strip_prefix("emery_")
                .or_else(|| stem.strip_prefix("emery-"))
                .unwrap_or(stem)
                .replace('_', "-");
            Some((name, path))
        })
        .collect();
    components.sort();
    components
}
