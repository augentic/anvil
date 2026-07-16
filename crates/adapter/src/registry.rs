//! Embedded prose vocabulary and lookup helpers.
//!
//! Each adapter's `build.rs` emits a sorted `DOCS` table; the adapter's
//! `registry` module includes it via [`registry!`].

/// One embedded reference document.
#[derive(Clone, Copy, Debug)]
pub struct Doc {
    /// Adapter-relative path, e.g. `prompts/build.md`.
    pub path: &'static str,
    /// Full markdown body.
    pub body: &'static str,
}

/// Binary-search lookup; `docs` must be sorted by path.
#[must_use]
pub fn find<'d>(docs: &'d [Doc], path: &str) -> Option<&'d Doc> {
    docs.binary_search_by(|doc| doc.path.cmp(path)).ok().map(|idx| &docs[idx])
}

/// Body for `path`, or `None` when absent from `docs`.
///
/// With `SPECIFY_PROSE_OVERLAY=1`, an on-disk `.eval/prose/<path>` body
/// wins; the doc *set* stays the embedded table's.
#[must_use]
pub fn resolve(docs: &[Doc], path: &str) -> Option<&'static str> {
    let doc = find(docs, path)?;
    if overlay_enabled()
        && let Some(body) = overlay(path)
    {
        return Some(body);
    }
    Some(doc.body)
}

fn overlay_enabled() -> bool {
    std::env::var("SPECIFY_PROSE_OVERLAY").is_ok_and(|value| value == "1")
}

// Overlay bodies are leaked to keep `&'static str`; fine for the
// per-call-instantiated guest. First hit attests on stderr so an overlay
// run cannot pass as embedded.
fn overlay(path: &str) -> Option<&'static str> {
    static ATTEST: std::sync::Once = std::sync::Once::new();
    let file = std::path::Path::new(".eval/prose").join(path);
    match std::fs::read_to_string(&file) {
        Ok(body) => {
            ATTEST.call_once(|| {
                eprintln!("prose overlay active: .eval/prose/ overrides embedded bodies");
            });
            Some(body.leak())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => panic!("prose overlay `{}` is unreadable: {err}", file.display()),
    }
}

/// Body the registry is guaranteed to embed.
///
/// # Panics
///
/// When `path` is missing — adapter tree and embedded table disagree.
#[must_use]
pub fn body(docs: &[Doc], path: &str) -> &'static str {
    resolve(docs, path)
        .unwrap_or_else(|| panic!("document `{path}` is not embedded in the registry"))
}

/// Generate an adapter's `registry` module over the `DOCS` table from `build.rs`.
///
/// ```ignore
/// mod registry {
///     adapter::registry!();
/// }
/// ```
#[macro_export]
macro_rules! registry {
    () => {
        pub use $crate::registry::Doc;

        include!(concat!(env!("OUT_DIR"), "/registry_docs.rs"));

        /// Every embedded document, sorted by adapter-relative path.
        #[must_use]
        pub fn docs() -> &'static [Doc] {
            DOCS
        }

        /// Look up one document by its adapter-relative path.
        #[must_use]
        pub fn doc(path: &str) -> Option<&'static Doc> {
            $crate::registry::find(DOCS, path)
        }

        /// Body the registry is guaranteed to embed.
        ///
        /// # Panics
        ///
        /// When `path` is not embedded.
        #[must_use]
        pub fn body(path: &str) -> &'static str {
            $crate::registry::body(DOCS, path)
        }
    };
}
