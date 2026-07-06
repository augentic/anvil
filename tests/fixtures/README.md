## WASI Tool Fixtures

`tools-test-project/` holds the deterministic declared-tool WASI components used by the `specify lint project` tool-path test in `tests/lint.rs` (`echo` today; the `read-*` permission probes are kept alongside it for future permission coverage).
The `.wasm` files are checked in so developer machines and CI do not need to rebuild
WASI components before running `cargo test --workspace`.

To rebuild the blobs, install the target, then run:

```bash
rustup target add wasm32-wasip2
scripts/regen-wasm-fixtures.sh
```

The Rust source crate lives at `tools-test-project/src-rust/`.
