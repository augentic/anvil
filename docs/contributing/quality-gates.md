# Quality gates

Emery proves engine correctness from this repository alone. The placement rules live in [testing standards](../standards/testing.md); the [developer loop](dev-loop.md) maps ordinary changes onto commands. This page is the gate model — what runs when, and what each gate is allowed to prove.

## Gate 1 — repository correctness (every push)

`make ci` owns formatting, lints (native and, through `make wasm`, the adapter SDK and mock adapter for `wasm32-wasip2`), the test suites, and the mdBook links gate (Developer Guide link integrity, `make links`). The native suites inside it are led by the root scenario binaries (`tests/specify.rs`, `tests/command.rs`, `tests/plugin.rs`), which drive the in-process command router over scripted capabilities — the whole product arc from argv to committed storage, plus the CLI wire contract and the plugin-rule grammar check. The surviving crate suites prove independent library contracts (the adapter SDK, prose); CLI-unreachable engine branches are kernel unit tests beside their code.

This gate is model-free and self-contained: no sibling checkout, no live model, no network.

## The WASM boundary

No gate in this repository instantiates a component. The wasm32 side is compiled twice instead: the root build script builds the engine guest for `wasm32-wasip2` on every native build (so `make lint` and `make test` already type-check `emery-cli`, `emery-engine`, and `emery-source` for the guest), and `make wasm` runs clippy over the adapter SDK's export side and the mock adapter for the same target. Instantiating the `emery:adapter/source` seam under the real omnia runtime is `emery-adapters`' conformance rung, which drives every first-party component through the published contract. `make source` / `make runtime` remain for the live Cursor journey.

## Placement decision

When adding coverage, the default write path is a root product scenario — a crate test is the exception, and a `src` unit test the last resort:

1. Put every CLI-reachable behavior in the root scenario suites (`tests/specify.rs`, `tests/command.rs`, `tests/plugin.rs`); a behavior whose subject is the wasm boundary itself belongs to `emery-adapters`' conformance rung.
2. Put an independently useful library contract (the adapter SDK, the prose walker) in that crate's integration suite; the same holds for a product invariant impractical to arrange through the entry points.
3. Put a private dense matrix in a kernel unit test only when integration is impractical.
4. If no deterministic predicate can decide the result, it has no automated home here — adapter output quality belongs to adapter authors, model transport to omnia.

Do not copy an assertion into another gate for reassurance: each fact has one owning surface.

## Boundaries

- omnia's `omnia-test` crate owns the scripted capability doubles: the FIFO request-recording `guest::Scripted` model, the keyed `guest::ScriptedLoader`, and the storage pair (`guest::{Memory, Namespaced}`). Suites own scenario content: scripted answers and assertions.
- `examples/adapter/` is the only adapter double. Do not add another mock adapter or mock-adapter copy.
- External adapters prove their own behavior against the published WIT package in `emery-adapters`; no Emery gate resolves that repository, and neither repository gates on the other's HEAD.

## Consistency (links)

Repo invariants that are cheap to enforce in CI and expensive to notice later. Developer Guide link integrity is the mdBook build (`mdbook-linkcheck2` via [`docs/book.toml`](../book.toml)); it runs inside `make ci`. Docs house style is **not** a CI predicate; ultrathin skill body style is guidance in [`docs/standards/cli-contract.md`](../standards/cli-contract.md).

```bash
make links              # Developer Guide link integrity (mdbook build)
make ci                 # the full gate (includes links)
make check              # the pre-commit subset
```

| Invariant                      | Owner                                                                                                          | When it runs                    |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------- | ------------------------------- |
| Developer Guide link integrity | `mdbook-linkcheck2` over [`docs/book.toml`](../book.toml) — `make links` locally, the `links` job in CI per push | Every `make ci` and every push |

Every relative link in the Developer Guide must resolve. Web links are skipped (`follow-web-links = false`); links that leave `docs/` (for example into `crates/` or `AGENTS.md`) are allowed (`traverse-parent-directories = true`). Chapters referenced as in-book targets must appear in `SUMMARY.md`; prefer file hrefs over bare directory paths.
