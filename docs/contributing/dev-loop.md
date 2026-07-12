# The Developer Loop

One command surface drives day-to-day development across the two sibling checkouts (`specify` and `specify-adapters`). The same `cargo make dev -- <command> [args...]` task exists at both repo roots, backed by one canonical Cargo Script, [`scripts/dev.rs`](https://github.com/augentic/specify/blob/main/scripts/dev.rs); the adapters root delegates to its sibling. `SPECIFY_FRAMEWORK` and `SPECIFY_ADAPTERS` override the sibling layout. The underlying cases are canonical scenarios; each rung selects a runtime/model profile and gate from the [quality model](quality-gates.md).

Start with the doctor, then climb the rungs only as far as the change demands:

```bash
cargo make dev -- doctor          # layout, toolchain, WASI target, cursor-agent
cargo make dev -- doctor --live   # + one real model call proving command-mode credentials
```

`cursor-agent status` proves an IDE login, not command-mode auth — only the `doctor --live` probe (or a live run) exercises the `--print` path the model backends spawn, which needs `cursor-agent login` or `CURSOR_API_KEY`.

## The three rungs

Every edit loop lives on exactly one rung. Escalate only when the lower rung cannot observe the change.

### 1. `dev check` — model-free, no WASM

The default edit loop. Runs native scripted/replay scenario profiles and seam tests in the `specify-adapters` checkout, plus the named adapter's native crate tests when scoped:

```bash
cargo make dev -- check         # harness suite only
cargo make dev -- check omnia   # + that adapter's native tests
```

Nothing here builds a component or calls a live model: the `specify-dev` shim lives at `specify-adapters/harness/native` and links that workspace's adapter crates directly, while consuming the engine crates through a revision-pinned git dependency. Omnia's testkit supplies scripted/replay responses and request recording. `specify-dev init <bare-name> --scaffold-only` needs no `.wasm` artifact. `cargo make dev -- run /path/to/project plan status` drives the same shim against any consumer project without changing directory.

### 2. `dev live` — deliberate repeated model trials

Live-model runs are always explicit, never a side effect. Bare `dev live` selects the workflow scenario's `native-live` profile (`specify-dev`, no WASM builds); naming an adapter runs exactly one adapter-local live quality case:

```bash
cargo make dev -- live                       # native-shim guest execute loop
cargo make dev -- live contracts             # that adapter's default scenario
cargo make dev -- live vectis single_screen
```

For adapter prompt iteration, the prose overlay turns on automatically once the run artifacts exist (a re-run skips cargo entirely; `SPECIFY_PROSE_OVERLAY=0` opts out). To watch one prose tree from the adapters repo, run `EVAL_FILTER=contracts::design cargo watch -w targets/contracts/prose -s 'SPECIFY_PROSE_OVERLAY=1 cargo test -p harness --test live -- --ignored --nocapture --exact "$EVAL_FILTER"'`.

### 3. `dev full` — the WASM boundary

The explicit outer gate, never the default edit loop: `doctor --live`, the deterministic rung, adapter-component WASM/WIT conformance (`cargo test -p harness --test composed` in the adapters repo), the workflow-core model-free composed profile, and the selected workflow scenario's `wasm-live` profile:

```bash
cargo make dev -- full
```

This is the only developer rung that combines live judgment with the wasm-only surface. The deterministic composed profile covers WIT bindings, dispatch-by-id, and mount/preopen wiring in CI; `dev full` adds current-model output quality.

## What CI runs

The deterministic halves are gated automatically; the model legs never are:

- `cargo make ci` in each repo — the per-repo workspace gate.
- `specify-adapters`' ordinary workspace gate — native scripted/replay profiles, linked-runtime seams, adapter crate tests, and adapter-local component conformance.
- Specify's composed job — the model-free workflow-core scenario with echo adapters and no sibling checkout.
- Live-model profiles stay operator-triggered (rungs 2 and 3); CI never requires model credentials.
