# Mock Source Example

Live `specify` journey via [omnia-cursor](https://github.com/augentic/omnia-backends/tree/main/crates/cursor): the mock adapter extracts greeting claims from `[docs/](docs/)` through the host model, the engine synthesises `spec.md` / `design.md`, and the generation commits.

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

`wasi-model` implements guest-defined tools using two streams rather than direct callbacks. The host sends `ToolCall` values to the guest through the session’s `calls` stream, while the guest returns corresponding `ToolResult` values through a second stream it creates.

To open a completion session, the guest creates the result stream, retains its writable end, and passes the readable end to `create`. The host returns a `reply` future and the `calls` stream. While awaiting the reply, the guest handles each tool call and writes a result with the same correlation ID, allowing the host to resume the completion.

The mount (`--mount`, or `[[mount]]` in `config.toml`) preopens `examples/cursor/workspace` as the tree named `.`; the guest lends it through `grants.workspace` and the cursor backend resolves it to the working tree the agent runs in.

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