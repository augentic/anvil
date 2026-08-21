# The Developer Loop

Emery's developer loop is self-contained: every rung runs from this checkout alone, with no sibling repository and no model credentials. The v1 live rungs (`cargo make eval`, `cargo make wasm-run`) were archived at tag `v1` with the workflow they exercised.

```bash
cargo make test                              # native tests; model-free and no Wasmtime
```

## `cargo make test` — the default edit loop

Runs `cargo nextest --workspace` over the workspace crates. The suites prove the pure engine kernels (the output home, the extras gate, reconciliation and synthesis over scripted model doubles), the CLI wire contract (grammar, exit codes, the C3 HTTP refusal), and the in-process `init` → `specify` journey (`tests/native.rs`) — without a component or a model call.

Nothing on this rung compiles Wasmtime. An ordinary change should never need to leave it.

`cargo make check` is the pre-commit gate: formatting, clippy under `-D warnings` (guest deny-list in `crates/clippy.toml`), this rung, doctests, and docs. `cargo make ci` adds the links gate plus vet/deny.

## What CI runs

- Per push: `cargo make ci` — the self-contained workspace gate (nextest `--workspace`, clippy/doc/doctest/links/vet/deny). No sibling checkout, no model.
- CI never requires model credentials.

`emery-adapters` gates its own crates and components against the published WIT contract and its declared engine pin; neither repository gates on the other's HEAD.
