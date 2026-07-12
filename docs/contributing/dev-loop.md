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

Nothing here builds a component or calls a live model: the `specify-dev` shim lives at `specify-adapters/harness/native` — a standalone workspace pinned to a declared engine revision — and links that repo's adapter crates directly. `dev check` and `dev run` automatically override that pin with this checkout's working-tree crates through generated `--config` patch flags (the tracked manifest and lockfile stay revision-pinned; nothing is written to either repo), so uncommitted engine changes are exercised against the real adapters. Omnia's testkit supplies scripted/replay responses and request recording. `specify-dev init <bare-name> --scaffold-only` needs no `.wasm` artifact. `cargo make dev -- run /path/to/project plan status` drives the same shim against any consumer project without changing directory.

An engine change that breaks the harness fails here at compile time — that is the design working, not a defect: fix nothing in this repo, land the engine change, then update the harness and advance its pin in `specify-adapters`. The repositories move at independent paces; the pin (not HEAD) is the harness's supported engine revision.

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

- `cargo make ci` in each repo — the per-repo workspace gate; neither resolves the other repository.
- `specify-adapters`' ordinary workspace gate — adapter crate tests and adapter-local component conformance, with no Specify dependency in its graph.
- `specify-adapters`' dedicated `native-harness` job — the standalone `harness/native` workspace against its declared engine pin (the only job holding the read-only `SPECIFY_READ_TOKEN`).
- Specify's composed job — the model-free workflow-core scenario with echo adapters and no sibling checkout.
- Neither repo gates on the other's HEAD: compatibility is owned by the adapters repo's pin, advanced deliberately.
- Live-model profiles stay operator-triggered (rungs 2 and 3); CI never requires model credentials.
