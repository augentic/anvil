# Crate Writer — Rules

Read this when authoring or updating a generated crate. The Hard Rules and Authority Hierarchy below are normative — every generation pass must satisfy them. SKILL.md and the artifacts cannot weaken anything stated here.

## Authority Hierarchy

When conflicts arise, follow this strict precedence:

1. **SKILL.md** (highest) -- generation/update rules and hard constraints
2. **Specify artifacts (specs + design.md)** -- behavior specification (artifacts always win for changed behavior)
3. **references/** -- authoritative patterns and SDK API
4. **examples/** -- canonical production code patterns
5. **Existing crate code** (UPDATE MODE ONLY) -- authoritative for unchanged behavior; trust existing code for anything the updated artifacts do not contradict
6. **Original source** (if provided) -- reference for ambiguity only
7. **LLM inference** (lowest) -- prohibited for `[unknown]` cases; use TODO markers

**Key difference between modes**: In update mode, existing crate code sits at level 5 -- authoritative for any behavior the artifacts do not explicitly change. In create mode, there is no existing code; levels 5-6 are skipped.

## Hard Rules

Violations of any rule below fail generation or update.

### Core Rules (both modes)

1. **Omnia SDK only** -- all errors return `omnia_sdk::Error`; no custom error types in public API
2. **Provider-only I/O** -- all external I/O through provider traits; no direct network/file/env access
3. **No forbidden crates** -- see [guardrails.md](guardrails.md)
4. **No mutable global state** -- no `static mut`, `OnceCell`, `lazy_static!`; `LazyLock` allowed only for immutable compile-time lookup tables
5. **Handler trait required** -- request structs implement `Handler<P>` with delegation pattern; no custom handler structs with `new()` / `process_message()`
6. **Strong typing** -- newtypes for IDs; enums for known value sets; no raw primitives for domain concepts
7. **WASM compatible** -- no `std::env`, `std::fs` (use `StateStore` / `Blobstore` / `DocumentStore` / `HttpRequest`), `std::net`; `std::thread::sleep` only under `#[cfg(not(debug_assertions))]`
8. **All operations async** -- no blocking I/O
9. **Correct adapter trait for data stores** -- SQL databases (PostgreSQL, MySQL, SQL Server) use `TableStore`; Azure Table Storage, Cosmos DB document API, and MongoDB use `DocumentStore`; Azure Blob Storage and AWS S3 use `Blobstore`; never `HttpRequest` for any managed data store. If the artifacts say "use HttpRequest" for a managed data store, override the artifacts (build brief > artifacts per authority hierarchy). See [anti-patterns.md](examples/crates/anti-patterns.md) #10.

### Update-Specific Rules (update mode only)

10. **No regressions** -- the build orchestration layer captures a test baseline before changes and runs a verify-repair loop after both crate-writer and test-writer complete; crate-writer must not introduce changes that break previously-passing tests
11. **Artifacts win for changed behavior** -- when the updated artifacts contradict existing code, trust the artifacts; the old behavior is intentionally being replaced
12. **Preserve unchanged code** -- do not reformat, restructure, or modify code regions that the slice set does not touch
13. **No silent removals** -- every subtractive change must be documented in CHANGELOG.md with the reason (artifacts no longer specify this behavior)
14. **Testable exports** -- every modified or added handler must be exported so test-writer can generate tests; subtractive changes must be reflected in the public API so test-writer can remove stale tests
15. **Atomic categories** -- complete all changes within a category before moving to the next; do not interleave
16. **Structural changes require re-inventory** -- after applying structural changes, re-scan the crate before proceeding to subsequent categories

---

Return to [build/crate.md](../briefs/build/crate.md) for the critical path, mode-dispatch table, and artifact mapping.
