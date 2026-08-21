# The Developer Loop

Emery's developer loop is self-contained: every rung runs from this checkout alone, with no sibling repository and no model credentials. The v1 live rungs (`cargo make eval`, `cargo make wasm-run`) were archived at tag `v1` with the workflow they exercised.

```bash
cargo make test                              # native tests; model-free and no Wasmtime
```

## `cargo make test` — the default edit loop

Runs `cargo nextest --workspace` over the workspace crates. The suites prove the pure engine kernels (the output home, the extras gate, reconciliation and synthesis over scripted model doubles), the CLI wire contract (grammar, exit codes, the C3 HTTP refusal), and the in-process `init` → `specify` journey (`tests/native.rs`) — without a component or a model call.

Nothing on this rung compiles Wasmtime. An ordinary change should never need to leave it.

`cargo make check` is the pre-commit gate: formatting, the three lint legs (stock clippy under `-D warnings`, the house Dylint lints from [augentic/lints](https://github.com/augentic/lints), the wasm32 guest deny-list), this rung, doctests, and docs. `cargo make ci` adds the links gate plus vet/deny. The wasm lint leg checks the guest crates on every run, so a WIT revision and its seam break in the same push.

Optional IDE integration: rust-analyzer can run the house lints on save via `"rust-analyzer.check.overrideCommand": ["cargo", "dylint", "--all", "--workspace", "--", "--all-targets", "--message-format=json"]`. It is slower than stock clippy-on-save and entirely opt-in — `cargo make lint` in CI remains the gate.

## What CI runs

- Per push: `cargo make ci` — the self-contained workspace gate (nextest `--workspace`, clippy/doc/doctest/links/vet/deny). No sibling checkout, no model.
- CI never requires model credentials.

`emery-adapters` gates its own crates and components against the published WIT contract and its declared engine pin; neither repository gates on the other's HEAD.
