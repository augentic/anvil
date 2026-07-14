# Greeting

A deterministic, manifest-hosted Specify workflow over the fixture adapter. The example deploys the workflow guest and one combined source/target component through `omnia.toml`; its `runtime!` host supplies scripted reconciliation and synthesis answers.

## Run

From the repository root:

```bash
# Build the greeting adapter(s) and specify.wasm
cargo build --example greeting-wasm --target wasm32-wasip2
cargo build --target wasm32-wasip2
```

Stage the adapters:

```bash
# create the cache and store directories
set -euo pipefail

EXAMPLE="examples/greeting"
export RUST_LOG="info,opentelemetry_sdk=off,omnia_wasi_http=debug"

mkdir -p "$EXAMPLE"/{workspace,cache,store}
cp target/wasm32-wasip2/debug/examples/greeting_wasm.wasm \
  "$EXAMPLE/workspace/fixture.wasm"

run() {
  cargo run --quiet -p examples --example greeting -- \
    run --config "$EXAMPLE/omnia.toml" -- "$@"
}

# Initialize the workspace
run init ./fixture.wasm --name greeting

# Author, transition, and execute the plan
run plan author greeting --source main="fixture:value:The greeting service."
run plan transition greeting approved
run plan execute
```

Clean up:

```bash
rm -rf examples/greeting/workspace examples/greeting/cache examples/greeting/store
```
