# Example Adapter

This is an end-to-end example of the Specify application. It uses Specify's core engine + source and target adapters to implement a rudimentary workflow.

Both source and target adapters are implemented by the same wasm guest. The guest implements an MCP server for the model agent to use to request reference documents from the adapter.

## What it demonstrates

The example runs a Specify ***change*** using a mock model:

1. The source adapter ***surveys*** and ***extracts*** greeting requirements.
2. Specify ***reconciles*** them using deterministic scripted answers.
3. The target adapter ***builds*** and ***merges*** the result.

After running, inspect the generated result at:

```text
examples/greeting/workspace/fixture-build/greeting.md
```

## Run

Run the example (and clean up afterwards):

```bash
# Run the example (create plan -> execute)
make run-example

# Clean up afterwards
make clean-example
```

