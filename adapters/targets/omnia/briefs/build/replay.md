# Omnia build — fixture replay

Loaded by [../build.md](../build.md) phase 7 when the slice's `plan.yaml.sources[]` list carries a `code-runtime` binding.

## Shared contract

Read [`../../../../shared/target-hooks/fixture-replay/hook-contract.md`](../../../../shared/target-hooks/fixture-replay/hook-contract.md) first — skip rules, generic preconditions, advisory posture, journal recording, merge summary, and the ban on hand-editing `.metadata.yaml`. Payload shapes: [`../../../../shared/target-hooks/fixture-replay/journal-payload.md`](../../../../shared/target-hooks/fixture-replay/journal-payload.md).

## Omnia preconditions

In addition to the shared contract:

- Phases 2–6 complete: crate, tests, guest (create mode), verify-repair loop, and code review have run.
- Replay fixtures are present under `$CRATE_PATH/tests/data/replay/` — copied or symlinked during [test writer](test.md) when a `code-runtime` binding exists.

Fixture wire format: [`code-runtime/references/fixture-format.md`](../../../../sources/code-runtime/references/fixture-format.md). Claim shape and 64 KiB inline cap: [`code-runtime/briefs/extract.md`](../../../../sources/code-runtime/briefs/extract.md).

## Omnia execution

1. **Confirm fixture tree.** List `$CRATE_PATH/tests/data/replay/<handler>/*.json`. Every scenario file the `code-runtime` adapter extracted should have a corresponding integration test from phase 3; if gaps exist, re-enter [test.md](test.md) before replay.

2. **Run the replay suite.**

   ```bash
   cd $CRATE_PATH && cargo nextest run --tests
   ```

   Fall back to `cargo test` when nextest is unavailable. The operator's `code-runtime` binding may point at a different root than the crate copy — replay always runs against `$CRATE_PATH/tests/data/replay/`.

3. **Classify results** per the shared contract (advisory in v1).

4. **Record the journal event** per [`journal-payload.md`](../../../../shared/target-hooks/fixture-replay/journal-payload.md) with `runner: omnia-target@1 (cargo nextest)` (adjust version suffix to match the resolved Omnia target adapter version when known).

## References

- [`../../../../shared/target-hooks/fixture-replay/README.md`](../../../../shared/target-hooks/fixture-replay/README.md) — shared hook index and target adoption table
- [`../../references/replay-crate-layout.md`](../../references/replay-crate-layout.md) — crate paths and fixture loading
- [`../../references/replay-fixtures.md`](../../references/replay-fixtures.md) — `setup` block and MockProvider mapping
- [`../../references/examples/replay/`](../../references/examples/replay/) — worked migration examples
- [`test.md`](test.md) — generates replay integration tests in phase 3
