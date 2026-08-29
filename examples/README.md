# Mock Source Example

Static host for the mock source adapter: `specify` extracts a deterministic greeting claim, synthesises from canned answers, and commits a generation.

## Prerequisites

- [cursor-sdk-bridge](https://github.com/cursor/sdk-bridge). See [below](#installing-cursor-sdk-bridge) for installation.
- `CURSOR_API_KEY`

## Build and run

```bash
# build the mock source component
cargo build --example source --target wasm32-wasip2 --release

# run the host (argv after `--` is the emery guest's)
export CURSOR_API_KEY=<Cursor API key>
export RUST_LOG=info,omnia_cursor=debug,cursor_wasm=debug,opentelemetry_sdk=off
cargo run --example runtime -- specify source

# review the committed spec
cargo run --example runtime -- show spec
```

The `--` is cargo's separator; the guest receives `specify source` directly. There is no host `run` subcommand.

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