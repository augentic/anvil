# The Developer Loop

One command surface drives day-to-day development across the two sibling checkouts (`specify` and `specify-adapters`). The same `make dev-*` targets exist at both repo roots, backed by one canonical orchestration script, [`scripts/dev.sh`](https://github.com/augentic/specify/blob/main/scripts/dev.sh); the adapters root delegates to its sibling. `SPECIFY_FRAMEWORK` and `SPECIFY_ADAPTERS` override the sibling layout.

Start with the doctor, then climb the rungs only as far as the change demands:

```bash
make dev-doctor          # layout, toolchain, WASI target, cursor-agent
make dev-doctor LIVE=1   # + one real model call proving command-mode credentials
```

`cursor-agent status` proves an IDE login, not command-mode auth — only the `LIVE=1` probe (or a live run) exercises the `--print` path the model backends spawn, which needs `cursor-agent login` or `CURSOR_API_KEY`.

## The three rungs

Every edit loop lives on exactly one rung. Escalate only when the lower rung cannot observe the change.

### 1. `dev-check` — model-free, no WASM

The default edit loop. Runs the native harness seam/replay suite in the `specify` checkout, plus the named adapter's native crate tests when scoped:

```bash
make dev-check                 # harness suite only
make dev-check ADAPTER=omnia   # + that adapter's native tests
```

Nothing here builds a component or calls a model: the `specify-dev` shim links the adapter crates directly (see [`harness/README.md`](https://github.com/augentic/specify/blob/main/harness/README.md)), and `specify-dev init <bare-name> --scaffold-only` needs no `.wasm` artifact at all. `make dev-run PROJECT=/path/to/project ARGS='plan status'` drives the same shim against any consumer project without changing directory.

### 2. `dev-live` — one deliberate model run

Live-model runs are always explicit, never a side effect. Bare `dev-live` runs the Specify workflow loop through the native shim (`SPECIFY_SHIM=native`, no WASM builds, no deployment manifest); naming an adapter runs exactly one live eval scenario:

```bash
make dev-live                                    # native-shim guest execute loop
make dev-live ADAPTER=contracts                  # that adapter's default scenario
make dev-live ADAPTER=vectis SCENARIO=single_screen
```

For adapter prompt iteration, the prose overlay turns on automatically once the run artifacts exist (a re-run skips cargo entirely; `SPECIFY_PROSE_OVERLAY=0` opts out), and `cargo make eval-watch` in the adapters repo re-runs one scenario on every prose save (`EVAL_FILTER=contracts::design cargo make eval-watch`).

### 3. `dev-full` — the WASM boundary

The explicit outer gate, never the default edit loop: `doctor --live`, the deterministic rung, the composed WASM/WIT coverage (`cargo test -p evals --test composed` in the adapters repo), and the composed guest execute loop against a real model:

```bash
make dev-full
```

This is the only rung that exercises the wasm-only surface — WIT bindings, dispatch-by-id, mount/preopen wiring. A green rung 1 plus a green rung 3 is the full picture; neither alone is (see the coverage boundary note in [`harness/README.md`](https://github.com/augentic/specify/blob/main/harness/README.md)).

## What CI runs

The deterministic halves are gated automatically; the model legs never are:

- `cargo make ci` in each repo — the per-repo workspace gate.
- The cross-repo harness job (`.github/workflows/ci.yaml`, `harness` job) — checks out the sibling adapters repo and runs `dev-check`'s content (native harness tests + harness clippy). Model-free by construction.
- Live-model evaluation stays operator-triggered (rungs 2 and 3); the composed tests remain the WASM/WIT gate inside the adapters repo's own suite.
