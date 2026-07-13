# The Developer Loop

Specify's developer loop is self-contained: every rung runs from this checkout alone, with no sibling repository, no adapter component build, and no model credentials on the default path. The three local rungs mirror the [testing standards](../standards/testing.md#the-three-rungs); climb only as far as the change demands.

```bash
cargo make test                              # fast native integration tests; model-free and no Wasmtime
(cd examples && cargo make test-wasm)        # build two guests and run the WASM boundary smoke
(cd examples && cargo make prompt-eval)      # run the prompt-eval harness
```

## 1. `cargo make test` — the default edit loop

Runs `cargo nextest` over the default workspace members: the workspace crates (including `testkit`), the examples package, and the `checks` package at `tests/`. The workflow suites (`full_loop`, `reconciliation`, `synthesis`, `judgment`, `adapter_seam`, …) drive the real operations through `testkit`'s fixture adapter seams and its scripted / committed-replay model doubles, so the complete `init → author → approve → execute` loop is proven here without a component or a model call.

Nothing on this rung compiles Wasmtime. An ordinary workflow change should never need to leave it.

`cargo make check` is the pre-commit gate: formatting, clippy under `-D warnings`, this rung, doctests, docs, and a compile-only `wasm32-wasip2` check (which includes the adapter shim, so a WIT revision and its seam break in the same push). `cargo make ci` adds vet/deny.

## 2. `cargo make test-wasm` (from `examples/`) — the WASM boundary

Builds `specify.wasm` and `greeting_wasm.wasm`, stages [`examples/greeting/omnia.toml`](../../examples/greeting/omnia.toml) unchanged in a temporary deployment, and runs it through the `runtime!` host in `examples/greeting/runtime.rs`. It asserts only facts unique to the component boundary — the combined world loads, both axes dispatch by id, metadata reads, the guest calls the model host, preopens and the component cache are wired, and the typed error lift works across the seam. A short scripted loop is the vehicle that reaches those seams; drained-loop and artifact-completeness outcomes stay on the native rung.

Escalate here only when the change crosses a WIT, dispatch, hosting, or preopen seam. Cadence: weekly / path-filtered / manual (`.github/workflows/wasm.yaml`), required before release tags — not every push. Expect minutes, not seconds — guest builds plus Wasmtime JIT dominate.

## 3. `cargo make prompt-eval` (from `examples/`) — prompt evaluation

Runs the `prompt-eval` example (`examples/prompt-eval/engine.rs`): plan → execute (refine → build → merge per slice) → finalize over an adversarial lead set, graded by the deterministic validators, with per-leg repair counts reported as the early drift warning. It needs command-mode model credentials — `cursor-agent login` or `CURSOR_API_KEY`; note `cursor-agent status` proves an IDE login, not the `--print` path the model backend spawns.

Live runs are always explicit, never a side effect. The documented cadence: before a release tag, and after any change to the judgment prompts (`crates/slice/prompts/`, `crates/change/prompts/`) or the generated answer schemas (`project::answers` / `slice::answers` and their goldens under `crates/project/answers/` + `crates/slice/answers/`). See [`examples/README.md`](../../examples/README.md).

## What CI runs

- Per push: `cargo make ci` — the self-contained workspace gate (nextest over default members, clippy/doc/doctest/vet/deny, the `wasm32-wasip2` compile check). No sibling checkout, no component hosting, no model.
- Weekly / path-filtered / manual: the WASM workflow, running rung 2.
- Never: rung 3. CI never requires model credentials.

`specify-adapters` gates its own crates and components against the published WIT contract and its declared engine pin; neither repository gates on the other's HEAD.
