# The Developer Loop

Emery's developer loop is self-contained: every rung runs from this checkout alone, with no sibling repository, no adapter component build, and no model credentials. The v1 live rungs (`cargo make eval`, `cargo make wasm-run`) were archived at tag `v1` with the workflow they exercised; new live rungs return with the spec walking skeleton.

```bash
cargo make test                              # fast native integration tests; model-free and no Wasmtime
```

## `cargo make test` — the default edit loop

Runs `cargo nextest --workspace` over the workspace crates (including `mock` and `native`). The suites drive the real operations — `init`, the `specify` stub, adapter resolution — through the `mock` catalog behind the offline `native` provider and scripted model doubles, without a component or a model call.

Nothing on this rung compiles Wasmtime. An ordinary change should never need to leave it.

`cargo make check` is the pre-commit gate: formatting, clippy under `-D warnings`, this rung, doctests, and docs. `cargo make ci` adds the links gate plus vet/deny. When a change crosses a WIT, dispatch, or preopen seam, also compile-check the wasm32 guest — `cargo check --lib -p emery --target wasm32-wasip2` — so a WIT revision and its seam break in the same push.

## What CI runs

- Per push: `cargo make ci` — the self-contained workspace gate (nextest `--workspace`, clippy/doc/doctest/links/vet/deny). No sibling checkout, no component hosting, no model.
- CI never requires model credentials.

`emery-adapters` gates its own crates and components against the published WIT contract and its declared engine pin; neither repository gates on the other's HEAD.
