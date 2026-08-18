//! The mock source adapter as a Wasm component (remediation Phase 2):
//! the journey-test fixture crossing the production seam
//! (target-architecture §8; ADR-0002, T1, CC-17).

// One export serves every mock identity: dispatch routes by the call's
// `adapter-id`, and `mock::behaviour` keys its deterministic profile
// (docs / code / fail-extract / minimal) off that routed id.
#[cfg(target_arch = "wasm32")]
mod guest {
    adapter::source!(mock::Adapter);
}
