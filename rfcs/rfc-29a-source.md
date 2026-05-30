# RFC-29a: Executable Source Operations

> Status: Draft — Milestone **M1** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-25](../done/rfc-25-workflow.md), [RFC-27](../done/rfc-27-synthesis.md), [RFC-35](done/rfc-35-synthesis-determinism.md) — Unblocks: RM-05 durable proof; the M2a inputs ([RFC-29b](rfc-29b-reconciliation.md))

This is the first independently shippable milestone of [RFC-29](rfc-29-fan-in-fan-out.md). It makes source-adapter `survey` and `extract` CLI-owned operations, adds the closed adapter `execution` mode, and adds the guarded journal emitter. It lands without depending on lead reconciliation, synthesis, or the build envelope: `specrun source survey` / `extract` are useful the day they ship — they make `/spec:refine` extraction CLI-owned and give acceptance a durable seam.

The cross-milestone wire contracts this milestone appends to (the closed `EventKind` taxonomy and the `Error` discriminant set) are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This document is the source of truth for D1, the source side of D9, and D12.

## Decisions owned by this milestone

| ID | Decision |
| -- | -------- |
| **D1 Source operation runner** | The CLI runs source adapter `survey` and `extract` operations: `specrun source survey` / `specrun source extract`, routed through `SourceAdapter::resolve`, declared tools, sandbox preopens, extraction cache, schema validation, and journal events. |
| **D9 Adapter execution mode** (source side) | Source adapters declare a closed `execution: executable \| agent-fallback` field selecting deterministic dispatch vs an agent-run brief. (The symmetric target side lands in [RFC-29d](rfc-29d-target-build-envelope.md).) |
| **D12 Journal emitter** | `specrun journal emit` is the schema-validated writer for agent-orchestrated phases with no deterministic emit command. |

## Operator surface

The new lower-level breakouts this milestone adds:

```bash
specrun source survey docs --format json
specrun source extract docs password-reset --slice identity-password-reset --format json
specrun journal emit slice.synthesize.agent --payload '{"slice":"identity-password-reset"}'   # D12 agent-orchestrated emitter
```

## Source operation runner (D1)

### Commands

Add two commands under the existing `specify source` family:

```bash
specrun source survey <source-key> [--plan <name>] [--format json]
specrun source extract <source-key> <lead-id> --slice <slice> [--format json]
```

`<source-key>` resolves against `plan.yaml.sources.<key>`, not against adapter name. The command then resolves the adapter from `SourceBinding.adapter`.

### `survey`

`survey` runs the source adapter's `briefs.survey` operation under the source-adapter sandbox:


| Root              | Mode       | Contents                                                              |
| ----------------- | ---------- | --------------------------------------------------------------------- |
| `$SOURCE_DIR`     | read-only  | Bound source path when the source uses `path:`.                       |
| `$CAPABILITY_DIR` | read-only  | Resolved source adapter manifest cache.                               |
| `$SCRATCH_DIR`    | write-only | Per-operation scratch under `.specify/.cache/extractions/<adapter>/`. |
| `$PROJECT_DIR`    | none       | Not visible to the adapter operation.                                 |


For value-bound sources such as `intent`, `$SOURCE_DIR` is absent and the value is passed through the build request envelope.

Output is a lead set, validated against `schemas/discovery/lead.schema.json`, then merged into `discovery.md` by CLI-owned discovery helpers. Re-running `survey` for the same source replaces leads by canonical `id`, preserves operator aliases, and keeps deterministic ordering.

### `extract`

`extract` runs the source adapter's `briefs.extract` operation for one `(source-key, lead-id)` pair and writes:

```text
.specify/slices/<slice>/evidence/<source-key>.yaml
```

The CLI validates the Evidence document against `schemas/evidence.schema.json` before the write becomes visible to later synthesis. Failure leaves the slice in `refining`.

### Cache and journal

Both operations use the RFC-27 cache fingerprint model:

```text
source identity + adapter name@version + brief sha256 + sorted tool versions + lead id?
```

`lead id` is absent for `survey` and present for `extract`.

Survey cache events (`source.survey.cache-hit`, `source.survey.cache-miss`) are new; extract cache events already exist in RFC-27. Full event taxonomy: [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts).

### Relationship to `specrun source preview`

The existing `specrun source preview` (`src/runtime/commands/source/preview.rs`) already resolves a source adapter, validates the `--source` path, scaffolds an `--out/evidence/` subtree, and surfaces brief paths — but it is **workflow-free**: no `.specify/` writes, no cache, no journal events, no `discovery.md` merge, and it does not dispatch the briefs (the agent runs them by hand against the prepared directory). The D1 runner is the **workflow-integrated** counterpart of that same operation. To keep one source-operation contract rather than two that drift:

