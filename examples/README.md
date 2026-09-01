# Mock Source Example

Live `specify` journey via [omnia-cursor](https://github.com/augentic/omnia-backends/tree/main/crates/cursor): the mock source crate extracts greeting claims from its fixture tree through the host model, the engine synthesises `spec.md` / `design.md`, and the generation commits.

The adapter lives at `[source/](source/)` — the same anatomy as a first-party adapter (`src/`, `prose/`, `build.rs`, native `tests/`). The `[runtime](runtime.rs)` host stays a root-package example because it embeds the engine guest. The bound input is `[source/fixture/](source/fixture/)`, not the embedded prose corpus.

## Prerequisites

- [cursor-sdk-bridge](https://github.com/cursor/sdk-bridge). See [below](#installing-cursor-sdk-bridge) for installation.
- `CURSOR_API_KEY`



## Build and run

```bash
# build the source adapter
cargo build -p adapter --target wasm32-wasip2 --release

# run the example
export CURSOR_API_KEY=<Cursor API key>
cargo run --example runtime -- --debug specify --config examples/emery.toml

# review the committed spec
cargo run --example runtime -- --debug show spec
```

The config binds the built mock component by path (`[emery.toml](emery.toml)`) and lends `[source/fixture/](source/fixture/)` as `$SOURCE_DIR`. A bare name still only dispatches guests declared in the runtime invocation, and this host declares none.

Extract and synthesis both complete through the Cursor backend. The mock guest answers reference-tool calls in-process the same way the [omnia-cursor example](https://github.com/augentic/omnia-backends/tree/main/examples/cursor) answers `lifecycle`.

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