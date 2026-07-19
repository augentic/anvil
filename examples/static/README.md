# Native example

This example runs the Specify command router in one native process with the mock source and target adapters linked into the binary. It uses [`linked::Catalog`](../../crates/linked/src/catalog.rs) for static adapter composition and [`linked::command::run`](../../crates/linked/src/command.rs) for the CLI boundary; no Wasm components are built or loaded.

## Quick start

Requires authenticated `cursor-agent` on `PATH` (`cursor-agent login` or `CURSOR_API_KEY`).

```bash
cargo run --example native -- --help
cargo build --release --example native
```

The release binary is written to `target/release/examples/native`. Run it from the project directory it should operate on.

The linked host executes trusted adapter code in-process. It does not provide component isolation, dynamic component loading, adapter-store lookup, or digest verification; use the Wasm deployment when those properties are required.
