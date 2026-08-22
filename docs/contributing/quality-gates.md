# Quality gates

Emery proves engine correctness from this repository alone. The placement rules live in [testing standards](../standards/testing.md); the [developer loop](dev-loop.md) maps ordinary changes onto commands. This page is the gate model — what runs when, and what each gate is allowed to prove.

## Gate 1 — repository correctness (every push)

`cargo make ci` owns formatting, lints, schemas, crate and wire integration, and the mdBook links gate (Developer Guide link integrity, `cargo make links`). The native suites inside it prove the pure engine kernels (reconciliation, the extras gate, the output home) over scripted models and the CLI wire contract (grammar, exit codes, the C3 HTTP refusal) over an inert provider.

This gate is model-free and self-contained: no sibling checkout, no adapter component build, no Wasmtime in the test compile path.

## The WASM seam

No test builds or spawns the mock source component. The model-free signal is the guest lint leg: `cargo make lint` runs clippy over `--target wasm32-wasip2` with the guest deny-list (`clippy-wasm/clippy.toml`), subsuming the old compile check. The `source` and `runtime` examples remain for local component-shape work (`cargo make source` / `cargo make runtime`). Run those when a change crosses a WIT, dispatch, hosting, or preopen seam.

## Placement decision

When adding coverage, the default write path is crate or wire integration — a `src` unit test is the exception, never the starting point:

1. Put a private dense matrix in a kernel unit test only when integration is impractical.
2. Put one-crate public behavior in that crate's integration suite.
3. Put cross-crate `init` / `specify` orchestration on the native rung (`tests/source.rs`). The `source` and `runtime` examples are not a test rung.
4. If no deterministic predicate can decide the result, it has no automated home here — adapter output quality belongs to adapter authors, model transport to omnia.

Do not copy an assertion into another gate for reassurance: each fact has one owning seam.

## Boundaries

- `omnia-testkit` owns reusable model doubles, recording, temporary manifests, and runtime hosting. Emery owns scenario content: scripted answers and assertions.
- `examples/source.rs` is the only adapter double. Do not add another mock adapter or mock-adapter copy.
- External adapters prove their own behavior against the published WIT package in `emery-adapters`; no Emery gate resolves that repository, and neither repository gates on the other's HEAD.

## Consistency (links)

Repo invariants that are cheap to enforce in CI and expensive to notice later. Developer Guide link integrity is the mdBook build (`mdbook-linkcheck2` via [`docs/book.toml`](../book.toml)); it runs inside `cargo make ci`. Docs house style is **not** a CI predicate; ultrathin skill body style is guidance in [`docs/standards/cli-contract.md`](../standards/cli-contract.md).

```bash
cargo make links              # Developer Guide link integrity (mdbook build)
cargo make ci                 # the full gate (includes links)
cargo make check              # the pre-commit subset
```

| Invariant                      | Owner                                                                                                                   | When it runs                         |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------ |
| Developer Guide link integrity | `mdbook-linkcheck2` over [`docs/book.toml`](../book.toml) — `cargo make links` locally, the `links` job in CI per push   | Every `cargo make ci` and every push |

Every relative link in the Developer Guide must resolve. Web links are skipped (`follow-web-links = false`); links that leave `docs/` (for example into `crates/` or `AGENTS.md`) are allowed (`traverse-parent-directories = true`). Chapters referenced as in-book targets must appear in `SUMMARY.md`; prefer file hrefs over bare directory paths.
