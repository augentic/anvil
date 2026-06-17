# `omnia` target — `with-replay` fixture

Worked example for the **optional** replay hook on the [`omnia` target adapter's `build/replay.md`](../../../../../adapters/targets/omnia/briefs/build/replay.md) sub-brief (the capture-backed replay workflow, target half). Shared hook contract: [`replay/hook-contract.md`](../../../../../adapters/shared/target-hooks/replay/hook-contract.md). The `metadata.yaml` in this directory carries a top-level `replay:` block alongside the standard slice metadata.

## What this fixture demonstrates

A slice whose `plan.yaml.sources[]` carries a `captures` binding routes through the Omnia build replay phase. When the target implements the hook it replays the captured scenarios (`cd $CRATE_PATH && cargo nextest run --tests` against `tests/data/replays/<handler>/<scenario>.json`) and persists the outcome two ways: a `replay:` block on `metadata.yaml` (this fixture) plus a `slice.replay.completed` journal event.

The block shape is documented in [`replay/journal-payload.md`](../../../../../adapters/shared/target-hooks/replay/journal-payload.md):

```yaml
replay:
  passed: <int>
  failed: <int>
  skipped: <int>
  ran-at: <ISO-8601 UTC>
  runner: <e.g. "omnia-target@1.4 (cargo nextest)">
```

`/spec:merge` reads the block when present and surfaces a one-line `replay: <passed> passed, <failed> failed, <skipped> skipped` summary in its closing message. `merge` does NOT auto-refuse on `failed > 0` in v1 — the operator decides whether to land the slice; the block is advisory. See [`hook-contract.md`](../../../../../adapters/shared/target-hooks/replay/hook-contract.md) § Merge posture.

## Diff posture against `../without-replay/`

The two fixtures are byte-identical apart from the trailing `replay:` block. A `diff -u ../without-replay/metadata.yaml metadata.yaml` should show only the appended block, no other field deltas — the replay hook is additive and never reshapes the rest of `metadata.yaml`.

## Validation

`metadata.yaml` follows the `SliceMetadata` shape in [`crates/workflow/src/slice/metadata.rs`](https://github.com/augentic/specify/blob/main/cli/crates/workflow/src/slice/metadata.rs) (kebab-case field names; closed `status:` and `outcome.phase:` enums; ISO-8601 UTC timestamps). The optional `replay:` block is currently additive at the document level (`SliceMetadata` allows unknown fields); a closed sub-schema lands when the field is promoted out of the v1 "optional" posture.
