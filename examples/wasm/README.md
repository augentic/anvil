# Wasm Example

This is an end-to-end example of the Specify application. It uses Specify's core engine + source and target adapters to implement a rudimentary workflow.

The two adapter components ([source.rs](source.rs), [target.rs](target.rs)) are each one SDK export-macro invocation over the canonical `mock::Adapter` implementor — the exact anatomy of a production adapter in `augentic/specify-adapters`. Each component also serves its embedded reference documents over MCP (the macro wires that in). The engine guest itself is the engine's `specify` cdylib: one `guest::export!()` over the `guest` crate.

There is no `omnia.toml` and no `specify run --config`: the example invokes the built `specify` binary directly, and the binary's launcher (RFC-70) derives, hydrates, and digest-verifies the component closure per invocation, then assembles the deployment in memory. The run script sandboxes the whole artifact layout with one `SPECIFY_HOME` override (store and cache derive together beneath it), seeds the locally-built engine guest into the store (entry plus `.meta` digest sidecar — the same shape registry hydration leaves), and seeds the adapters — the mock target as an operator-supplied local component at init, and the mock source into the project component cache via `specify adapter add` — exactly the states a real install would reach through registry hydration, `specify init`, and `specify adapter add`.

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

The runtime logs one non-fatal `trigger http ... but no routes` error per invocation: derived `/mcp/<name>` route rows are RFC-70 Stage 2 scope, so the HTTP trigger has no routes yet and command mode proceeds without it.

## What it demonstrates

The example runs the Specify ***change*** workflow. It will `author->approve->execute` a ***plan*** using a mock model:

1. The launcher computes each invocation's closure (engine + `target:mock` + `source:mock-source`), verifies it fail-closed, and boots the deployment — `plan author --source …` surveys a source the launcher enumerated pre-run, the RFC-70 ensure-then-dispatch invariant.
2. The source adapter ***surveys*** and ***extracts*** greeting requirements.
3. Specify ***reconciles*** them using deterministic scripted answers.
4. The target adapter ***builds*** and ***merges*** the result.

After running, inspect the generated result at:

```text
sandbox/wasm/project/mock-build/<slice>.md
```

