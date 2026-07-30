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

`GUEST_TIMEOUT_MS` defaults to one hour (Omnia's per-invocation wall-clock cap; default is 30s). Set `RUST_LOG` yourself when debugging the seam.

The runtime's HTTP trigger serves adapter MCP reference shelves on a per-invocation port: the launcher pre-binds an ephemeral loopback listener when `HTTP_ADDR` is unset (an operator-set value must bind or startup fails), so concurrent `emery` invocations never contend. The runtime injects the listener's local address as the guest-visible `HTTP_ADDR`; every judgment dispatch grants the spawned agent `http://127.0.0.1:<port>/mcp/<axis>/<name>[@<version>]` derived from it, and the deployment's `http_router` maps that path back onto the routed adapter id so the component's own `wasi:http` handler serves it (the native eval rung hosts the same shelves in-process at `/mcp/<name>`). A path outside that grammar is an ordinary 404; a claimed route whose guest cannot be served — including one without the handler export — is an error-logged 500, never a dispatch to the engine guest.

## What it demonstrates

1. Every command runs in the embedded engine guest; bound adapters fault in by routed id through the fail-closed resolver (`target:target`, `source:source`).
2. The source adapter surveys and extracts greeting requirements.
3. Emery reconciles them and drives refine → build → merge.
4. The target adapter builds and merges the result.

After running, inspect the generated result at:

```text
sandbox/wasm/project/mock-build/<slice>.md
```
