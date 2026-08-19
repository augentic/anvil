# Style

Cross-cutting code-quality rules every Rust change in this workspace honours, complementing the broader rules in [coding-standards.md](./coding-standards.md). The external baseline is the [Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/guidelines/index.html); each section below is a house delta layered on top, and where one disagrees with the baseline, this document wins.

## Naming by context

The baseline's M-SHORT-NAMES, sharpened: a type lives in `crates/<crate>/<module>/<file>.rs`, and that path is four words of free context. Don't prefix the type with module-name fragments. Private and `pub(crate)` symbols rarely need disambiguation; re-exports that cross crate boundaries may.

```rust
// crates/engine/src/resolve/resolver.rs
// BAD: AdapterResolverComponent GOOD: Component
// crates/engine/src/resolve/ensure.rs
// BAD: ResolveEnsureError       GOOD: Error
```

## Error variants budgeted by recovery, not source

If two variants of an error enum collapse to the same `Diag` code, exit code, or human action, they should be one variant with a `kind: …` discriminator, not two. Per-field `///` docs on `pub` structs whose names are self-evident (`path: PathBuf`, `source: io::Error`) are forbidden — keep variant-level docs only.

```rust
// BAD — three variants, one exit code, one recovery path.
enum Error {
    ReadProject  { path: PathBuf, source: io::Error },
    ReadRegistry { path: PathBuf, source: io::Error },
    ReadPlan     { path: PathBuf, source: io::Error },
}
// GOOD
enum Error {
    /// Failed to read a managed file under `.emery/`.
    Read { kind: ReadKind, path: PathBuf, source: io::Error },
}
```

## One body per command, no wrapper newtype

Don't introduce `XxxBody` to hang `Render` off a domain type. Move `Render` onto the domain type, or pass an inline closure to `ctx.emit_with`. If the same wrapper appears in three command files, it's a domain concept — promote it to the crate that owns the type.

```rust
// BAD — wrapper newtype existing only to carry Render.
struct ContextRenderInput<'a>(&'a ResolvedContext);
impl Render for ContextRenderInput<'_> { /* ... */ }
// GOOD — Render on the domain type, or:
ctx.emit_with(&resolved, |w, r| write_resolved(w, r))?;
```

## No traits for testability alone

House rule — generic advice about abstracting dependencies for mockability does not apply here. Don't introduce a trait whose only non-test impl is `RealX`. The right test boundary is the lowest external surface — `std::process::Command` or the filesystem. When a stable in-tree boundary already exists — for example the `emery_artifacts::atomic` write envelope every `.emery/` YAML write goes through — use that instead of inventing a sibling trait pair.

```rust
// BAD — trait pair that exists so MockProjectStore can swap in.
trait ProjectStore { fn load(&self) -> Result<Project>; }
struct RealProjectStore;
// GOOD — write through the existing shared boundary.
emery_artifacts::atomic::yaml_write(&path, &project)?;
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

Comments — doc comments and `//` line comments alike — describe what the code *does today*, in ≤ 3 lines. Historical framing — "Phase 1 …", "old contract renamed …", "previously lived in …", "former tests collapse here", "to avoid the X → Y cycle" — is deleted, not relocated; git history is the record. The density caps (module `//!` 1–3 prose lines, `///` overview under ~8, `//` runs ≤ 3) are mechanically enforced by the `doc_brevity` root-crate test — see [coding-standards.md § Comments](./coding-standards.md#comments).

```rust
// BAD
//! the pre-cutover name was `charter`. To avoid the
//! foo → bar → foo cycle we re-export `Layout` from here.
// GOOD
//! Resolves project layout and `project.yaml` for every command.
```
