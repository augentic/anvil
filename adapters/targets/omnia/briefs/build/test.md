# Omnia build — test writer

Loaded by [../build.md](../build.md) phase 3. Reads `specs/<domain>/spec.md` + `design.md` + the existing crate inventory and writes `$CRATE_PATH/tests/`.

Tests are spec-driven, not code-driven — generate the side-effect assertion implied by `design.md` even when the current handler does not yet satisfy it; a failing test is the right signal back to the [crate writer](crate.md) step.

## Authority hierarchy

`specs/<domain>/spec.md` scenarios drive happy-path, error-path, and validation coverage. `design.md` "Business logic" and "Provider trait dependencies" drive side-effect assertions (publishes, state writes, cache updates, transactions, rollback). The mapping is canonical and lives in [`spec-to-test-mapping.md`](../../references/spec-to-test-mapping.md).

Manual tests in the existing suite are **flagged as drift, never silently deleted**. The drift report lists missing tests (in spec, not in suite), extra tests (in suite, not in spec), and assertion-drift cases (test present but assertions stale).

## Test generation process

1. **Load artifacts and references** — `specs/<domain>/spec.md`, `design.md`, [`providers/`](../../references/providers/) for the trait-specific MockProvider patterns, [`mock-provider.md`](../../references/mock-provider.md) for Static + Replay variants, and the slice's `tests/data/` replay data if any.
2. **Inventory crate and tests** — enumerate handlers, provider trait bounds, request / response types, existing `tests/*.rs`, existing replay data.
3. **Map specs to tests** — one deterministic test function per scenario, named `test_<crate>_<scenario_snake_case>`. Trace each test to the stable `REQ-XXX` ID in `specs/<domain>/spec.md` via a doc comment. Mapping rules: [`spec-to-test-mapping.md`](../../references/spec-to-test-mapping.md).
4. **Assert side effects** — enumerate every provider interaction in `design.md` and emit assertions: `assert_eq!(provider.publish_calls(), &[…])`, `assert_eq!(provider.state_writes(…), …)`, cache-aside hit/miss order, transaction commit vs rollback.
5. **Generate `MockProvider`** — implement only the provider traits the handler under test consumes. Static / replay variants per [`mock-provider.md`](../../references/mock-provider.md) and the per-trait deep dives under [`providers/`](../../references/providers/).
6. **Load JSON replay data** — `include_str!("data/<capture>.json")` from `tests/data/`. Preserve any existing data style.
7. **Report drift** — emit drift notes inline (a leading `// DRIFT: ...` comment on tests that needed regeneration) but never delete operator-authored tests.

## Runtime capture replay

When the slice's `plan.yaml.sources[]` includes a `captures` binding:

1. **Copy or symlink** the bound capture tree into `$CRATE_PATH/tests/data/replays/` preserving handler/scenario layout per [`captures/references/capture-format.md`](../../../../sources/captures/references/capture-format.md).
2. **Generate one integration test per scenario** — for each `kind: example` claim in `evidence/<runtime-key>.yaml` (or each `<handler>/<scenario>.json` under the bound tree), add or extend tests that load the capture, apply `setup` per [`replay-fixtures.md`](../../references/replay-fixtures.md), invoke the handler, and assert on `output`. Layout: [`replay-crate-layout.md`](../../references/replay-crate-layout.md).
3. **Trace to Evidence** — doc-comment each replay test with the contributing `id` and `REQ-XXX` where synthesis linked the example claim to a requirement.
4. **Worked examples** — [`examples/replay/`](../../references/examples/replay/) (handler, tests, captures for migration scenarios including time-sensitive `shift_time` patterns).

Capture wire format authority stays at the source adapter — do not duplicate the schema here. Execution of the replay suite belongs to [replay.md](replay.md) (phase 7).

## Worked examples

- [`examples/tests/testing.md`](../../references/examples/tests/testing.md) — core test patterns: layout, MockProvider, test structures, test data.
- [`examples/tests/testing-http.md`](../../references/examples/tests/testing-http.md) — simple HTTP handler testing with Config-only MockProvider.
- [`examples/tests/testing-statestore.md`](../../references/examples/tests/testing-statestore.md) — multi-trait MockProvider with StateStore and cache-aside.
- [`examples/tests/testing-publisher.md`](../../references/examples/tests/testing-publisher.md) — publish, event capture, request-reply, topic checks.
- [`examples/tests/testing-blobstore.md`](../../references/examples/tests/testing-blobstore.md) — Blobstore-backed handlers.
- [`examples/tests/testing-documentstore.md`](../../references/examples/tests/testing-documentstore.md) — DocumentStore-backed handlers.
- [`examples/replay/`](../../references/examples/replay/) — runtime capture replay for migration (time-sensitive handlers, directory-scanning replay runner).

## Output and quality checklist

- Every requirement block in `specs/<domain>/spec.md` has at least one matching test function.
- Every provider interaction in `design.md` has at least one assertion.
- The MockProvider implements exactly the trait set the handlers consume (no extras).
- Replay JSON files referenced by `include_str!` exist under `tests/data/`.
- No `cargo test` invocation here — execution belongs to the parent brief's verify-repair loop.
