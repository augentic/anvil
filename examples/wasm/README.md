# Wasm Example

End-to-end run of the Emery change workflow over the real WASM component seam: the shipped `emery` binary (embedded engine guest) plus the mock source and target adapter components.

The two adapter components ([source.rs](source.rs), [target.rs](target.rs)) are each one SDK export-macro invocation over the canonical `mock::Adapter` implementor — the same anatomy as a production adapter in `augentic/emery-adapters`. There is no `omnia.toml`: the example invokes the built binary directly. The run script sandboxes the layout with `EMERY_HOME`, seeds the source via `emery adapter add` (plan `--source` accepts bare names only), and admits the target as a local `.wasm` at `emery init`.

## Quick start

Login to the Cursor agent:

```bash
agent login
```

or set `CURSOR_API_KEY` in `.env`.

Run the example:

```bash
cargo make wasm-run
```

Each run wipes `sandbox/wasm/` then rebuilds only when Cargo says so. To remove leftover artifacts without re-running:

```bash
cargo make wasm-clean
```

Artifacts land under the gitignored `sandbox/wasm/` — the project tree at `sandbox/wasm/project/`, with the store and cache beside it.

`GUEST_TIMEOUT_MS` defaults to one hour (Omnia's per-invocation wall-clock cap; default is 30s). Set `RUST_LOG` yourself when debugging the seam. The runtime may log a non-fatal `no guest exports the http handler; http trigger inert` line per invocation — command mode proceeds without it.

## What it demonstrates

1. Every command runs in the embedded engine guest; bound adapters fault in by routed id through the fail-closed resolver (`target:target`, `source:source`).
2. The source adapter surveys and extracts greeting requirements.
3. Emery reconciles them and drives refine → build → merge.
4. The target adapter builds and merges the result.

After running, inspect the generated result at:

```text
sandbox/wasm/project/mock-build/<slice>.md
```
