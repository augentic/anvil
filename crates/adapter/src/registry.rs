//! Embedded prose lookup.
//!
//! [`crate::registry!`] includes the sorted `DOCS` table emitted by `build.rs`.

/// An embedded reference document.
#[derive(Clone, Copy, Debug)]
pub struct Doc {
    /// Adapter-relative path.
    pub path: &'static str,
    /// Markdown body.
    pub body: &'static str,
}

/// Finds `path`; `docs` must be sorted by path.
#[must_use]
pub fn find<'d>(docs: &'d [Doc], path: &str) -> Option<&'d Doc> {
    docs.binary_search_by(|doc| doc.path.cmp(path)).ok().map(|idx| &docs[idx])
}

/// Returns the body for `path`.
#[must_use]
pub fn resolve(docs: &[Doc], path: &str) -> Option<&'static str> {
    find(docs, path).map(|doc| doc.body)
}

/// Returns the body for an embedded `path`.
///
/// # Panics
///
/// Panics if `path` is absent, indicating a registry/tree mismatch.
#[must_use]
pub fn body(docs: &[Doc], path: &str) -> &'static str {
    resolve(docs, path)
        .unwrap_or_else(|| panic!("document `{path}` is not embedded in the registry"))
}

/// Generates registry accessors for the build-time `DOCS` table.
///
/// ```ignore
/// mod registry {
///     emery_adapter::registry!();
/// }
/// ```
#[macro_export]
macro_rules! registry {
    () => {
        pub use $crate::registry::Doc;

        include!(concat!(env!("OUT_DIR"), "/prose_docs.rs"));

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
