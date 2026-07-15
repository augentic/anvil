# The Developer Loop

Specify's developer loop is self-contained: every rung runs from this checkout alone, with no sibling repository, no adapter component build, and no model credentials on the default path. The two local rungs mirror the [testing standards](../standards/testing.md#the-two-rungs); climb only as far as the change demands.

```bash
cargo make test                              # fast native integration tests; model-free and no Wasmtime
cargo make eval                              # run the prompt-evaluation harness
```

## 1. `cargo make test` — the default edit loop

Runs `cargo nextest --workspace` over the workspace crates (including `testkit` and `checks`). The workflow suites (`full_loop`, `reconciliation`, `synthesis`, `judgment`, `adapter_seam`, …) drive the real operations through `testkit`'s fixture adapter seams and its scripted model doubles, so the complete `init → author → approve → execute` loop is proven here without a component or a model call.

Nothing on this rung compiles Wasmtime. An ordinary workflow change should never need to leave it.

`cargo make check` is the pre-commit gate: formatting, clippy under `-D warnings`, this rung, doctests, and docs. `cargo make ci` adds vet/deny. When a change crosses a WIT, dispatch, or preopen seam, also compile-check the wasm32 guests — `cargo check --lib -p specify --example change --target wasm32-wasip2` — so a WIT revision and its seam break in the same push.

## 2. `cargo make eval` — prompt evaluation

Runs the `eval` harness (`crates/eval`): plan → execute (refine → build → merge per slice) → finalize over an adversarial lead set, graded by the deterministic validators, with per-leg repair counts reported as the early drift warning. It needs command-mode model credentials — `cursor-agent login` or `CURSOR_API_KEY`; note `cursor-agent status` proves an IDE login, not the `--print` path the model backend spawns.

Live runs are always explicit, never a side effect. The documented cadence: before a release tag, and after any change to the judgment prompts (`crates/slice/prompts/`, `crates/change/prompts/`) or the generated answer schemas (`project::answers` / `slice::answers` and their goldens under `crates/project/answers/` + `crates/slice/answers/`). See [`crates/eval/README.md`](../../crates/eval/README.md).

## The WASM seam

There is no automated WASM boundary rung. The component seam — the combined `change.wasm` loading through the checked-in `omnia.toml`, dispatch-by-id on both axes, metadata reads, guest-to-host model wiring, preopens — is exercised by the operator-invoked change example: `cargo make change-run` (live model; `CURSOR_API_KEY` in `examples/.env`; see [`examples/change/README.md`](../../examples/change/README.md)). Run it when a change crosses a WIT, dispatch, hosting, or preopen seam. Expect minutes, not seconds — guest builds plus Wasmtime JIT dominate.

## What CI runs

- Per push: `cargo make ci` — the self-contained workspace gate (nextest `--workspace`, clippy/doc/doctest/vet/deny). No sibling checkout, no component hosting, no model.
- Never: the eval rung or the change example. CI never requires model credentials.

`specify-adapters` gates its own crates and components against the published WIT contract and its declared engine pin; neither repository gates on the other's HEAD.
