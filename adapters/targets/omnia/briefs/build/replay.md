# Omnia build — capture replay

Loaded by [../build.md](../build.md) phase 7 when the slice's `plan.yaml.sources[]` list carries a `captures` binding.

## Shared contract

Read [`../../../../shared/target-hooks/replay/hook-contract.md`](../../../../shared/target-hooks/replay/hook-contract.md) first — skip rules, generic preconditions, advisory posture, journal recording, merge summary, and the ban on hand-editing `metadata.yaml`. Payload shapes: [`../../../../shared/target-hooks/replay/journal-payload.md`](../../../../shared/target-hooks/replay/journal-payload.md).

## Omnia preconditions

In addition to the shared contract:

- Phases 2–6 complete: crate, tests, guest (create mode), verify-repair loop, and code review have run.
- Replay data is present under `$CRATE_PATH/tests/data/replays/` — copied or symlinked during [test writer](test.md) when a `captures` binding exists.

Capture wire format: [`captures/references/capture-format.md`](../../../../sources/captures/references/capture-format.md). Claim shape and 64 KiB inline cap: [`captures/briefs/extract.md`](../../../../sources/captures/briefs/extract.md).

## Omnia execution

1. **Confirm replay tree.** List `$CRATE_PATH/tests/data/replays/<handler>/*.json`. Every scenario file the `captures` adapter extracted should have a corresponding integration test from phase 3; if gaps exist, re-enter [test.md](test.md) before replay.

2. **Run the replay suite.**

   ```bash
   cd $CRATE_PATH && cargo nextest run --tests
   ```

   Fall back to `cargo test` when nextest is unavailable. The operator's `captures` binding may point at a different root than the crate copy — replay always runs against `$CRATE_PATH/tests/data/replays/`.

3. **Classify results** per the shared contract (advisory in v1).

4. **Record the journal event** per [`journal-payload.md`](../../../../shared/target-hooks/replay/journal-payload.md) with `runner: omnia-target@1 (cargo nextest)` (adjust version suffix to match the resolved Omnia target adapter version when known).

## References

- [`../../../../shared/target-hooks/replay/README.md`](../../../../shared/target-hooks/replay/README.md) — shared hook index and target adoption table
- [`../../references/replay-crate-layout.md`](../../references/replay-crate-layout.md) — crate paths and fixture loading
- [`../../references/replay-fixtures.md`](../../references/replay-fixtures.md) — `setup` block and MockProvider mapping
- [`../../references/examples/replay/`](../../references/examples/replay/) — worked migration examples
- [`test.md`](test.md) — generates replay integration tests in phase 3
