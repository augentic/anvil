# `omnia` target — `with-fixture-replay` fixture

Worked example for the **optional** fixture-replay hook on the [`omnia` target adapter's `build` brief](../../../../../adapters/targets/omnia/briefs/build.md) (RFC-27 §D1, target half). The `.metadata.yaml` in this directory carries a top-level `fixture-replay:` block alongside the standard slice metadata.

## What this fixture demonstrates

A slice whose `plan.yaml.sources[]` carries a `code-runtime` binding routes through the build brief's § Fixture replay step. When the target implements the hook it replays the captured fixtures (`cd $CRATE_PATH && cargo nextest run --tests` against `tests/data/replay/<handler>/<scenario>.json`) and persists the outcome two ways: a `fixture-replay:` block on `.metadata.yaml` (this fixture) plus a `slice.fixture-replay.completed` journal event.

The block shape is the closed five-field shape from the build brief:

```yaml
fixture-replay:
  passed: <int>
  failed: <int>
  skipped: <int>
  ran-at: <ISO-8601 UTC>
  runner: <e.g. "omnia-target@1.4 (cargo nextest)">
```

`/spec:merge` reads the block when present and surfaces a one-line `fixture-replay: <passed> passed, <failed> failed, <skipped> skipped` summary in its closing message. `merge` does NOT auto-refuse on `failed > 0` in v1 — the operator decides whether to land the slice; the block is advisory.

## Diff posture against `../without-fixture-replay/`

The two fixtures are byte-identical apart from the trailing `fixture-replay:` block. A `diff -u ../without-fixture-replay/.metadata.yaml .metadata.yaml` should show only the appended block, no other field deltas — the fixture-replay hook is additive and never reshapes the rest of `.metadata.yaml`.

## Validation

`.metadata.yaml` follows the `SliceMetadata` shape in [`crates/domain/src/slice/metadata.rs`](https://github.com/augentic/specify-cli/blob/main/crates/domain/src/slice/metadata.rs) (kebab-case field names; closed `status:` and `outcome.phase:` enums; ISO-8601 UTC timestamps). The optional `fixture-replay:` block is currently additive at the document level (`SliceMetadata` allows unknown fields); a closed sub-schema lands when the field is promoted out of the v1 "optional" posture.
