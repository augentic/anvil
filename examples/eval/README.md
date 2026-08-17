# Eval composition

The native composition example over the mock catalog, shaped like the
`emery` CLI plus the case runner: emery verbs pass through to the native
command surface, while `eval` (or a leading case id) routes to the live
case runner. Both modes share
[`probe::client`](../../crates/probe/src/client.rs) — Tokio, argv,
`mock::catalog()`, and the `cases/` root are all this example owns.

This is the operator surface for the judgment-prompt loop:

```text
edit crates/{slice,change}/prompts/  →  cargo make eval auth --restart  →  read grades / repairs  →  repeat
```

Outputs are graded by deterministic validators, not a model. Case mechanics
and grading posture live in [`crates/probe/README.md`](../../crates/probe/README.md).

## Live cases (the prompt loop)

Requires authenticated `cursor-agent` on `PATH` (`cursor-agent login` or
`CURSOR_API_KEY` in `.env` at the repository root).

```bash
cargo make eval                              # list the cases
cargo make eval auth                         # the auth workflow case, end to end
cargo make eval auth --until plan            # stop after plan author to inspect the plan
cargo make eval auth --restart               # replace the sandbox, rerun from fresh state
```

Case data lives in [`cases/`](cases/) (`cases/<id>/case.toml`). Each case
keeps one stable retained sandbox at the repository-root `sandbox/<id>/`
(composition-owned; beside the wasm example's `sandbox/wasm/`), on
success and failure alike. A failed or stopped workflow run is continued
— graded — by running the same command again: a sandbox holding an
authored plan resumes at `plan refine`; a bound-not-authored sandbox
(no reconcile fact) re-runs `plan author`, which resumes its open and
parked domains. `--restart` is the only runner-owned reset; build
sandboxes and unbound workflow sandboxes refuse without it. Inspect a
retained sandbox with a bound passthrough verb:

```bash
cargo make eval auth plan status
```

The reserved `--debug` / `--quiet` host log flags (peeled before
dispatch; a flag wins over `RUST_LOG`, flagless invocations default
to `info`) and the `CURSOR_MODEL`,
`CURSOR_TIMEOUT_SECS`, `RUST_LOG`, and `EVAL_LOG` env knobs are
documented in [`crates/probe/README.md`](../../crates/probe/README.md);
`cargo make eval` defaults the timeout to 300s.

Cadence: before a release tag, and after any change to the judgment prompts
(`crates/slice/prompts/`, `crates/change/prompts/`) or the generated answer
schemas. Never CI — see [the developer loop](../../docs/contributing/dev-loop.md).

## Command passthrough

Any emery verb against the mock catalog, in-process (no Wasm). A leading
case id binds the verb to that case's retained sandbox (its own store,
cache, snapshot, and workspace roots); cargo-make needs `--` only when
the first token is a cargo-make flag:

```bash
cargo make eval auth plan status             # bound to sandbox/auth
cargo make eval -- --project-dir <dir> slice list
cargo run --example eval -- --help
```

The native host executes trusted adapter code in-process. It does not provide
component isolation, dynamic component loading, adapter-store lookup, or digest
verification — use the wasm example (`cargo make wasm-run`) when those
properties are required.
