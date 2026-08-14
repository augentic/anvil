//! Compiled D9 read-policy starting values. Not a planning input.

/// Bounded-read policy for delivery binding (RFC-88 D9).
///
/// Declared starting values, not calibrated measurements. Production
/// always uses [`Self::standard`]; there is no host or project override.
/// Exhaustion fails the wave for upstream narrowing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Policy {
    /// Independent bind-read fan-out. Focused-survey fan-out is RFC-96.
    pub concurrency: usize,
    /// Maximum locator rows in one wave bind.
    pub bindings: usize,
    /// Git ls-remote / fetch and HTTPS requests across the wave.
    pub api_requests: usize,
    /// Wall-clock budget for the whole wave bind, in milliseconds.
    pub time_ms: u64,
    /// Cumulative bytes inspected (tree walks + HTTPS bodies).
    pub inspected_bytes: u64,
    /// Trees stored under a CID (a file is a one-file tree).
    pub imported_trees: usize,
    /// HTTPS 3xx hops followed, including the final non-redirect.
    pub https_redirects: usize,
    /// Maximum HTTPS response body, in bytes.
    pub https_body: usize,
}

impl Policy {
    /// Compiled starting values (closed Open Question 10).
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            concurrency: 4,
            bindings: 32,
            api_requests: 128,
            time_ms: 120_000,
            inspected_bytes: 512 * 1024 * 1024,
            imported_trees: 32,
            https_redirects: 5,
            https_body: 32 * 1024 * 1024,
        }
    }
}

impl Default for Policy {
    fn default() -> Self {
        Self::standard()
    }
}
