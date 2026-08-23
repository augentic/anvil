# Quality gates

Emery proves engine correctness from this repository alone. The placement rules live in [testing standards](../standards/testing.md); the [developer loop](dev-loop.md) maps ordinary changes onto commands. This page is the gate model — what runs when, and what each gate is allowed to prove.

## Gate 1 — repository correctness (every push)

`cargo make ci` owns formatting, lints, schemas, the test suites, and the mdBook links gate (Developer Guide link integrity, `cargo make links`). The native suites inside it are led by the root scenario binaries (`tests/specify.rs`, `tests/command.rs`, `tests/shelf.rs`, `tests/plugin.rs`), which drive the in-process command router and HTTP listener over scripted capabilities — the whole product arc from argv to committed storage, plus the CLI wire contract, the MCP spec shelf with its C3 HTTP refusal, and the plugin-rule grammar check. The surviving crate suites prove independent library contracts and the CLI-impractical engine invariants.

This gate is model-free and self-contained: no sibling checkout, no adapter component build, no Wasmtime in the test compile path.

## The WASM seam

No test builds or spawns the mock source component. The `source` and `runtime` examples remain for local component-shape work (`cargo make source` / `cargo make runtime`). Run those when a change crosses a WIT, dispatch, hosting, or preopen seam.

## Placement decision

When adding coverage, the default write path is a root product scenario — a crate test is the exception, and a `src` unit test the last resort:

1. Put every CLI- or MCP-reachable behavior in the root scenario suites (`tests/specify.rs`, `tests/command.rs`, `tests/shelf.rs`, `tests/plugin.rs`). The `source` and `runtime` examples are not a test rung.
2. Put an independently useful library contract (the adapter SDK, artifact parsers, diagnostics, error display, the prose walker) in that crate's integration suite; the same holds for a product invariant impractical to arrange through the entry seams.
3. Put a private dense matrix in a kernel unit test only when integration is impractical.
4. If no deterministic predicate can decide the result, it has no automated home here — adapter output quality belongs to adapter authors, model transport to omnia.

Do not copy an assertion into another gate for reassurance: each fact has one owning seam.

## Boundaries

- `omnia-testkit` owns reusable model doubles, recording, temporary manifests, and runtime hosting. `emery-testkit` owns scripted storage doubles (`StateStore` / `BlobStore`). Suites own scenario content: scripted answers and assertions.
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
