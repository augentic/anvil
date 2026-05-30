# RFC-29a: Executable Source Operations

> Status: Draft — Milestone **M1** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-25](../done/rfc-25-workflow.md), [RFC-27](../done/rfc-27-synthesis.md), [RFC-35](done/rfc-35-synthesis-determinism.md) — Unblocks: RM-05 durable proof; the M2a inputs ([RFC-29b](rfc-29b-reconciliation.md))

This is the first independently shippable milestone of [RFC-29](rfc-29-fan-in-fan-out.md). It makes source-adapter `survey` and `extract` CLI-owned operations, adds the closed adapter `execution` mode, and adds the guarded journal emitter. It lands `specrun source survey` / `extract`, which are useful the day they ship — they make `/spec:refine` extraction CLI-owned and give acceptance a durable seam.

The cross-milestone wire contracts this milestone appends to (the closed `EventKind` taxonomy and the `Error` discriminant set) are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This document is the source of truth for D1, the source side of D9, and D12.

## Decisions owned by this milestone


| ID                                     | Decision                                                                                                                                                                                                                                               |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **D1 Source operation**                | The CLI runs source adapter `survey` and `extract` operations: `specrun source survey` / `specrun source extract`, routed through `SourceAdapter::resolve`, declared tools, sandbox preopens, extraction cache, schema validation, and journal events. |
| **D9 Adapter execution** (source side) | Source adapters declare a closed `execution: tool                                                                                                                                                                                                      |
| **D12 Journal emitter**                | `specrun journal emit` is the schema-validated writer for agent-orchestrated phases with no deterministic emit command.                                                                                                                                |


## Operator surface

The new lower-level breakouts this milestone adds (`documentation` source adapter example):

```bash
specrun source survey documentation --format json
specrun source extract documentation password-reset --slice identity-password-reset --format json
specrun journal emit slice.synthesize.agent --payload '{"slice":"identity-password-reset"}'
```

## Source operation runner (D1)

### Commands

Add two commands under the existing `specify source` family:

```bash
specrun source survey <source-key> [--plan <name>] [--format json]
specrun source extract <source-key> <lead-id> --slice <slice> [--format json]
```

`<source-key>` resolves against `plan.yaml.sources.<key>`, not against adapter name. The command then resolves the adapter from `SourceBinding.adapter`.

Both commands locate adapter brief bodies through the `briefs-dir` field on `specrun source resolve --format json` (RFC-35 D9). That field is **not present in the current `ResolveBody`** despite the RFC-35 plan marking it done, so M1 lands it as its first commit: add `briefs-dir` (the absolute path to the resolved adapter's `briefs/` directory) to the resolve JSON envelope in `src/runtime/commands.rs`. The addition is wire-safe — existing parsers ignore unknown fields — so it carries no migration cost.

### `survey`

`survey` runs the source adapter's `briefs.survey` operation under the source-adapter sandbox:


| Root              | Mode       | Contents                                                              |
| ----------------- | ---------- | --------------------------------------------------------------------- |
| `$SOURCE_DIR`     | read-only  | Bound source path when the source uses `path:`.                       |
| `$CAPABILITY_DIR` | read-only  | Resolved source adapter manifest cache.                               |
| `$SCRATCH_DIR`    | write-only | Per-operation scratch under the extraction tree (see below).          |
| `$PROJECT_DIR`    | none       | Not visible to the adapter operation.                                 |


`$SCRATCH_DIR` nests under the existing per-adapter extraction tree (`.specify/.cache/extractions/<adapter>/`), disjoint from the fingerprint result cache so a scratch write never pollutes a cache artifact. Because `survey` runs at plan time and carries no slice, the scratch path is keyed per operation: `extract` uses `.specify/.cache/extractions/<adapter>/<slice>/scratch/`; `survey` uses `.specify/.cache/extractions/<adapter>/survey/scratch/`. This supersedes the looser `<adapter>/` wording carried in earlier drafts and aligns with the per-slice shape in [docs/explanation/adapter-anatomy.md](../docs/explanation/adapter-anatomy.md).

For value-bound sources such as `intent`, `$SOURCE_DIR` is absent and the value is passed through a minimal source request envelope: `path:` bindings carry `source-path`, value bindings carry `value-inline: <string>`. The cache layer already distinguishes the two through `FingerprintSource::{Path, Value}` (the value variant keys on the sha256 of the literal body), so no new cache machinery is required. M1 does **not** adopt the full RFC-29d build-request schema — a two-field source envelope is sufficient and keeps `intent` (the degenerate N=1 entry point) working.

Output is a lead set, validated against `schemas/discovery/lead.schema.json`, then merged into `discovery.md` by a CLI-owned merge helper. Re-running `survey` for the same source replaces leads by canonical `id`, preserves operator-authored `aliases[]` on surviving ids, and keeps deterministic ordering. Implement this as `Discovery::merge_survey(source_key, leads)` on the existing model in `crates/model/src/discovery/document.rs` (which already owns `write_atomic`, `add_alias`, `remove_alias`, and `check_alias_collisions`): it removes prior blocks whose `id` is in the incoming set for that source, re-renders atomically, then fails the whole merge on any `check_alias_collisions` hit so no partial state lands on disk. Cover it with tests modelled on `tests/discovery_aliases.rs` §re-survey survival.

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

The genuinely-new machinery D1 introduces over today's `preview` — the sandbox preopens, the `tool` vs `agent` dispatch branch, the cache, the journal events, validate-before-visible, and the discovery/Evidence persistence — is what makes `survey` / `extract` workflow commands rather than a scaffolding helper; none of it should be re-implemented in `preview`.

## Skill integration

The `/spec:refine` and `/spec:plan` skills move onto the new verbs once the runner lands, and **only then** — the skills must never reference a verb that does not yet exist. `refine/SKILL.md` step 3 replaces the hand-invoked `extract` brief with a `specrun source extract <source-key> <lead-id> --slice <slice>` call; the plan skill's survey path calls `specrun source survey <source-key>`. Under `execution: agent` the change is small because the verb is the two-phase prepare/finalize handoff described under D9: the skill runs the brief against the prepared sandbox and the CLI owns validate, persist, and journal. These edits are the **last** step of the milestone, sequenced after the CLI runner is proven, matching how RFC-35 ordered skill edits after CLI edits.

## Adapter execution mode (D9)

Source and target adapters declare a closed `execution` field on their respective `adapter.yaml`. This milestone lands the source side and the shared schema additions; the target side (`build` / `merge` dispatch) lands in [RFC-29d](rfc-29d-target-build-envelope.md).

```yaml
# adapters/sources/<name>/adapter.yaml
execution: tool     # or `agent`
```

The two values are:

- `**agent**` — the adapter's brief is executed by an agent against the same sandbox preopens. The CLI orchestrates inputs and validates outputs against the same schemas, but does not cache the result.
- `**tool**` — `survey` and `extract` (sources) or `build` and `merge` (targets) are dispatched through a declared WASI tool or a built-in deterministic Rust adapter path. Inputs and outputs validate against the schemas committed in the RFC-29 family.

When `execution: agent`, the CLI:

1. emits a `source.execution.agent` (sources) or `target.execution.agent` (targets) journal event on every operation invocation;
2. forces `cache: opt-out` regardless of the adapter's declared cache mode (rejected at parse time as `adapter-execution-agent-cache-conflict` if the manifest declares any other cache mode);
3. surfaces a `suggestion`-severity `adapter-execution-agent` finding on the framework standards layer for first-party adapters, and not at all for third-party adapters.

### First-party adapters in M1

All five first-party source adapters (`intent`, `documentation`, `code-typescript`, `screenshots`, `captures`) ship `execution: agent` in M1 — none has a WASI tool today, so `agent` is the truthful value. The `tool` dispatch branch is wired and schema-valid but unexercised by first-party manifests until a source gains a real deterministic tool. Because `execution: agent` forces `cache: opt-out` (rule 2 above), audit each adapter's current cache posture before stamping: a source that relies on cache hits today becomes a guaranteed cache miss under `agent`. That trade-off is intentional and must be called out in the implementation plan rather than discovered at runtime.

### Agent dispatch is two-phase

The `tool` path is **single-phase**: the CLI dispatches the WASI tool and captures its output synchronously within one process. The `agent` path is **two-phase**, reusing the `specrun source preview` scaffolding (equivalently, the `--dry-run --out <dir>` mode of the shared runner):

1. **Prepare.** The CLI resolves the adapter, builds the four-root sandbox layout, scaffolds the output target (`evidence/` for `extract`, the lead-set target for `survey`), emits the `source.execution.agent` journal event, and prints a handoff envelope on stdout: `{ adapter, version, briefs-dir, source-dir, scratch-dir, evidence-dir, leads[], execution: "agent" }`. Control returns to the agent.
2. **Finalize.** The agent runs the brief against the prepared directory and writes outputs to the declared paths; a follow-up CLI call runs validate-before-visible, the `discovery.md` merge (`survey`) / Evidence persist (`extract`), and the cache write.

The CLI never blocks waiting on agent work — the prepare/finalize split is what makes the agent path orchestratable.

The schema additions are mechanical extensions of `schemas/source.schema.json` and `schemas/target.schema.json`:

```json
{
  "execution": {
    "type": "string",
    "enum": ["tool", "agent"],
    "description": "Closed adapter execution mode per RFC-29 D9."
  }
}
```

with `execution` added to the `required` list on both schemas. The loader rejects a manifest that omits `execution` rather than defaulting silently.

Making `execution` required is a coordinated two-repo migration that lands in one PR, even though target dispatch is M3: (1) add the `execution` enum to both schemas without `required`; (2) stamp `execution: agent` on all eight first-party `adapter.yaml` files (five sources plus the three targets `omnia` / `vectis` / `contracts`, which take `agent` as a placeholder until M3 wires real target dispatch); (3) flip `required` on; (4) add the `adapter-execution-mode-required` loader rejection and the `adapter-execution-agent-cache-conflict` parse check; (5) run the cross-repo `rg` sweep the `crates/workflow/src/adapter/` touch rule mandates (the workflow contract spans both `augentic/specify` and `augentic/specify-cli`). 2.0 is a hard cut with no compatibility aliases, so there is no migration shim — but every example `adapter.yaml` body in the docs of both repos needs the new field too, or `specdev lint` / `specrun lint` will flag drift.

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

The "per-kind payload shape" is the closed `EventKind` enum itself — there is no parallel JSON-schema registry. The three M1 events become first-class typed variants alongside the existing `SliceExtractCacheHit` template, e.g. `SourceSurveyCacheHit { source-key, adapter, fingerprint }`, `SourceSurveyCacheMiss { source-key, adapter, fingerprint, reason }`, and `SourceExecutionAgent { source-key, adapter, operation }`. The `journal emit` guard is then a single serde round-trip: deserialize `<event-id>` + `--payload` into `EventKind`; an unknown tag yields `journal-emit-unknown-event`, a required-field failure yields `journal-emit-payload-schema`. This keeps "one closed taxonomy, one writer" with no second validation mechanism. The `reason` field reuses the closed `CacheMissReason` enum; confirm it carries the `adapter-opt-out` variant the cache-miss path references and add it if absent.

This keeps a **single emission path and a single closed taxonomy**: deterministic commands and the agent-facing verb both write the same `Event` shape through the same writer, so there is no second NDJSON format to drift. The emitter adds no new event kinds of its own — it is purely a guarded front door onto the kinds defined in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts).

## Wire contracts introduced by this milestone

The canonical closed tables live in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This milestone appends:

- **Journal events:** `source.survey.cache-hit`, `source.survey.cache-miss`, `source.execution.agent`, plus the `specrun journal emit` front door (D12) onto the whole taxonomy.
- **Operational validation codes (`Error::Validation`, not new enum variants):** `adapter-execution-mode-required`, `adapter-execution-agent-cache-conflict`, `journal-emit-unknown-event`, `journal-emit-payload-schema` — single-signal aborts at adapter load / `journal emit`, exit 2. See [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts) for the error-tiering model.
- **Schema edits:** the `execution` enum added (and `required`) on `schemas/source.schema.json` and `schemas/target.schema.json`. The loader rejects a manifest that omits `execution` with `adapter-execution-mode-required` rather than defaulting silently.

## Implementation sequence

M1 lands as a stepped sequence, mirroring the RFC-35 implementation-plan format. Each step is independently reviewable; CLI work precedes the skill edits that depend on it.

| Step | Work                                                                                                                          | Repo(s)     |
| ---- | --------------------------------------------------------------------------------------------------------------------------- | ----------- |
| 0    | Preflight inventory; pin `$SCRATCH_DIR` shape, the agent prepare/finalize handoff, and the M1 journal payload variants       | both        |
| 1    | Add `briefs-dir` to `source` / `target resolve` JSON output                                                                  | specify-cli |
| 2    | `execution` schema enum + loader rejection + `agent`/cache conflict check + stamp all eight first-party manifests            | both        |
| 3    | M1 typed `EventKind` variants + `specrun journal emit` serde-guard                                                           | specify-cli |
| 4    | Shared sandbox-prep helper; refactor `source preview` onto it                                                                | specify-cli |
| 5    | `specrun source survey` + `Discovery::merge_survey`                                                                          | specify-cli |
| 6    | `specrun source extract` + Evidence persist + value-binding envelope                                                         | specify-cli |
| 7    | Skill updates (`refine` step 3, `plan` survey path)                                                                          | specify     |
| 8    | Acceptance scenario `5j` (source-adapter sandbox path-denied)                                                                | both        |

