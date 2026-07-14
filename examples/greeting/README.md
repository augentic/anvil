# Example Adapter

This example shows how to implement both Specify source and target adapters and use them to implement Specify's end-to-end workflow.

The adapters implement a single MCP server for the model agent to use when requesting adapter reference documents.

## What it demonstrates

The example runs a complete Specify change without model credentials:

1. The source adapter surveys and extracts greeting requirements.
2. Specify reconciles them using deterministic scripted answers.
3. The target adapter builds and merges the result.

One WebAssembly component exports both adapter interfaces and serves their reference documentation over MCP.

After running, inspect the generated result at:

```text
examples/greeting/workspace/fixture-build/greeting.md
```

## Run

Run the example (and clean up afterwards):

```bash
# Run the example (create plan -> execute)
make run-example

# Optionally, clean up
make clean-example
```
