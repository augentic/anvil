# Replay test harness

Omnia target test generation consumes runtime captures copied or symlinked into `$CRATE_PATH/tests/data/replays/`. The wire format authority lives at [`adapters/sources/captures/references/capture-format.md`](../../../sources/captures/references/capture-format.md) — this document covers **test-harness** semantics only.

## Setup block

The optional `setup` block configures the MockProvider before the handler runs. Needed when handler provider bounds go beyond `HttpRequest` (e.g. `TableStore`, `StateStore`) or when the test requires specific config overrides.

```json
{
    "setup": {
        "data": "@samples/fleet-data.json",
        "seed_cache": { "fleet_api:fleet_data": "@samples/cached-fleet.json" },
        "config": { "CAPACITY_OVERWRITE": "{\"bus\": 0.8}" }
    },
    "input": "operator_code=NBUS&vehicle_type=Bus",
    "output": { "success": [...] }
}
```

### Setup fields

- **data** — bulk data to pre-load into the MockProvider (e.g. raw entities for `TableStore::query`). Inline value or `@samples/` file reference.
- **seed_cache** — key-value pairs to pre-populate in `StateStore` before the handler runs. Each value inline JSON or `@samples/` reference. Tests call `provider.seed_cache(key, data)` (or equivalent) so the handler hits the cache path.
- **config** — config key overrides merged into the MockProvider's config map (capacity multipliers, feature flags, TTL values).
- **state_store** — alias for `seed_cache`.
- **table_store** — alias for `data` when the handler uses `TableStore`.

All `setup` fields are optional. Captures without `setup` use default provider construction from `INSTRUCTIONS.md` or the crate's existing test patterns.

### `@samples/` file references

Values prefixed with `@samples/` resolve relative to `tests/data/replays/`. Example: `"@samples/fleet-data.json"` → `tests/data/replays/samples/fleet-data.json`. Keeps captures small by referencing shared bulk data.

## INSTRUCTIONS.md

Optional per-handler `INSTRUCTIONS.md` under `tests/data/replays/<handler>/` provides freeform guidance for test generation. Use when the standard TestDef format is insufficient or domain-specific context is needed.

### When to use

- Handler uses provider traits beyond `HttpRequest` (`TableStore`, `StateStore`) and MockProvider needs specific construction.
- Sample data must load from `samples/` and be shared across captures.
- Assertions require domain-specific logic (timestamp normalisation, partial matching).
- MockProvider has multiple construction modes (cache-hit vs cache-miss paths).

### Example

```markdown
# Replay Test Instructions

## Sample Data

Load `samples/fleet-data.json` as a `Vec<RawVehicle>` and pass it to
`MockProvider::new(raw_vehicles)`. All fixtures share this fleet dataset
unless a fixture's `setup.data` overrides it.

## Provider Setup

- **Cache miss (default)**: `MockProvider::new(raw_vehicles)` — handler will
  query TableStore and populate the cache.
- **Cache hit**: When a fixture has `setup.seed_cache`, call
  `provider.seed_cache("fleet_api:fleet_data", &serialized_vehicles)` before
  running the handler.

## Config Overrides

Captures may include `setup.config` to override config values. Merge these
into the provider using `MockProvider::with_config(raw_vehicles, config)`.

## Assertions

The handler returns `Vec<VehicleInfo>` directly (no side-effect publishing).
Compare the response body against `output.success` by JSON equality.
```

`INSTRUCTIONS.md` is **not** behavioural Evidence — the `captures` extract brief may read it for surface-naming context only.

## TestDef → MockProvider mapping

When generating tests from scenario captures:

1. If `INSTRUCTIONS.md` exists, read it first.
2. If the capture has a `setup` block, configure MockProvider before invoking the handler (`setup.data`, `setup.seed_cache`, `setup.config`; resolve `@samples/` paths).
3. Deserialize `input` and invoke the handler.
4. Assert on `output.success` / `output.failure` and any side effects recorded in the fixture.
5. Apply `params` (e.g. `delay`) for time-sensitive handlers via `shift_time` — see [`examples/replay/tests.md`](examples/replay/tests.md).

## See also

- [`replay-crate-layout.md`](replay-crate-layout.md) — generated-crate paths and fixture loading
- [`mock-provider.md`](mock-provider.md) — Static + Replay MockProvider variants
- [`../briefs/build/test.md`](../briefs/build/test.md) — test writer phase
