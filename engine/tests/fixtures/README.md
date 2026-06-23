## WASI Tool Fixtures

`tools-test-project/` and `tools-test-adp/` hold deterministic declared-tool contract acceptance fixtures.
`adapters/targets/dispatch-fixture/` holds the adapter-agnostic host-dispatch fixture (`adapter.wasm`
beside `adapter.yaml`) used by `catalog infer`, `slice build` prepare, and `extension schema` tests.
The `.wasm` files are checked in so developer machines and CI do not need to rebuild
WASI components before running `cargo test --workspace`.

To rebuild the blobs, install the target plus `wasm-tools`, then run:

```bash
rustup target add wasm32-wasip2
cargo install wasm-tools
scripts/regen-wasm-fixtures.sh
```

The Rust source crate lives at `tools-test-project/src-rust/`. `exit-seven.wasm`
is generated from `tools-test-adp/src-wat/exit-seven.component.wat` because the
stable WASI 0.2 Rust bindings expose only success/failure through `std::process`,
while this fixture needs the Preview 2 `exit-with-code` import to assert exit 7.

The checked-in manifests use `file:///__SPECIFY_FIXTURE_ROOT__/...` placeholders
because declared-tool contract requires local tool sources to be absolute. Integration tests copy
the fixtures to a tempdir and rewrite those placeholders to the copied fixture root
before invoking `specify`.
