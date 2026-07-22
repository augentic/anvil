# Wasm Example

This is an end-to-end example of the Specify application. It uses Specify's core engine + source and target adapters to implement a rudimentary workflow.

The two adapter components ([source.rs](source.rs), [target.rs](target.rs)) are each one SDK export-macro invocation over the canonical `mock::Adapter` implementor — the exact anatomy of a production adapter in `augentic/specify-adapters`. Each component also serves its embedded reference documents over MCP (the macro wires that in). The engine guest itself is the engine's `specify` cdylib: one `guest::export!()` over the `guest` crate.

There is no `omnia.toml` and no `specify run --config`: the example invokes the built `specify` binary directly. The binary is one `omnia::runtime!` invocation that **embeds the engine guest as static component bytes** and boots it for every command — help, version, and `adapter add` included. The launcher's deployment policy (RFC-70) contributes only the mounts and a fail-closed, adapters-only `GuestResolver`; each adapter is admitted lazily by exact routed id on first dispatch (`target:mock`, `source:mock-source` — verify-and-load only, never downloaded by the resolver). The run script sandboxes the whole artifact layout with one `SPECIFY_HOME` override (store and cache derive together beneath it) and seeds both mock components into the project component cache via the in-guest `specify adapter add` — exactly the state a real install reaches through `specify adapter add`.

A second variant, `cargo make wasm-static-run`, drives the same workflow through the macro-static example host ([host.rs](host.rs)): the same `omnia::runtime!` shape, but with the mock target additionally registered as a **static guest** (static-wins — its dispatch never consults the resolver) while the mock source still faults in dynamically from the project cache. It demonstrates all three guest admission routes — embedded bytes, path-static, and resolver-dynamic — in one deployment.

## Quick start

Login to the Cursor agent:

```bash
agent login
```

or set `CURSOR_API_KEY` in `.env`.

Run the example:

```bash
make wasm-run
```

Clean up afterwards:

```bash
make wasm-clean
```

Artifacts land under the gitignored `sandbox/wasm/` — the project tree at `sandbox/wasm/project/`, with the store and cache beside it.

The runtime logs one non-fatal `no guest exports the http handler; http trigger inert` line per invocation: MCP route projection is RFC-70 Stage 2 scope, so the HTTP trigger stays inert and command mode proceeds without it.

## What it demonstrates

The example runs the Specify ***change*** workflow. It will `author->approve->execute` a ***plan*** using a mock model:

1. The runtime boots with the embedded engine guest registered statically; every command runs in it. Inside the engine guest, the ensure legs provision each bound adapter into the store/cache mounts, and each adapter dispatch (`target:mock`, `source:mock-source`) resolves lazily through the fail-closed adapters-only resolver — verify-and-load, with no pre-enumerated adapter list.
2. The source adapter ***surveys*** and ***extracts*** greeting requirements.
3. Specify ***reconciles*** them using deterministic scripted answers.
4. The target adapter ***builds*** and ***merges*** the result.

After running, inspect the generated result at:

```text
sandbox/wasm/project/mock-build/<slice>.md
```

