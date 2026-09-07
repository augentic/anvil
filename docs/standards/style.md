# Style

Cross-cutting code-quality rules every Rust change in this workspace honours, complementing the broader rules in [coding-standards.md](./coding-standards.md). The external baseline is the [Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/guidelines/index.html); each section below is a house delta layered on top, and where one disagrees with the baseline, this document wins.

## Naming by context

The baseline's M-SHORT-NAMES, sharpened: a type lives in `crates/<crate>/<module>/<file>.rs`, and that path is four words of free context. Don't prefix the type with module-name fragments. Private and `pub(crate)` symbols rarely need disambiguation; re-exports that cross crate boundaries may.

```rust
// crates/engine/src/plugin.rs
// BAD: SourceAdapterLoader GOOD: Loader
```

## Engine failures are Omnia errors

Engine and CLI code does not introduce an `Error` type. Return `omnia_guest::Error` and pick the class on a direct match: `BadRequest` for operator or input refusals, `NotFound` for missing resources, `BadGateway` for upstream or model failures; everything else is `ServerError`. Construct defaults with `bad_request!` and siblings (snake_case `error` field). Keep explicit variants only for `specify-source-required`, `unsupported-version`, and `spec-not-generated`. The adapter WIT seam (`emery_source::types::Error`) is a different contract — do not replace it with Omnia errors.

```rust
// BAD — a house error type, even if it later maps to Omnia.
enum Error {
    ReadProject  { path: PathBuf, source: io::Error },
    ReadRegistry { path: PathBuf, source: io::Error },
}
// GOOD — Omnia class via the crate-root macro.
let path = path.display();
omnia_guest::server_error!("{path} ({source})")
```

## One body per command, no wrapper newtype

Don't introduce a wrapper newtype to hang a rendering off a body. Implement the façade's `Text` trait (`crates/cli/src/text.rs`) on the engine body itself — the orphan rule permits a local trait on a foreign type — and keep `std::fmt::Display` off engine bodies altogether: their terminal shape is the CLI's contract, not the engine's. If the same rendering appears in three command files, it's one body — promote it.

```rust
// BAD — wrapper newtype existing only to carry a rendering.
struct SpecifyText<'a>(&'a SpecifyBody);
impl Text for SpecifyText<'_> { /* ... */ }
// GOOD — Text on the body, in the façade.
impl Text for SpecifyBody { /* ... */ }
```

## No traits for testability alone

House rule — generic advice about abstracting dependencies for mockability does not apply here. Don't introduce a trait whose only non-test impl is `RealX`. The right test boundary is the lowest external surface — `std::process::Command` or the filesystem. When a stable in-tree boundary already exists — for example the storage capability pair (`omnia_guest::StateStore` / `BlobStore`) every engine-state write goes through, scripted in memory by native tests — use that instead of inventing a sibling trait pair.

```rust
// BAD — trait pair that exists so MockGenerationStore can swap in.
trait GenerationStore { fn load(&self) -> Result<SpecSet>; }
struct RealGenerationStore;
// GOOD — write through the existing capability boundary.
store.cas(CURRENT_KEY, observed.as_deref(), id.as_bytes()).await?;
```

## Reach for the standard crate first

Before writing a macro or a trait, search crates.io. Top-1000 crates that fit beat hand-rolled equivalents: `strum` for kebab-case enum mirrors, `thiserror` for error layering, `anyhow` for error wrapping in tests, `derive_more` for trivial newtype impls.

```rust
// BAD — hand-rolled Display/FromStr mirror of a Serialize derive.
impl Display for Kind { /* match arm per variant */ }
// GOOD — derive it.
#[derive(Serialize, Deserialize, strum::Display, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
enum Kind { /* ... */ }
```

## No archaeology in code

Comments — doc comments and `//` line comments alike — describe what the code *does today*, in ≤ 3 lines. Historical framing — "Phase 1 …", "old contract renamed …", "previously lived in …", "former tests collapse here", "to avoid the X → Y cycle" — is deleted, not relocated; git history is the record. The density caps (module `//!` 1–3 prose lines, `///` overview under ~8, `//` runs ≤ 3) are review-only — see [coding-standards.md § Comments](./coding-standards.md#comments).

```rust
// BAD
//! the pre-cutover name was `charter`. To avoid the
//! foo → bar → foo cycle we re-export `Layout` from here.
// GOOD
//! Resolves the deployed preopen layout for every command.
```
