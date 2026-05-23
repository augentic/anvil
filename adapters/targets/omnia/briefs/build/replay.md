# Omnia build — fixture replay

Loaded by [../build.md](../build.md) phase 7 when the slice's `plan.yaml.sources[]` list carries a `code-runtime` binding. **OPTIONAL in v1** — skip entirely when no `code-runtime` source is bound; omission is not an error.

## Preconditions

- Phases 2–6 complete: crate, tests, guest (create mode), verify-repair loop, and code review have run.
- The slice's Evidence includes `kind: example` claims from the `code-runtime` extract pass (or the bound fixture tree is available at the plan-level source path).
- Replay fixtures are present under `$CRATE_PATH/tests/data/replay/` — copied or symlinked during [test writer](test.md) when a `code-runtime` binding exists.

Fixture wire format: [`code-runtime/references/fixture-format.md`](../../../../sources/code-runtime/references/fixture-format.md). Claim shape and 64 KiB inline cap: [`code-runtime/briefs/extract.md`](../../../../sources/code-runtime/briefs/extract.md).

## Execution

1. **Confirm fixture tree.** List `$CRATE_PATH/tests/data/replay/<handler>/*.json`. Every scenario file the `code-runtime` adapter extracted should have a corresponding integration test from phase 3; if gaps exist, re-enter [test.md](test.md) before replay.

2. **Run the replay suite.**

   ```bash
   cd $CRATE_PATH && cargo nextest run --tests
   ```

   Fall back to `cargo test` when nextest is unavailable. The operator's `code-runtime` binding may point at a different root than the crate copy — replay always runs against `$CRATE_PATH/tests/data/replay/`.

3. **Classify results.** Count passed, failed, and skipped tests. Replay failures are **advisory in v1** — they do not park the build. The slice still transitions to `built`; the operator inspects results at merge time.

4. **Record the journal event.** Emit `slice.fixture-replay.completed` (`EventKind::SliceFixtureReplayCompleted`) via `specify slice journal append` with payload `{ passed, failed, skipped, runner }`. Example runner string: `omnia-target@1 (cargo nextest)`.

5. **Do not hand-edit `.metadata.yaml`.** RFC-25 retired `specify slice outcome set`. A future CLI surface may persist a `fixture-replay:` block to `$SLICE_DIR/.metadata.yaml` (RFC-27 §D1); until that lands, the journal event is the supported v1 recorder:

   ```yaml
   # aspirational — CLI-owned in a future release
   fixture-replay:
     passed: <int>
     failed: <int>
     skipped: <int>
     ran-at: <ISO-8601 UTC>
     runner: <e.g. "omnia-target@1 (cargo nextest)">
   ```

## Merge posture

When a `fixture-replay:` block is present (future CLI or operator tooling), `merge` surfaces a one-line summary (`fixture-replay: <passed> passed, <failed> failed, <skipped> skipped`). `merge` does **not** auto-refuse on `failed > 0` in v1 — the operator decides whether to land a slice whose generated code does not pass captured fixtures, mirroring RFC-25 posture on `[conflict]` and `[divergence]` tags. Stricter posture: custom target adapter fork or CI policy reading journal events / `.metadata.yaml` directly.

## References

- [`../../references/replay-crate-layout.md`](../../references/replay-crate-layout.md) — crate paths and fixture loading
- [`../../references/replay-fixtures.md`](../../references/replay-fixtures.md) — `setup` block and MockProvider mapping
- [`../../references/examples/replay/`](../../references/examples/replay/) — worked migration examples
- [`test.md`](test.md) — generates replay integration tests in phase 3
- [`../../../../../plugins/spec/references/phase-outcome-contract.md`](../../../../../plugins/spec/references/phase-outcome-contract.md) — retired outcome verbs
