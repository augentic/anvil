# RFC-22: Migration Ledger and Slice Mapping

> **Status: Superseded (archived).** Change-scoped coordination ownership moved to [RFC-88 Detached Changes](../rfc-88-detached-changes.md). Do not implement this document; historical prior art only.
>
> Depends: [RFC-21](rfc-21-catalogue.md) (source catalogue, also archived) and the source-adapter flow in [`engine/docs/standards/workflow.md`](../../docs/standards/workflow.md).

## Abstract

Add the cumulative cross-change state required to plan, route, and audit migrations that span many changes. Today's framework records per-change history through `.emery/archive/plans/<date>-<name>/`, but the survey and propose steps have no machine-readable answer to "is this source migrated yet?" or "what's the source-to-target pattern of this slice?" — both questions get harder as a multi-repo migration grows.

This RFC adds:

1. **`.emery/migration-log.yaml`** — a cumulative ledger recording, per source key, which target projects each extracted capability landed in, when, and via which change. Written only by `emery slice merge` and `emery plan archive`; read by survey, propose, and the new `emery migration-log show` verb.
2. **A `status` field on `sources.yaml:sources[]`** (closed enum: `pending` / `in-progress` / `migrated` / `abandoned`) - driven by the same ledger writers. Operators can override via an explicit `emery source status` verb, but normal lifecycle transitions are framework-driven.
3. **An optional `mapping` field on each `plan.yaml.slices[]` entry** (`one-to-one` / `many-to-one` / `one-to-many` / `greenfield`) — produced by `/emery:plan`, consumed by audit, validated by `emery plan validate`. Audit-only; the slice loop does not branch on it.

These additions fold cross-change durability into the planning loop without touching the slice loop, the workspace-tier boundary, or the one-slice-one-project invariant. The ledger is a *materialised cache* over archive contents; archives remain the source of truth.

## Motivation

Plan-time cross-source synthesis is in place, and RFC-21 adds the source catalogue and source-clone cache. With both in place, three gaps remain visible at scale:

- **No machine-readable cross-change memory.** Routing uses baseline-spec affinity (the baseline surface projection in `.emery/topology.lock`, read into the propose request as `ProjectRef.surface[]`) — "this target already has overlapping specs" — but the framework cannot answer "is `legacy-billing` done?" without scanning archives. The information is scattered across `.emery/archive/plans/<date>-<name>/`, with no index.
- **Source status is implicit.** RFC-21 deliberately punted on the `status` field for `sources[]` because there were no writers. Operators want to know which legacy sources are pending, in flight, or done — and they want the framework to maintain that signal honestly, not by hand-edit.
- **No first-class consolidation/split metadata on slices.** `sources: [k1, k2]` mechanically expresses consolidation, and a single source appearing in two slices with different projects mechanically expresses splitting, but neither is *labelled*. Audit and review must reverse-engineer the intent. Survey recommends mappings; without a place to record them, the recommendations vanish into operator review.

This RFC adds the smallest set of cross-change durable artifacts that fix all three, with the same single-writer invariants the framework already applies to `plan.yaml` and `registry.yaml`.

## Design

### Principles

1. **The ledger is a materialised view.** `.emery/migration-log.yaml` is an append-mostly cache over archive contents. Re-deriving it from archives must always be possible. The archive remains the source of truth.
2. **The CLI is the single writer.** Only `emery slice merge` and `emery plan archive` write to the ledger. `emery source status` is the *only* operator-facing override for the `status` field; otherwise it tracks the ledger.
3. **Schemas are strict.** `additionalProperties: false`, kebab-case identifiers, deny-unknown-fields, byte-stable serialisation. Same posture as `plan.schema.json` and `Registry::validate_shape`.
4. **`mapping` is audit-only.** The slice loop, propose, and execute do **not** branch on the field. It captures intent for review and regression-detection, not control flow.
5. **Additive throughout.** Every existing `plan.yaml`, `sources.yaml`, and archive layout continues to validate without change.
6. **No cross-platform-repo state.** The ledger is per-platform-repo, mirroring `registry.yaml` and `sources.yaml`.

