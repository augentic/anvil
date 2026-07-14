# Example Adapter

This example shows how to implement both Specify source and target adapters and use them to implement Specify's end-to-end workflow.

The adapters implement a single MCP server for the model agent to use when requesting adapter reference documents.

## Run

Run the example (and clean up afterwards):

```bash
# Run the example (create plan -> execute)
make run-example

# Optionally, clean up
make clean-example
```

Optionally, build the example adapter(s) and specify.wasm before running the example:

```bash
# Build the example adapter(s) and specify.wasm
cargo build --example greeting-wasm --target wasm32-wasip2
cargo build --target wasm32-wasip2
```