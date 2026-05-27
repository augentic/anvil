//! Seed fixture for RFC-32 Phase 2 deterministic-hint verification.
//!
//! Intentionally trips two seeded hints:
//!   * UNI-014 (regex on Rust source) via the literal URL below.
//!   * OMNIA-002 (regex on Rust source) via the `std::env` reference.

pub const SERVICE_URL: &str = "https://example.com/api/v1/things";

pub fn load_env() -> Option<String> {
    std::env::var("RFC_32_SEED").ok()
}