### `.emery/migration-log.yaml` — the cumulative ledger

A new durable artifact at `.emery/migration-log.yaml`. Schema at `schemas/migration-log/schema.json`.

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/augentic/emery/main/schemas/migration-log/schema.json
version: 1
entries:
  - source: legacy-billing
    target_project: billing-svc
    capabilities: [dunning, invoicing]
    change: migrate-billing-2026-q2
    slice: extract-billing-core
    merged_at: 2026-04-15
  - source: legacy-billing
    target_project: billing-svc
    capabilities: [refunds]
    change: extend-billing-refunds
    slice: extract-billing-refunds
    merged_at: 2026-05-02
```

Schema rules (`additionalProperties: false`):

| Field | Required | Notes |
|---|---|---|
| `version` | yes | `1` only. Future bumps go through an RFC update. |
| `entries[].source` | yes | Must match a key in `sources.yaml` at write time. Pinned in the ledger even if the catalogue entry is later removed. |
| `entries[].target_project` | yes | Kebab-case; matches `registry.yaml:projects[].name` at write time. |
| `entries[].capabilities` | yes | Kebab-case capability names; sorted alphabetically; non-empty. Derived from the slice's `spec.md` requirement headings or target-specific spec grouping, deduplicated and kebab-cased. |
| `entries[].change` | yes | Kebab-case change name. |
| `entries[].slice` | yes | Kebab-case slice name within that change. |
| `entries[].merged_at` | yes | ISO 8601 date, UTC, day precision (no time component). The deterministic part of slice merge time; chosen over a full timestamp to keep the file diff-friendly and preserve the framework's idempotency posture. |

Idempotency: entries are stored sorted by `(source, merged_at, slice)`. Re-running `emery plan archive` on an already-finalized change is a no-op (entries are deduplicated by `(change, slice, source, target_project)`).

### Ledger writers

The **only** writers to `migration-log.yaml`:

- **`emery slice merge`** — when a slice with `sources: [...]` and `project: <name>` is merged and the owning plan entry becomes `done`, appends one entry per `(source, target_project)` pair derived from the slice's `sources[]` list. The `capabilities` list is computed from the slice's `spec.md` requirement headings or target-specific spec grouping. For greenfield slices (`sources: []`), nothing is written.
- **`emery plan archive`** — defensive idempotency. Walks every `done` slice in the change, ensures the matching ledger entries exist, and writes any missing ones. Also updates `sources.yaml:sources[].status` (see below).

The ledger never gets a writer in any phase skill body (`/emery:refine`, `/emery:build`, `/emery:merge`). The deterministic CLI merge/finalize verbs are the natural single-writer sites for cross-change durable state.

### `status` field on `sources.yaml:sources[]`

RFC-21 deferred this field; this RFC introduces it. The schema gains:

```json
"status": {
  "type": "string",
  "enum": ["pending", "in-progress", "migrated", "abandoned"],
  "description": "Lifecycle state. Driven by the migration ledger writers; operator override via `emery source status`."
}
```

Defaults and transitions:

- **Default on `emery source add`** - `pending`.
- **`pending` -> `in-progress`** - when any active plan entry references the key in its `sources[]` list and the entry is in `pending` or `in-progress` status.
- **`in-progress` -> `migrated`** - when the ledger contains at least one entry for the key *and* every active plan entry referencing the key has reached `done`. Computed at finalize time. The plan has no per-entry `skipped` state; a future skip state can extend this rule when it returns.
- **Any state -> `abandoned`** - operator-driven only, via `emery source status <key> abandoned --reason "..."`. Refuses if any active plan entry still references the key.

The transitions are **automatic** for `pending` -> `in-progress` -> `migrated`. Operators may also force a transition with `emery source status <key> <value>`, which prompts for confirmation and records the override in `.emery/migration-log.yaml` as a special operator-override entry:

```yaml
- source: legacy-billing
  target_project: ""              # empty for status overrides
  capabilities: []            # empty for status overrides
  change: ""                      # empty for status overrides
  slice: ""                       # empty for status overrides
  merged_at: 2026-05-08
  override:
    status: abandoned
    reason: "Replaced by SaaS vendor; no migration planned."
    operator: "<git config user.email>"
