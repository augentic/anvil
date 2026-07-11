# Composed workflow-core harness

This non-shipped package hosts the built `specify.wasm` workflow guest with the model-free echo target fixture. Its smoke test runs `init --scaffold-only` through Omnia command mode and proves the WASM-only boundary: `wasi:cli/run`, both adapter WIT links, dispatch by the `target:echo-target` ID, and writes through the project and `/specify-cache` preopens.

Build the guests before running the host test:

```shell
cd harness
cargo make test-composed
```

A full replayed refine/build/merge loop is not part of this focused profile. `omnia-testkit::model::Replay` targets guest-core tests through `omnia_guest::Model`; composed WASM hosting instead links the existing `omnia_wasi_model::ModelDefault` replay backend through `WasiModelCtx`. The smoke links that backend with an empty fixture store so any accidental model call fails deterministically. Extending this to a full loop requires a request-keyed replay corpus for every judgment leg plus the corresponding project scenario; that behavioral coverage remains in the native harness rather than being duplicated here.
