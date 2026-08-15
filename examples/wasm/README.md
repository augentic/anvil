# Wasm Example

End-to-end run of the Emery change workflow over the real WASM component seam: the shipped `emery` binary (embedded engine guest) plus the mock source and target adapter components.

The two adapter components ([source.rs](source.rs), [target.rs](target.rs)) are each one SDK export-macro invocation over the canonical `mock::Adapter` implementor — the same anatomy as a production adapter in `augentic/emery-adapters`. There is no `omnia.toml`: the example invokes the built binary directly. The run script sandboxes the layout with `EMERY_HOME`, seeds the source via `emery adapter add`, and admits the target as a local `.wasm` at `emery init`. Authoring uses `--from` / `--wave` against a fixture definition home.

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

Set `RUST_LOG` yourself when debugging the seam (the run keeps the env-driven filter because it wants `omnia_wasi_http=debug` without `omnia_cursor=debug`); for ad-hoc `emery` invocations the reserved host flags `--debug` / `--quiet` are the ergonomic route — they win over any ambient `RUST_LOG`.

The runtime's HTTP trigger serves adapter MCP reference shelves on a per-invocation port: the launcher pre-binds an ephemeral loopback listener when `HTTP_ADDR` is unset (any bind failure is a startup failure), so concurrent `emery` invocations never contend. The runtime injects the listener's local address as the guest-visible `HTTP_ADDR`; every judgment dispatch grants the spawned agent `http://127.0.0.1:<port>/mcp/<axis>/<name>[@<version>]` derived from it, and the deployment's `http_paths` hook maps that path back onto the routed adapter id so the component's own `wasi:http` handler serves it (the native eval rung hosts the same shelves in-process at `/mcp/<name>`). A path outside that grammar — or a claimed identity nothing supplies — is an ordinary 404; a genuine fault on a claimed route, including a guest without the handler export, is an error-logged 500, never a dispatch to the engine guest.

## What it demonstrates

1. Every command runs in the embedded engine guest; bound adapters fault in by routed id through the fail-closed resolver (`target:target`, `source:source`).
2. The source adapter surveys and extracts greeting requirements.
3. Emery reconciles them and drives refine → build → merge.
4. The target adapter builds and merges the result.

After running, inspect the generated result at:

```text
sandbox/wasm/project/mock-build/<slice>.md
```

## Publication scenario (RFC-95)

```bash
cargo make wasm-publication
```

Same seam, but the delivery target binds to a local bare Git origin (`sandbox/wasm/origin.git`) through a minted reviewed definition home ([definition.rs](definition.rs), `cargo run --example definition`). The execute drain's last merge satisfies the D11 predicate and materializes the publication worktree over real host Git at `sandbox/wasm/publication/change/app/`; the script then runs the operator Git loop — commit with both `Emery-Change` trailers, push `change/change` to the origin — and prints the `emery plan status` publication milestone. Archive verification reads the forge, and GitHub is the only v1 forge, so the scenario ends at the pushed branch rather than `emery plan archive`.