```

The schema permits the `override` block as an optional field on entries with empty `change`/`slice`. This keeps the ledger as the single audit trail for the status field without inventing a parallel store.

### Ledger readers

The ledger has many consumers; none of them write:

- **Survey brief** — surfaces "previously migrated" rows in its source inventory. The new column is `Migrated` with values `—`, `partial`, or `yes`.
- **Propose routing (`/emery:plan`)** — when a plan entry's source key has prior `target_project` rows, defaults to that project with high confidence and surfaces `previously migrated to <project> via <change>` as the rationale. The routing-hint precedence becomes:

  1. Domain-model `target_project` hint.
  2. **Ledger lookup — same source key has prior target.** *(new in this RFC)*
  3. Survey mapping recommendation.
  4. Description match (existing).
  5. Baseline spec affinity (existing).
  6. Adapter compatibility (existing).
  7. Ambiguity -> human (existing).

- **Propose slice culling (`/emery:plan`)** — when survey shows an adapter whose source has been fully migrated, propose surfaces the prior migration and omits the slice unless the operator explicitly keeps it. The plan has no per-entry `skipped` state to pre-mark.
- **`emery source show`** - joins the catalogue with the ledger to render a per-source migration history.

A new dedicated read verb is also provided:

```bash
emery migration-log show [--source <key>] [--target-project <name>] [--change <name>] [--format json]
```

This is read-only; it does not write to the ledger. Useful for ad-hoc audit and operator queries.

### `mapping` field on `plan.yaml.slices[]`

An additive field on the existing `plan.yaml.slices[]` entry schema:

```yaml
slices:
  - name: consolidate-identity
    sources: [legacy-a, legacy-b]
    project: identity-svc
    mapping: many-to-one          # new optional field
    description: Consolidate user-registration from legacy-a and legacy-b.
    status: pending
