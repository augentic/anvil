# JSON Fixture Format for tests/data/replay/

Fixtures used by the **replay-writer** skill live in the crate's `tests/data/replay/` folder.

## TestDef-Style (Single-Scenario) Fixtures

Same schema as the test-writer skill. Use when adding tests that run one input through a handler and assert on success or failure.

The internal structures of the test JSON will depend on the handler being tested. It is expected that each handler will have it's own set of fixtures. The top level schema that contains the specific structures needed for the test is this:

```json
{
    "setup": { ... },
    "input": "<raw value>",
    "params": { "delay": 0 },
    "http_requests": [ { "path": "/api/x", "response": { "body": [...] } } ],
    "output": { "success": [...] } | { "failure": { "BadRequest": { "code": "...", "description": "..." } } }
}
```

- **setup**: Optional; provider configuration for this specific test case. See [Setup block](#setup-block) below.
- **input**: Raw input to the handler (string for query/path, object for JSON body, etc.). The format of this input can be anything that deserializes into the structures needed as input to the test. Typically the input will be the same as the handler's request object.
- **params**: Optional; These parameters can be any format needed by the test (consistent across all tests for a handler). The intention of these generic parameters is to provide for data transformation of the input e.g. normalizing timestamps to today, or adding a delay.
- **http_requests**: Optional; mock HTTP responses keyed by path/method for handlers that use `HttpRequest` internally. Many handlers under test will make outbound HTTP requests to collect extra data for processing. Use this part of the test fixture to describe the mock requests and responses needed.
- **output**: Optional; expected success (array of events/data) or failure (error variant and code/description). This "output" can be any side-effect produced by the handler and is not necessarily the handler's response. For example, a handler might process a message and publish a subsequent message (before returning a response). The "output" in this case would be the published message - the main side effect of the handler function. Some handlers may have more than one side effect so this section of the schema can be expanded to include all of those.

All fields other than **input** are optional. Test fixtures could include scenarios where no processing is required, or business logic concludes that some or all of the intermediate steps (such as data transformation, outbound HTTP requests, etc.) are unecessary or fail.

## Setup Block

The optional `setup` block configures the MockProvider before the handler runs. This is needed when the handler's provider bounds go beyond `HttpRequest` (e.g. `TableStore`, `StateStore`) or when the test requires specific config overrides.

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

- **data**: Bulk data to pre-load into the MockProvider (e.g. raw entities for `TableStore::query`). Can be an inline value or a `@samples/` file reference.
- **seed_cache**: Key-value pairs to pre-populate in `StateStore` before the handler runs. Each value can be inline JSON or a `@samples/` file reference. When present, tests should call `provider.seed_cache(key, data)` (or equivalent) so the handler hits the cache path instead of querying the backing store.
- **config**: Config key overrides merged into the MockProvider's config map. Use for testing alternative capacity multipliers, feature flags, TTL values, etc.
- **state_store**: Alternative name for key-value state pre-population (alias for `seed_cache`).
- **table_store**: Bulk data specifically for `TableStore`-backed handlers (alias for `data` when the handler uses `TableStore`).

All `setup` fields are optional. Fixtures without a `setup` block use the default provider construction from `INSTRUCTIONS.md` or the crate's existing test patterns.

### The `@samples/` file reference

Values prefixed with `@samples/` are resolved as file paths relative to `tests/data/replay/`. For example, `"@samples/fleet-data.json"` loads the file `tests/data/replay/samples/fleet-data.json`.

This keeps fixtures small by referencing shared bulk data rather than embedding it inline. Multiple fixtures can reference the same sample file.

## Instructions File

An optional `INSTRUCTIONS.md` file in `tests/data/replay/` provides freeform guidance to the replay-writer skill. Use it when the standard TestDef fixture format is insufficient or when domain-specific context is needed.

### When to use INSTRUCTIONS.md

- The handler uses provider traits beyond `HttpRequest` (e.g. `TableStore`, `StateStore`) and the MockProvider needs specific construction.
- Sample data must be loaded from files in `samples/` and shared across fixtures.
- Assertions require domain-specific logic (e.g. timestamp normalization, partial matching).
- The MockProvider has multiple construction modes (e.g. cache-hit vs cache-miss paths).

### Example INSTRUCTIONS.md

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

Fixtures may include `setup.config` to override config values. Merge these
into the provider using `MockProvider::with_config(raw_vehicles, config)`.

## Assertions

The handler returns `Vec<VehicleInfo>` directly (no side-effect publishing).
Compare the response body against `output.success` by JSON equality.
```

## Sample Data Directory

Shared data files live in `tests/data/replay/samples/`. These are **not** fixtures — the skill does not generate tests from them. They are bulk datasets loaded by test setup code per instructions or `@samples/` references.

```text
tests/data/replay/
├── INSTRUCTIONS.md          # Special instructions for test generation
├── samples/                 # Shared data (not fixtures)
│   ├── fleet-data.json      # Array of raw vehicle entities
│   └── cached-fleet.json    # Pre-parsed vehicle data for cache-hit tests
├── query-by-operator.json   # Fixture: filter by operator_code
├── cache-hit.json           # Fixture: pre-seeded cache path
└── empty-fleet.json         # Fixture: no vehicles in store
```
