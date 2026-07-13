# Greeting

A deterministic, manifest-hosted Specify workflow over the fixture adapter. The example deploys the workflow guest and one combined source/target component through `omnia.toml`; its `runtime!` host supplies scripted reconciliation and synthesis answers.

## Run

From the repository root:

```bash
cargo make test-wasm
```

The task builds `specify.wasm` and `greeting_wasm.wasm`, stages this directory's `omnia.toml` unchanged in a temporary deployment, then drives:

```text
init → plan author → approve → execute
```

The smoke checks adapter resolution on both axes, typed failure lifting, model-host invocation, writable preopens, component-cache writes, and the greeting build output.
