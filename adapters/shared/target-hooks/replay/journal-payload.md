# Replay payload shapes

Closed wire shapes for the build-time `replay` hook. Recording rules and merge posture live in [`hook-contract.md`](hook-contract.md).

## Journal event (v1 recorder)

Emit via `specify journal emit slice.replay.completed --payload <json>`:

| Field | Value |
|---|---|
| Event kind | `slice.replay.completed` |
| Rust enum | `EventKind::SliceReplayCompleted` |
| Payload keys | `passed`, `failed`, `skipped`, `runner` |

Example NDJSON line (illustrative):

```json
{"event":"slice.replay.completed","passed":47,"failed":0,"skipped":2,"runner":"omnia-target@1 (cargo nextest)"}
```

- **`passed`** / **`failed`** / **`skipped`** — non-negative integers from the target runner's test classification.
- **`runner`** — identifies the target adapter version and command (e.g. `omnia-target@1 (cargo nextest)`, `contracts-target@1 (specify extension run contract)`).

Taxonomy reference: [specify-cli `DECISIONS.md` — journal events](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md).

## Aspirational `metadata.yaml` block (future CLI)

the capture-backed replay workflow defines an additive block targets MAY write to `$SLICE_DIR/metadata.yaml` once a CLI-owned surface lands. **Agents must not hand-edit this block in v1** — journal-only recording until then.

```yaml
replay:
  passed: <int>
  failed: <int>
  skipped: <int>
  ran-at: <ISO-8601 UTC>
  runner: <e.g. "omnia-target@1 (cargo nextest)">
```

Worked examples:

- With block: [`evals/fixtures/targets/omnia/with-replay/`](../../../../evals/fixtures/targets/omnia/with-replay/)
- Without block (omission-is-not-an-error): [`evals/fixtures/targets/omnia/without-replay/`](../../../../evals/fixtures/targets/omnia/without-replay/)

The block is additive; it must not reshape other `metadata.yaml` fields. `/spec:merge` reads it when present for the one-line closing summary described in [`hook-contract.md`](hook-contract.md).

## See also

- [`hook-contract.md`](hook-contract.md) — when to emit, advisory posture, merge rules
- [`../../../targets/omnia/briefs/build/replay.md`](../../../targets/omnia/briefs/build/replay.md) — Omnia runner that produces these payloads
