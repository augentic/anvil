# The Developer Loop

Specify's developer loop is self-contained: every rung runs from this checkout alone, with no sibling repository, no adapter component build, and no model credentials on the default path. The three local rungs mirror the [testing standards](../standards/testing.md#the-three-rungs); climb only as far as the change demands.

```bash
cargo make test       # fast native integration tests; model-free and no Wasmtime
cargo make test-wasm  # build two guests and run the composed smoke test
cargo make test-live  # run the explicit live-model workflow test
```

## 1. `cargo make test` — the default edit loop

Runs `cargo nextest` over the default workspace members: the workspace crates, the fixture adapter's native core, and the framework-quality checks at `tests/framework/`. The workflow suites (`full_loop`, `reconciliation`, `synthesis`, `judgment`, `adapter_seam`, …) drive the real operations through the fixture adapter's native seams and `omnia-testkit` scripted models, so the complete `init → author → approve → execute` loop is proven here without a component or a model call.

Nothing on this rung compiles Wasmtime. An ordinary workflow change should never need to leave it.

`cargo make check` is the pre-commit gate: formatting, clippy under `-D warnings`, this rung, doctests, docs, and a compile-only `wasm32-wasip2` check (which includes the fixture-adapter shim, so a WIT revision and its seam break in the same push). `cargo make ci` adds vet/deny.

## 2. `cargo make test-wasm` — the WASM boundary

Builds `specify.wasm` and the combined `fixture_adapter.wasm`, hosts them in one temporary deployment, and runs the single composed smoke (`harness/composed`). It asserts only facts unique to the component boundary — the combined world loads, both axes dispatch by id, metadata reads, the guest reaches the model host, preopens and the component cache are wired — and that the loop lands in the same terminal state as the native test.

Escalate here only when the change crosses a WIT, dispatch, hosting, or preopen seam. Cadence: the scheduled/manual composed workflow (`.github/workflows/composed.yaml`), not per push. Expect minutes, not seconds — guest builds plus Wasmtime JIT dominate.

## 3. `cargo make test-live` — the explicit live trial

Runs the one ignored native live-model test (`harness/live`): the same fixture workflow over an adversarial lead set, graded by the deterministic validators, with per-leg repair counts reported as the early drift warning. It needs command-mode model credentials — `cursor-agent login` or `CURSOR_API_KEY`; note `cursor-agent status` proves an IDE login, not the `--print` path the model backend spawns.

Live runs are always explicit, never a side effect. The documented cadence: before a release tag, and after any change to the judgment prompts (`crates/slice/prompts/`, `crates/change/prompts/`) or the answer schemas (`schemas/answers/`). See `harness/live/README.md`.

## What CI runs

- Per push: `cargo make ci` — the self-contained workspace gate (nextest over default members, clippy/doc/doctest/vet/deny, the `wasm32-wasip2` compile check). No sibling checkout, no component hosting, no model.
- Scheduled/manual: the composed workflow, running rung 2.
- Never: rung 3. CI never requires model credentials.

`specify-adapters` gates its own crates and components against the published WIT contract and its declared engine pin; neither repository gates on the other's HEAD.
