# Generated Crate Layout

Crates produced through the Specify workflow follow a consistent layout. The **replay-writer** skill assumes this layout when locating handlers, tests, and fixture data.

## Directory Structure

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
│       └── replay/      # Replay test fixtures (JSON) — user places real-life data here
│           ├── INSTRUCTIONS.md  # (optional) special instructions for test generation
│           ├── samples/         # (optional) shared bulk data files referenced by fixtures
│           │   └── *.json
│           └── *.json
├── Cargo.toml
├── Migration.md         # Manual steps and notes
└── Architecture.md     # Component design (if generated)
```

## Key Paths

| Path | Purpose |
|------|--------|
| `$CRATE_DIR/src/` | Production code; handlers implement `Handler<P>` with provider bounds from QWASR/Omnia. |
| `$CRATE_DIR/tests/` | Integration tests; each `.rs` file is a separate test binary. |
| `$CRATE_DIR/tests/provider.rs` | Shared MockProvider used by all test modules. |
| `$CRATE_DIR/tests/data/replay/` | Replay fixture JSON files. Loaded via `include_bytes!("data/replay/<name>.json")` or by path. |
| `$CRATE_DIR/tests/data/replay/INSTRUCTIONS.md` | (Optional) Freeform instructions for how to load sample data, construct the MockProvider, and assert on results. |
| `$CRATE_DIR/tests/data/replay/samples/` | (Optional) Shared bulk data files referenced by fixtures via `@samples/` paths or loaded per instructions. Not fixtures — the skill does not generate tests from these. |

## How Fixtures Are Used

- **StateStore-backed handlers**: Tests often load a single JSON file (e.g. `fleet-data.json`) with `include_bytes!("data/replay/samples/fleet-data.json")` and inject it via `MockProvider::with_state("key", data)` or `MockProvider::seed_cache("key", data)`.
- **HttpRequest-backed handlers**: Tests use `include_bytes!("data/replay/<endpoint>.json")` and dispatch in the mock by `request.uri().path()` to return that payload.
- **TableStore-backed handlers**: Bulk entity data is loaded from `samples/` and passed to the MockProvider constructor. Fixtures specify query parameters and expected results.
- **TestDef-style fixtures**: JSON has `setup`, `input`, `params`, `http_requests`, `output`; tests deserialize and run one scenario per file.
- **Setup block**: When a fixture has a `setup` field, it configures the MockProvider for that specific test (bulk data, cache seeding, config overrides). Values prefixed with `@samples/` reference files in the `samples/` directory. See [fixture-format.md § Setup block](fixture-format.md).

When adding tests from new fixtures in `tests/data/replay/`, follow the same pattern already used in the crate's existing test modules (same provider setup, same loading mechanism).

## Instructions and Sample Data

When `INSTRUCTIONS.md` exists in the replay directory, read it **before** inspecting fixtures. It provides domain-specific guidance that overrides default assumptions about how to construct providers and load data.

When a `samples/` subdirectory exists, its files are shared bulk datasets used across multiple fixtures. This avoids duplicating large data arrays in every fixture file. Common examples:

- `samples/fleet-data.json` — raw entity data for TableStore-backed handlers
- `samples/cached-fleet.json` — pre-parsed data for cache-hit test paths
- `samples/api-response.json` — shared HTTP response body for HttpRequest-backed handlers
