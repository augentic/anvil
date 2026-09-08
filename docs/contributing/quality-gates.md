# Quality gates

Emery proves engine correctness from this repository alone. The placement rules live in [testing standards](../standards/testing.md); the [developer loop](dev-loop.md) maps ordinary changes onto commands. This page is the gate model — what runs when, and what each gate is allowed to prove.

## Gate 1 — repository correctness (every push)

`make ci` owns formatting, lints, schemas, the test suites, and the mdBook links gate (Developer Guide link integrity, `make links`). The native suites inside it are led by the root scenario binaries (`tests/specify.rs`, `tests/command.rs`, `tests/plugin.rs`), which drive the in-process command router over scripted capabilities — the whole product arc from argv to committed storage, plus the CLI wire contract and the plugin-rule grammar check. The surviving crate suites prove independent library contracts (the adapter SDK, prose); CLI-unreachable engine branches are kernel unit tests beside their code.

This gate is model-free and self-contained: no sibling checkout, no live model, no network.

## The WASM boundary

`examples/component/tests/component.rs` is the component rung and runs inside the same `make test` as its own unpublished workspace package: the shipped deployment (`emery::manifest()`, overlaid by the package's `run` harness through `omnia_test::host::Deployment`) and the built mock adapter component instantiated under the real omnia runtime, over omnia-test's scripted host-side model and in-memory storage backends. `examples/component/build.rs` drives `omnia_test::build::Components` to compile `--example adapter` for `wasm32-wasip2` (the root build script builds only the engine, for every build of the binary), so this is the one place the gate compiles a component — incremental after the first build. The rung owns the boundary alone: `wasi:cli/run`, the seam lowering, the real path loader and digest pin, a seamless component refused, the reference-tool streams. `make source` / `make runtime` remain for the live Cursor journey.

## Placement decision

When adding coverage, the default write path is a root product scenario — a crate test is the exception, and a `src` unit test the last resort:

1. Put every CLI-reachable behavior in the root scenario suites (`tests/specify.rs`, `tests/command.rs`, `tests/plugin.rs`); a scenario goes to the component rung (`examples/component/tests/component.rs`) only when the wasm boundary is its subject.
2. Put an independently useful library contract (the adapter SDK, the prose walker) in that crate's integration suite; the same holds for a product invariant impractical to arrange through the entry points.
3. Put a private dense matrix in a kernel unit test only when integration is impractical.
4. If no deterministic predicate can decide the result, it has no automated home here — adapter output quality belongs to adapter authors, model transport to omnia.

Do not copy an assertion into another gate for reassurance: each fact has one owning surface.

## Boundaries

- omnia's `omnia-test` crate owns the scripted capability doubles: the FIFO request-recording `guest::Scripted` model, the storage pair (`guest::{Memory, Namespaced}`), and the host-side `host::{ScriptedModel, Backends}`. The `component` package owns only the component rung's fixture build, the `run` overlay of the shipped deployment, and the rung's scenarios. Suites own scenario content: scripted answers and assertions.
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
