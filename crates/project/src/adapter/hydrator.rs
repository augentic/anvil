//! Deployment capability for fetching pinned adapter components.
//!
//! Hydration is the provisioning half of the adapter story: a pinned
//! package reference (`specify:omnia@1.0.0`) that misses the global
//! single-file store is fetched from the configured registry and
//! installed (`specify init` / `specify init --upgrade` drive it).
//! Only the byte transport is deployment-specific — the shipped WASI
//! provider fetches over `wasi:http`; harness and test providers
//! script or refuse — so only the fetch crosses this seam. URL
//! derivation, the store write, the digest sidecar, and
//! verify-after-write are deterministic and live in
//! `project::init::hydrate`.

use std::future::Future;

use error::Error;

/// Provider capability that fetches component bytes from a registry
/// URL.
pub trait Hydrator: Send + Sync {
    /// Fetch the component bytes at `url` (an absolute URL derived
    /// from the project's registry configuration).
    ///
    /// # Errors
    ///
    /// A transport or non-success-status failure. Callers wrap it in
    /// the typed `adapter-hydrate-failed` diagnostic.
    fn fetch(&self, url: &str) -> impl Future<Output = Result<Vec<u8>, Error>> + Send;
}
