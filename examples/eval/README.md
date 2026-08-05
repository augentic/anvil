# Eval composition

The native composition example over the mock catalog: command passthrough by
default, and the live eval case runner under the `eval` subcommand. Both
modes share [`probe::client`](../../crates/probe/src/client.rs) — Tokio,
argv, `mock::catalog()`, and the `cases/` root are all this example owns.

This is the operator surface for the judgment-prompt loop:

```text
edit crates/{slice,change}/prompts/  →  cargo make eval auth --restart  →  read grades / repairs  →  repeat
```

Outputs are graded by deterministic validators, not a model. Case mechanics
and grading posture live in [`crates/probe/README.md`](../../crates/probe/README.md).

## Live cases (the prompt loop)

Requires an authenticated agent CLI on `PATH` for whichever backend
`EMERY_MODEL_BACKEND` selects: `cursor` (default) wants `cursor-agent login`
or `CURSOR_API_KEY` in `.env` at the repository root; `claude` wants
`claude login` or `ANTHROPIC_API_KEY`. Only the selected backend connects.

```bash
cargo make eval                              # list the cases
cargo make eval auth --restart               # the auth workflow case, end to end
cargo make eval auth --restart --until plan  # stop after plan author to inspect the plan
```

Case data lives in [`cases/`](cases/) (`cases/<id>/case.toml`). Each case
keeps one stable retained sandbox at the repository-root `sandbox/<id>/`
(composition-owned; beside the wasm example's `sandbox/wasm/`), on
success and failure alike; `--restart` is the only runner-owned reset,
and an existing sandbox without it refuses before mutation. Continue or
debug a retained sandbox explicitly:

```bash
cargo make lab -- --project-dir sandbox/auth plan execute
```

The reserved `--debug` / `--quiet` host log flags (peeled before
dispatch, a flag wins over `RUST_LOG`; flagless defaults to `info`) and
the `EMERY_MODEL_*`, `CURSOR_*`, `CLAUDE_*`, `RUST_LOG`, and `EVAL_LOG`
env knobs are documented in
[`crates/probe/README.md`](../../crates/probe/README.md);
`cargo make eval` defaults the cursor timeout to 300s.

Cadence: before a release tag, and after any change to the judgment prompts
(`crates/slice/prompts/`, `crates/change/prompts/`) or the generated answer
schemas. Never CI — see [the developer loop](../../docs/contributing/dev-loop.md).

## Command passthrough

Any emery verb against the mock catalog, in-process (no Wasm):

```bash
cargo make lab -- --project-dir <dir> slice list
cargo run --example eval -- --help
```

The native host executes trusted adapter code in-process. It does not provide
component isolation, dynamic component loading, adapter-store lookup, or digest
verification — use the wasm example (`cargo make wasm-run`) when those
properties are required.
