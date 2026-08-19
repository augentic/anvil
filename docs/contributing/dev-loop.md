# The Developer Loop

Emery's developer loop is self-contained: every rung runs from this checkout alone, with no sibling repository and no model credentials. The v1 live rungs (`cargo make eval`, `cargo make wasm-run`) were archived at tag `v1` with the workflow they exercised.

```bash
cargo make test                              # fast native tests; model-free and no Wasmtime
cargo make journey                           # the seam rung: shipped shape over built components
```

## `cargo make test` — the default edit loop

Runs `cargo nextest --workspace` over the workspace crates. The suites prove the pure engine kernels (the output home, the extras gate, reconciliation and synthesis over scripted model doubles) and the CLI wire contract (grammar, exit codes, the C3 HTTP refusal) — without a component or a model call.

Nothing on this rung compiles Wasmtime. An ordinary change should never need to leave it.

`cargo make check` is the pre-commit gate: formatting, clippy under `-D warnings`, this rung, doctests, and docs. `cargo make ci` adds the links gate plus vet/deny. When a change crosses a WIT, dispatch, or preopen seam, also compile-check the wasm32 guest — `cargo check --lib -p emery --target wasm32-wasip2` — so a WIT revision and its seam break in the same push.

## `cargo make journey` — the seam rung

Builds the mock source components (`cargo make mock-component`) and the dev-only journey host, then drives the walking-skeleton journey (`tests/journey.rs`) across the real component seam: `init` over local components, `specify` end-to-end with a scripted model, the generation-pointer swap. This is the one integration rung (ADR-0002); run it when a change touches dispatch, admission, the WIT seam, or the `init`/`specify` behavior itself.

## What CI runs

- Per push: `cargo make ci` — the self-contained workspace gate (nextest `--workspace`, clippy/doc/doctest/links/vet/deny) — and the required `journey` job (`cargo make journey`). No sibling checkout, no model.
- CI never requires model credentials.

`emery-adapters` gates its own crates and components against the published WIT contract and its declared engine pin; neither repository gates on the other's HEAD.
