# Mock Source Example

Live `specify` journey via [omnia-cursor](https://github.com/augentic/omnia-backends/tree/main/crates/cursor): the mock adapter extracts greeting claims from `[docs/](docs/)` through the host model, the engine synthesises `spec.md` / `design.md`, and the revision commits.

The adapter lives at `[adapter/](adapter/)` — the same anatomy as a first-party adapter. The `[runtime](runtime.rs)` host is a root-package example because it embeds the engine guest. The source input is `[docs/](docs/)`.

## Prerequisites

- [cursor-sdk-bridge](https://github.com/cursor/sdk-bridge). See [below](#installing-cursor-sdk-bridge) for installation.
- `CURSOR_API_KEY`



## Build and run

```bash
# build the source adapter
cargo build --example adapter --target wasm32-wasip2 --release

# run the example
export CURSOR_API_KEY=<Cursor API key>
cargo run --example runtime -- --debug specify --config examples/emery.toml

# review the committed spec
cargo run --example runtime -- --debug show spec
```

The config binds the built mock component by path (`[emery.toml](emery.toml)`) and lends `[docs/](docs/)` as `$SOURCE_DIR`. A bare name still only dispatches guests declared in the runtime invocation, and this host declares none.

*Extract* and *synthesis* both complete through the Cursor backend. The mock guest answers reference-tool calls in-process the same way the [omnia-cursor example](https://github.com/augentic/omnia-backends/tree/main/examples/cursor) does.

See [#host-to-guest-tool-calls](#host-to-guest-tool-calls) for more detail.

## Host-to-guest tool calls

In Emery, the only tools a completion session declares are the reference tools — `list_docs` and `read_doc` — over the adapter's embedded prose corpus. `wasi-model` delivers them as two streams rather than direct callbacks: the host writes each `ToolCall` to the session's `calls` stream, and the guest answers with a `ToolResult` on a second stream it created and passed to `create`, carrying the same correlation ID so the host can resume the completion.

Every answer is served in-process from the embedded corpus (`emery_adapter::references`): `list_docs` returns the embedded document paths, `read_doc` returns one document body by adapter-relative path, and anything else — an unknown tool, malformed arguments, an unembedded path — comes back as a repairable error. No HTTP shelf, no MCP callback, and no access to the source input or the revision store crosses this seam; the model reaches nothing but the adapter's own reference documents.

## Installing cursor-sdk-bridge

```bash
# download and install
curl -fsSL -o /tmp/cursor-sdk-bridge.tar.gz \
  https://github.com/cursor/sdk-bridge/releases/latest/download/cursor-sdk-bridge-standalone-darwin-arm64.tar.gz \
  && tar -xzf /tmp/cursor-sdk-bridge.tar.gz -C /tmp \
  && install /tmp/bin/cursor-sdk-bridge ~/.local/bin/cursor-sdk-bridge

# verify
cursor-sdk-bridge --help
```

See [bridge docs](https://cursor.com/docs/sdk/bridge) for more information.