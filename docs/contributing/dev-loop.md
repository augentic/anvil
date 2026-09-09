# The Developer Loop

Emery's developer loop is self-contained: every rung runs from this checkout alone, with no sibling repository and no model credentials. The v1 live rungs (`cargo make eval`, `cargo make wasm-run`) were archived at tag `v1` with the workflow they exercised.

```bash
make test                              # native suites; model-free
```

## `make test` — the default edit loop

Runs `cargo nextest --workspace` over the workspace. The root scenario suites carry the product: `tests/specify.rs` (the in-process `specify` → `show` arc — bindings, extraction, the grouping and draft judgments, canonical rendering, the revision store), `tests/command.rs` (the CLI wire contract: grammar, exit codes, channels), and `tests/plugin.rs` (plugin-rule mentions vs the shipped grammar), all over scripted `Model` + `Source` + storage — without a component, a model call, or filesystem engine state. The surviving crate suites prove independent library contracts (the adapter SDK, prose); CLI-unreachable engine branches are kernel unit tests beside their code. No suite compiles or instantiates a component: the root build script compiles the engine guest for wasm32 on every native build, and `make wasm` lints the adapter SDK and the mock adapter for `wasm32-wasip2`.

An ordinary change should never need to leave this rung.

`make check` is the pre-commit gate: formatting, clippy under `-D warnings` (guest deny-list in `crates/clippy.toml`), `make wasm`, this rung, doctests, docs, and the links gate. `make ci` adds vet/deny.

## Keeping `target/` bounded

Cargo never garbage-collects `target/`: every distinct feature set, profile, `RUSTFLAGS`, or `Cargo.lock` revision leaves its own copy of the dependency tree behind, and wasmtime plus Cranelift are the bulk of it. Two guards keep that in check — dependencies build with `debug = "line-tables-only"` (workspace crates keep full debuginfo), and the root `build.rs` compiles the wasm32 engine into one shared `target/wasm32-engine` directory instead of a fresh tree under each per-hash `OUT_DIR`. Run `make sweep` (needs `cargo install cargo-sweep`) periodically, and `make clean` after a package rename or a large lock bump, since a renamed package's old build directories are never reused.

## What CI runs

- Per push: `make ci` — the self-contained workspace gate (nextest `--workspace`, clippy/doc/doctest/links/vet/deny). No sibling checkout, no model.
- CI never requires model credentials.

`emery-adapters` gates its own crates and components against the published WIT contract and its declared engine pin; neither repository gates on the other's HEAD.
