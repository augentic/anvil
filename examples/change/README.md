# Change Example

This is an end-to-end example of the Specify application. It uses Specify's core engine + source and target adapters to implement a rudimentary workflow.

The two adapter components ([source.rs](source.rs), [target.rs](target.rs)) are each one SDK export-macro invocation over the canonical `mock::Adapter` implementor — the exact anatomy of a production adapter in `augentic/specify-adapters`. Each component also serves its embedded reference documents over MCP (the macro wires that in). The workflow guest itself is the engine's `specify` cdylib: one `guest::export!()` over the `guest` crate.

## Quick start

Login to the Cursor agent:

```bash
agent login
```

or set `CURSOR_API_KEY` in `.env`.

Run the example:

```bash
make change-run
```

Clean up afterwards:

```bash
make change-clean
```

Artifacts land under the gitignored `sandbox/change/`.

## What it demonstrates

The example runs the Specify ***change*** workflow. It will `author->approve->execute` a ***plan*** using a mock model:

1. The source adapter ***surveys*** and ***extracts*** greeting requirements.
2. Specify ***reconciles*** them using deterministic scripted answers.
3. The target adapter ***builds*** and ***merges*** the result.

After running, inspect the generated result at:

```text
sandbox/change/workspace/mock-build/<slice>.md
```

