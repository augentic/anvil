# The Developer Loop

Emery's developer loop is self-contained: every rung runs from this checkout alone, with no sibling repository and no model credentials. The v1 live rungs (`cargo make eval`, `cargo make wasm-run`) were archived at tag `v1` with the workflow they exercised.

```bash
make test                              # native tests; model-free and no Wasmtime
```

## `make test` — the default edit loop

Runs `cargo nextest --workspace` over the workspace. The root scenario suites carry the product: `tests/specify.rs` (the in-process `specify` → `show` arc — bindings, extraction, reconciliation, synthesis, the generation home), `tests/command.rs` (the CLI wire contract: grammar, exit codes, channels), and `tests/plugin.rs` (plugin-rule mentions vs the shipped grammar), all over scripted `Model` + `Source` + storage — without a component, a model call, or filesystem engine state. The surviving crate suites prove independent library contracts and the CLI-impractical engine invariants.

Nothing on this rung compiles Wasmtime. An ordinary change should never need to leave it.

`make check` is the pre-commit gate: formatting, clippy under `-D warnings` (guest deny-list in `crates/clippy.toml`), this rung, doctests, and docs. `make ci` adds the links gate plus vet/deny.

## What CI runs

- Per push: `make ci` — the self-contained workspace gate (nextest `--workspace`, clippy/doc/doctest/links/vet/deny). No sibling checkout, no model.
- CI never requires model credentials.

`emery-adapters` gates its own crates and components against the published WIT contract and its declared engine pin; neither repository gates on the other's HEAD.
