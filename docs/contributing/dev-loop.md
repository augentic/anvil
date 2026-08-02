# The Developer Loop

Emery's developer loop is self-contained: every rung runs from this checkout alone, with no sibling repository, no adapter component build, and no model credentials on the default path. The two local rungs mirror the [testing standards](../standards/testing.md#the-two-rungs); climb only as far as the change demands.

```bash
cargo make test                              # fast native integration tests; model-free and no Wasmtime
cargo make eval auth --restart               # run the live prompt-evaluation rung (one eval case; bare `cargo make eval` lists them)
```

## 1. `cargo make test` — the default edit loop

Runs `cargo nextest --workspace` over the workspace crates (including `mock` and `native`). The engine suites (`full_loop`, `reconciliation`, `synthesis`, `judgment`, `adapter_seam`, …) drive the real operations through the `mock` catalog behind the offline `native` provider and its scripted model doubles, so the complete `init → author → execute` loop is proven here without a component or a model call.

Nothing on this rung compiles Wasmtime. An ordinary workflow change should never need to leave it.

`cargo make check` is the pre-commit gate: formatting, clippy under `-D warnings`, this rung, doctests, and docs. `cargo make ci` adds vet/deny. When a change crosses a WIT, dispatch, or preopen seam, also compile-check the wasm32 guests — `cargo check --lib -p emery --examples --target wasm32-wasip2` — so a WIT revision and its seam break in the same push.

## 2. `cargo make eval` — prompt evaluation

Runs one live eval case (the `crates/probe` case runner composed by the root `eval` example). The engine's `auth` workflow case drives plan → execute (refine → build → merge per slice) over an adversarial lead set, graded by the deterministic validators, with per-leg repair counts reported as the early drift warning; `--until plan` stops after plan author to inspect the authored plan. The hard synthesis case is authority divergence (`session-timeout` / `session-policy`), not evidence volume. It needs command-mode model credentials — `cursor-agent login` or `CURSOR_API_KEY`; note `cursor-agent status` proves an IDE login, not the `--print` path the model backend spawns. `CURSOR_MODEL` and `CURSOR_TIMEOUT_SECS` are documented in [`crates/probe/README.md`](../../crates/probe/README.md).

If you want to set environment variables in a file, see `.env.example`. Copy to `.env` and set variables then run

```
set -a && source .env && set +a && cargo make eval auth --restart
```

Live runs are always explicit, never a side effect. The documented cadence: before a release tag, and after any change to the judgment prompts (`crates/slice/prompts/`, `crates/change/prompts/`) or the generated answer schemas (`project::answers` / `slice::answers` and their goldens under `crates/project/answers/` + `crates/slice/answers/`). Composition surface: [`examples/eval/README.md`](../../examples/eval/README.md); case/grading mechanics: [`crates/probe/README.md`](../../crates/probe/README.md).

Each case keeps one stable retained sandbox at `sandbox/<case>/` (composition-owned root beside the wasm example's `sandbox/wasm/`), on success and failure alike. `--restart` is the only runner-owned reset; an existing sandbox without `--restart` refuses before mutation. Continue or debug a retained sandbox explicitly with `cargo make lab -- --project-dir sandbox/auth <verb…>` (e.g. `plan execute` after `--until plan`).

## The WASM seam

There is no automated WASM boundary rung. The component seam — the embedded engine guest, the per-axis mock components faulting in through the fail-closed resolver, dispatch-by-id on both axes, metadata reads, guest-to-host model wiring, preopens — is exercised by the operator-invoked wasm example: `cargo make wasm-run` (live model; `CURSOR_API_KEY` in `examples/.env`; see [examples/wasm/README.md](../../examples/wasm/README.md)). Run it when a change crosses a WIT, dispatch, hosting, or preopen seam. Expect minutes, not seconds — guest builds plus Wasmtime JIT dominate.

## What CI runs

- Per push: `cargo make ci` — the self-contained workspace gate (nextest `--workspace`, clippy/doc/doctest/vet/deny). No sibling checkout, no component hosting, no model.
- Never: the eval rung or the wasm example. CI never requires model credentials.

`emery-adapters` gates its own crates and components against the published WIT contract and its declared engine pin; neither repository gates on the other's HEAD.
