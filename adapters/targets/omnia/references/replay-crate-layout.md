# Replay crate layout

Generated Omnia crates follow a consistent layout for runtime fixture replay. The build test-writer phase assumes this structure when locating handlers, tests, and fixture data.

## Directory structure

```text
$CRATE_DIR/
├── src/
│   ├── lib.rs           # Public API, re-exports
│   ├── handler.rs       # Handler impls (or split by domain)
│   ├── error.rs         # Domain errors (if any)
│   └── ...
├── tests/
│   ├── provider.rs      # MockProvider implementing the crate's provider bounds
│   ├── <handler_or_feature>.rs   # One or more integration test modules
│   └── data/
│       └── replay/      # Replay test fixtures (JSON)
│           ├── INSTRUCTIONS.md  # (optional) per-handler test-generation hints
│           ├── samples/         # (optional) shared bulk data
│           │   └── *.json
│           └── <handler>/
│               └── <scenario>.json
├── Cargo.toml
├── Migration.md         # Manual steps and notes
└── Architecture.md      # Component design (if generated)
```

When the slice has a `runtime-fixtures` source binding, copy or symlink the bound fixture tree into `$CRATE_DIR/tests/data/replay/` preserving the handler/scenario layout from [`runtime-fixtures/references/fixture-format.md`](../../../sources/runtime-fixtures/references/fixture-format.md).

## Key paths

| Path | Purpose |
|------|--------|
| `$CRATE_DIR/src/` | Production code; handlers implement `Handler<P>` with Omnia provider bounds. |
| `$CRATE_DIR/tests/` | Integration tests; each `.rs` file is a separate test binary. |
| `$CRATE_DIR/tests/provider.rs` | Shared MockProvider used by all test modules. |
| `$CRATE_DIR/tests/data/replay/` | Replay fixture JSON. Loaded via `include_bytes!("data/replay/<handler>/<scenario>.json")` or by path. |
| `$CRATE_DIR/tests/data/replay/INSTRUCTIONS.md` | Optional per-handler hints — see [`replay-fixtures.md`](replay-fixtures.md). |
| `$CRATE_DIR/tests/data/replay/samples/` | Shared bulk data via `@samples/` references. Not fixtures. |

## How fixtures are used in tests

- **StateStore-backed handlers**: load JSON with `include_bytes!("data/replay/samples/fleet-data.json")` and inject via `MockProvider::with_state("key", data)` or `MockProvider::seed_cache("key", data)`.
- **HttpRequest-backed handlers**: `include_bytes!("data/replay/<handler>/<endpoint>.json")`; dispatch in the mock by `request.uri().path()`.
- **TableStore-backed handlers**: bulk entity data from `samples/` passed to MockProvider constructor; fixtures specify query parameters and expected results.
- **TestDef-style fixtures**: JSON has `setup`, `input`, `params`, `http_requests`, `output`; tests deserialize and run one scenario per file.
- **Setup block**: configure MockProvider per [`replay-fixtures.md`](replay-fixtures.md).

When adding tests from new fixtures, follow the same pattern already used in the crate's existing test modules.

## See also

- [`replay-fixtures.md`](replay-fixtures.md) — `setup` block, `INSTRUCTIONS.md`, MockProvider mapping
- [`examples/replay/`](examples/replay/) — worked handler, test, and fixture examples
- [`../briefs/build/test.md`](../briefs/build/test.md) — test writer phase