- **Share the environment prep.** Factor a single internal helper — adapter resolution, brief-directory resolution (the landed RFC-35 D9 `briefs-dir`), the four-root sandbox preopen layout (`$SOURCE_DIR` / `$CAPABILITY_DIR` / `$SCRATCH_DIR` / `$PROJECT_DIR`), and `evidence/` scaffolding — and have **both** `source preview` and `source survey` / `source extract` consume it. The runner is then a thin layer that adds the workflow-integrated behaviour on top of the shared prep.
- **Keep the surfaces distinct in role.** `source preview` stays the workflow-free dry run (adapter authoring / debugging, output under `--out`). `source survey` / `source extract` add the sandboxed `execution`-branched dispatch (D9), the RFC-27 cache fingerprint, the journal events, validate-before-visible, and the `discovery.md` merge (`survey`) / Evidence persist (`extract`). Equivalently, `preview` may be implemented as the `--dry-run --out <dir>` mode of the same runner so the dispatch code is literally shared.
- **Align the "which lead(s)" surface.** `source preview` already takes `--lead <id>…`; `source extract` takes a positional `<lead-id>` plus `--slice`. The shared helper should use one spelling for lead selection across the family so the `preview` → `extract` path reads consistently.

The genuinely-new machinery D1 introduces over today's `preview` — the sandbox preopens, the `executable` vs `agent-fallback` dispatch branch, the cache, the journal events, validate-before-visible, and the discovery/Evidence persistence — is what makes `survey` / `extract` workflow commands rather than a scaffolding helper; none of it should be re-implemented in `preview`.

## Adapter execution mode (D9)

Source and target adapters declare a closed `execution` field on their respective `adapter.yaml`. This milestone lands the source side and the shared schema additions; the target side (`build` / `merge` dispatch) lands in [RFC-29d](rfc-29d-target-build-envelope.md).

```yaml
# adapters/sources/<name>/adapter.yaml
execution: executable     # or `agent-fallback`
```

The two values are:

- `**executable**` — `survey` and `extract` (sources) or `build` and `merge` (targets) are dispatched through a declared WASI tool or a deterministic Rust adapter path. Inputs and outputs validate against the schemas committed in the RFC-29 family.
- `**agent-fallback**` — the adapter's brief is executed by an agent against the same sandbox preopens. The CLI orchestrates inputs and validates outputs against the same schemas, but does not cache the result.

When `execution: agent-fallback`, the CLI:

1. emits a `source.execution.agent-fallback` (sources) or `target.execution.agent-fallback` (targets) journal event on every operation invocation;
2. forces `cache: opt-out` regardless of the adapter's declared cache mode (rejected at parse time as `adapter-execution-agent-fallback-cache-conflict` if the manifest declares any other cache mode);
3. surfaces a `suggestion`-severity `adapter-execution-agent-fallback` finding on the framework standards layer for first-party adapters, and not at all for third-party adapters.

The schema additions are mechanical extensions of `schemas/source.schema.json` and `schemas/target.schema.json`:

```json
{
  "execution": {
    "type": "string",
    "enum": ["executable", "agent-fallback"],
    "description": "Closed adapter execution mode per RFC-29 D9."
  }
}
```

with `execution` added to the `required` list on both schemas. The loader rejects a manifest that omits `execution` rather than defaulting silently.

## Journal emitter (D12)

Deterministic commands emit their own events; agent-orchestrated steps (D2/D9/D10 agent paths, agent-driven build/merge) use the guarded emitter below. Why RFC-35 rejected this verb and why RFC-29 adds it: [RFC-29 §"Relationship to RFC-35"](rfc-29-fan-in-fan-out.md#relationship-to-rfc-35).

RFC-29 introduces:

```bash
specrun journal emit <event-id> [--payload <json>] [--format json]
```

The emitter is deliberately thin and closed:

- `<event-id>` must be a member of the closed `EventKind` taxonomy in `crates/workflow/src/journal.rs`; an unknown id is rejected with `journal-emit-unknown-event` (exit 2).
- `--payload` is validated against the per-kind payload shape before the line is appended; a payload that fails its kind's required fields is rejected with `journal-emit-payload-schema` (exit 2).
- The CLI stamps the `timestamp` (second-precision UTC) and appends one well-formed line to `.specify/journal.jsonl`. The agent never composes the envelope, the timestamp, or the wire id by hand.

This keeps a **single emission path and a single closed taxonomy**: deterministic commands and the agent-facing verb both write the same `Event` shape through the same writer, so there is no second NDJSON format to drift. The emitter adds no new event kinds of its own — it is purely a guarded front door onto the kinds defined in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts).

## Wire contracts introduced by this milestone

The canonical closed tables live in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This milestone appends:

- **Journal events:** `source.survey.cache-hit`, `source.survey.cache-miss`, `source.execution.agent-fallback`, plus the `specrun journal emit` front door (D12) onto the whole taxonomy.
- **Operational validation codes (`Error::Validation`, not new enum variants):** `adapter-execution-mode-required`, `adapter-execution-agent-fallback-cache-conflict`, `journal-emit-unknown-event`, `journal-emit-payload-schema` — single-signal aborts at adapter load / `journal emit`, exit 2. See [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts) for the error-tiering model.
- **Schema edits:** the `execution` enum added (and `required`) on `schemas/source.schema.json` and `schemas/target.schema.json`. The loader rejects a manifest that omits `execution` with `adapter-execution-mode-required` rather than defaulting silently.
