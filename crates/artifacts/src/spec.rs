//! Spec-format parsing: [`ast`] is the one fail-closed load gate
//! (A17). The lenient v1 helpers were deleted at the Phase 3 spine
//! cut with their `crates/project` consumers (archived at tag `v1`).

pub mod ast;