```

Schema (additive change to [`schemas/plan/plan.schema.json`](../../schemas/plan/plan.schema.json)):

```json
"mapping": {
  "type": "string",
  "enum": ["one-to-one", "many-to-one", "one-to-many", "greenfield"],
  "description": "Optional source-to-target mapping pattern. Audit-only; the slice's actual behaviour is determined by `sources[]` and `project`."
}
```

Rules:

- **Optional.** Every existing plan validates as today.
- **Audit-only.** The slice loop, propose, and execute do **not** branch on this field. It is metadata for the operator and for survey's recommendations.
- **Cross-slice consistency check.** `emery plan validate` adds advisory findings (warnings, not errors) when:
  - `mapping: one-to-one` is set but `len(sources) != 1`;
  - `mapping: many-to-one` is set but `len(sources) < 2`;
  - `mapping: one-to-many` is set but the same `sources[0]` does not appear in another slice with a different `project` *within the same plan*;
  - `mapping: greenfield` is set but `len(sources) > 0`.
  The `one-to-many` check is intentionally scoped to the current plan; cross-change `one-to-many` is the ledger's concern, not validate's.
- **Survey produces it.** When survey emits a recommendation, propose pre-fills `mapping` per the survey's recommendation vocabulary (`one-to-one`, `candidate-consolidation` -> `many-to-one`, `candidate-split` -> `one-to-many`, `greenfield`).
- **Operator overrides via `emery plan amend <slice> --mapping <value>`** — same single-writer rule as every other plan field. Passing `""` or `--clear-mapping` removes the field.

### Why `mapping` ships with the ledger and not RFC-21

`mapping` is closely coupled to the survey recommendations and the cross-change picture the ledger paints (this RFC). Recording `mapping: many-to-one` is uninteresting without an audit trail that ties the slice to ledger entries; recording `mapping: one-to-many` is uninteresting without the cross-plan / cross-change visibility the ledger enables. Putting them in the same RFC keeps the audit story coherent.

### CLI surface summary

Net adds:

- `emery migration-log show` — read-only ledger query.
- `emery source status <key> <value> [--reason "..."]` - operator-driven status override.
- `emery plan amend <slice> --mapping <value>` — additive flag on the existing verb. Also `--clear-mapping` for removal.

Net schema changes:

- New: `schemas/migration-log/schema.json`.
- Additive: `status` field on `sources[]` in `schemas/sources.schema.json`.
- Additive: `mapping` field on `plan.yaml.slices[]` in `schemas/plan/plan.schema.json`.

No verb is renamed, retired, or repurposed. No existing schema field is changed in shape or required-ness.

### Scenario coverage

| Scenario | Pre-RFC-22 (RFCs 20 + 21 landed) | Post-RFC-22 |
|---|---|---|
| 1. Single-repo migration | Survey + synthesise; catalogue + cache via RFC-21. | Same, plus the source's status flips through `pending` -> `in-progress` -> `migrated` automatically. Single-source `mapping` rarely useful. |
| 2. Multi-repo migration (80+ repos) | Sources declared; cache shared; `survey` fans out. No cross-change memory; consolidations/splits not labelled. | Ledger records every merged migration; propose routes against ledger; survey labels consolidation/split candidates; `mapping` field carries intent into audit. |
| 3. Greenfield multi-repo | Domain-model-driven topology. | `mapping: greenfield` available for audit. Ledger empty (no sources). |
| 4. Brownfield multi-repo | Routing via baseline + domain-model hints. | Ledger augments routing: previously migrated keys default to their prior target with high confidence. |

## Implementation Plan

1. **Schemas.** Land `migration-log/schema.json`, the additive `status` field on `schemas/sources.schema.json`, and the additive `mapping` field on `plan/plan.schema.json`. Update each schema's README. Add JSON Schema fixtures.
2. **Domain types.** Add `MigrationLog`, `MigrationEntry`, `MigrationOverride` types in `project` (`crates/project/src/migration_log/`). Mirror the `Registry` posture: `serde(deny_unknown_fields)`, `path()` / `load()` / `append()` helpers, byte-stable sort.
3. **Ledger writers.** Hook `emery slice merge` and `emery plan archive` to derive ledger entries. Atomic file-write through the existing `AtomicYaml` trait. Land integration tests under the owning crate's `tests/`.
4. **`status` on `sources[]`.** Wire `emery plan archive` to update statuses. Add `emery source status` verb (`src/commands/source/status.rs`) with the operator-override override-block writer.
5. **`emery migration-log show`.** Read-only verb; small handler with `--source`, `--target-project`, `--change` filters and JSON envelope.
6. **`mapping` field plumbing.** Extend `Plan::validate` with the four advisory cross-checks. Extend `emery plan amend` to accept `--mapping` and `--clear-mapping`. Update propose brief to pre-fill `mapping` from survey recommendations.
7. **Routing-hint precedence.** Update the propose routing brief to insert the ledger lookup as hint #2. Surface `previously migrated to <project> via <change>` in the routing-table rationale.
8. **`emery migration-log import --from-archive` (one-shot helper).** Optional helper for operators upgrading from a pre-ledger state. Walks `.emery/archive/plans/<date>-<name>/` directories and synthesises ledger entries from archived `plan.yaml` + slice `specs/`. Refuses to overwrite an existing ledger; appends only missing entries when `--merge` is set.
9. **Tutorials and references.** Update `docs/tutorials/legacy-migration-at-scale.md` and the new `docs/tutorials/multi-repo-legacy-migration.md` (from RFC-21) to use the ledger and `mapping` field. Add a section to `docs/explanation/concepts.md` introducing the ledger.
10. **Acceptance.** Extend the cross-repo Deno acceptance suite with: an N=10 multi-source migration over two consecutive changes asserting ledger writes, status auto-transitions, and propose's ledger-driven routing; a `mapping` validate-warning fixture; an operator-override `emery source status ... abandoned` flow.

## Migration

This RFC is **strictly additive**. Pre-existing plans, registries, sources, archives, and changes continue to work without change.

For operators:

- The ledger first populates on the next `emery slice merge` or `emery plan archive` after upgrade. It does not retroactively backfill. Operators who want backfill can run `emery migration-log import --from-archive` once, post-upgrade.
- The `status` field on `sources[]` is optional and defaults to `pending` on existing entries when first read by a writer. The framework auto-promotes `pending` -> `in-progress` -> `migrated` based on plan / ledger state; manual overrides go through `emery source status`.
- The `mapping` field on plan slices is optional. Existing plans validate without it; new plans authored after upgrade may set it manually or accept the propose brief's pre-fill from survey recommendations.

For adapter authors:

- Survey and synthesise briefs gain a new readable input (`migration-log.yaml`) and a new column in the source-inventory table (`Migrated`). Existing briefs without survey/synthesise are unaffected.
- The `mapping` enum is shared vocabulary; adapter briefs may consume it but are not required to produce it.

For skill authors:

- `emery migration-log show --format json` is a new readable surface with a stable JSON envelope. Treat it like a read-only inspection surface.

There is **no breaking change** to: existing `plan.yaml` files (the `mapping` field is optional), existing `sources.yaml` files (the `status` field is optional with a sensible default), existing `registry.yaml` files (untouched), existing exit codes (new discriminants live within `EXIT_VALIDATION_FAILED=2` and `EXIT_GENERIC_FAILURE=1`), or existing archive layouts (the ledger lives outside the archive directory).

## Alternatives Considered

**Make the migration ledger a derived view rather than a materialised file.** Rejected. Survey and propose both consume it on every plan run; computing it from archives every time is O(archives × slices) and re-introduces the very scaling problem this RFC fixes. Materialising it as a small, append-mostly file is the cheaper and clearer answer. The archive remains the source of truth; the ledger is a cache with explicit writers.

**Encode mapping as `tags: [many-to-one]` rather than a typed enum.** Rejected. Free-form tags evade schema validation and the cross-slice consistency check. The closed enum is small, audit-friendly, and matches the framework's posture on every other taxonomy (kinds, statuses, shapes).

**Implicit ledger updates from the `/emery:merge` skill body rather than `emery slice merge`.** Rejected. The skill body is per-slice orchestration; deterministic state mutation belongs in the CLI merge verb that already writes the plan entry's `done` status.

**Allow `mapping: many-to-many`.** Rejected. The slice loop's invariant is one slice -> one project; `many-to-many` cannot exist on a single slice (it requires multiple slices). The four-value enum captures every legal shape.

**Put ledger overrides in a separate `migration-overrides.yaml` file.** Rejected. Two files for one logical audit trail invites drift. The override-block on a ledger entry with empty `change`/`slice` keeps the audit story in one place and is unambiguous to parse (the `override` field is the discriminator).

**Compute `sources[].status` on the fly from the ledger and never persist it.** Rejected. Operators want to query status with a single `emery source show`, not a join across `sources.yaml` + every active plan + the ledger. Persisting the field is a small denormalisation that pays for itself in operational ergonomics; the writers maintain consistency.

**Make `status` writes synchronous with `emery plan next`.** Considered and rejected. Status writes happen when a plan entry becomes `in-progress` (cheap) and at finalize time for `migrated` (because that's when "every slice referencing the key is done" is settled). Splitting the write across two verbs avoids a costly cross-plan check on every transition.

**Block `emery plan archive` on any source referenced by the change still being `in-progress`.** Considered and rejected. The check is well-defined but punitive - partial progress is a legitimate state for multi-change migrations. Warning, not block; operators can opt to gate via CI if they want stricter posture.

## Non-Goals

- Multi-plan output or parallel changes.
- Cross-platform-repo ledger sharing (the ledger is per-platform-repo).
- A `confidence` field on ledger entries (review findings live in their own surface).
- Driving execution from the ledger (the ledger is read-only for every executor-side path).
- Backstage / external catalogue export (deferred pending a demonstrated catalog-assisted discovery need).
- Replacing operator review with ledger-driven decisions in propose. The ledger is advisory throughout.
- A general "migration timeline" UI or report. The JSON envelope on `emery migration-log show` is sufficient for downstream tooling to build that.
- Retroactively rewriting ledger entries when a change is dropped post-finalize (drops happen pre-merge; post-merge edits are out of scope).

## Open Questions

1. Should the ledger record `started_at` (when the plan entry first transitioned to `in-progress`) in addition to `merged_at`? Current preference: no - keep the schema minimal; archives carry per-transition timestamps via `journal.jsonl` for forensic detail.
2. Should `emery plan archive` block on `sources.yaml:sources[].status: in-progress` for any source key referenced by the change's slices, to enforce status hygiene? Current preference: warning, not block.
3. Should the `mapping: one-to-many` validate-warning be cross-change (i.e., the *other* slice with a different project may live in a separate change)? Current preference: scoped to the current plan; cross-change is the ledger's job, not validate's.
4. Should `emery migration-log import --from-archive` ship in this RFC or as a follow-up? Current preference: in this RFC, gated behind explicit `--merge` to prevent accidental ledger surgery.
5. Should the operator-override entry shape live under `entries[]` (with empty fields) or a separate `overrides[]` collection? Current preference: under `entries[]` for single-source-of-truth; the discriminator is the `override` block.
6. Should the override `operator` field be required, optional, or auto-populated from `git config user.email`? Current preference: auto-populated, with `emery source status --operator <id>` as an explicit override for CI.
7. Should `emery plan amend --mapping` require `--reason` like the registry-amendment verbs? Current preference: no — the field is audit-only and the change is recoverable via re-amend.
8. How should the ledger handle a change name being reused after archive (e.g., `migrate-billing-2026-q2` merged then a new change with the same name created)? Current preference: ledger entries are immutable once written; the new change's transitions append fresh entries with the same `change` name and a later `merged_at`. Survey readers may surface the duplication as a warning.

## References

- [`docs/standards/workflow.md`](../../docs/standards/workflow.md) — the source-adapter flow and merge/archive writers this RFC's ledger annotates.
- [`engine/crates/project/src/journal/event.rs`](../../crates/project/src/journal/event.rs) — the `slice.archive.created` outcome-ledger event a rewrite should project the migration ledger over.
- [RFC-21: Source Catalogue and Source-Clone Cache](rfc-21-catalogue.md) — `sources.yaml` and the cache the ledger annotates.
- [`docs/explanation/adapter-anatomy.md`](../../docs/explanation/adapter-anatomy.md) — the source/target axis split the ledger annotates.
- [`docs/tutorials/legacy-migration-at-scale.md`](../../docs/tutorials/legacy-migration-at-scale.md) — the canonical multi-source migration walkthrough this RFC updates.
- [`schemas/plan/plan.schema.json`](../../schemas/plan/plan.schema.json) — the schema this RFC additively extends with `mapping`.
- [`engine/crates/project/src/registry/catalog.rs`](../../crates/project/src/registry/catalog.rs) — reference implementation for the `Registry` posture the ledger mirrors.
