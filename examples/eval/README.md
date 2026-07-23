# Eval composition

The native composition example over the mock catalog: command passthrough by
default, and the live prompt-evaluation trial under the `eval` subcommand.
Both modes share [`probe::client`](../../crates/probe/src/client.rs) — Tokio,
argv, and `mock::catalog()` are all this example owns.

This is the operator surface for the judgment-prompt loop:

```text
edit crates/{slice,change}/prompts/  →  cargo make eval  →  read grades / repairs  →  repeat
```

Outputs are graded by deterministic validators, not a model. Trial mechanics
and grading posture live in [`crates/probe/README.md`](../../crates/probe/README.md).

## Live trial (the prompt loop)

Requires authenticated `cursor-agent` on `PATH` (`cursor-agent login` or
`CURSOR_API_KEY` in `.env` at the repository root).

```bash
cargo make eval           # full trial over sandbox/
cargo make eval init      # one phase: init | plan | execute | finalize | clean
```

A passing full run cleans `sandbox/`; a failing phase retains it for in-place
review or per-phase re-runs. `SPECIFY_EVAL_MODEL` and
`SPECIFY_EVAL_TIMEOUT_SECS` are documented in
[`crates/probe/README.md`](../../crates/probe/README.md); `cargo make eval`
defaults the timeout to 300s.

Cadence: before a release tag, and after any change to the judgment prompts
(`crates/slice/prompts/`, `crates/change/prompts/`) or the generated answer
schemas. Never CI — see [the developer loop](../../docs/contributing/dev-loop.md).

## Command passthrough

Any specify verb against the mock catalog, in-process (no Wasm):

```bash
cargo make specify -- --project-dir <dir> slice list
cargo run --example eval -- --help
```

The native host executes trusted adapter code in-process. It does not provide
component isolation, dynamic component loading, adapter-store lookup, or digest
verification — use the wasm example (`cargo make wasm-run`) when those
properties are required.
