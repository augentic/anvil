# RFC-22: Migration Ledger and Slice Mapping

> Status: Draft - Depends: [RFC-3a](archive/rfc-3a-monoliths.md), [RFC-3b](archive/rfc-3b-platform.md), [RFC-9](archive/rfc-9-platform.md), [RFC-20](rfc-20-survey.md), [RFC-21](rfc-21-catalogue.md)

## Abstract

Add the cumulative cross-change state required to plan, route, and audit migrations that span many changes. Today's framework records per-change history through `.specify/archive/plans/<date>-<name>/`, but assignment, propose, and survey have no machine-readable answer to "is this source migrated yet?" or "what's the source-to-target pattern of this slice?" — both questions get harder as a multi-repo migration grows.

This RFC adds:

1. **`.specify/migration-log.yaml`** — a cumulative ledger recording, per source key, which target projects each capability landed in, when, and via which change. Written only by `specify plan transition <slice> done` and `specify change finalize`; read by survey, assignment, propose, and the new `specify migration-log show` verb.
2. **A `status` field on `sources.yaml:sources[]`** (closed enum: `pending` / `in-progress` / `migrated` / `abandoned`) — driven by the same ledger writers. Operators can override via an explicit `specify sources status` verb, but normal lifecycle transitions are framework-driven.
3. **An optional `mapping` field on each `planSlice`** (`one-to-one` / `many-to-one` / `one-to-many` / `greenfield`) — produced by survey, consumed by audit, validated by `specify plan validate`. Audit-only; the slice loop does not branch on it.

These additions fold cross-change durability into the planning loop without touching the slice loop, the workspace-tier boundary, or the one-slice-one-project invariant. The ledger is a *materialised cache* over archive contents; archives remain the source of truth.

## Motivation

RFC-20 added cross-source synthesis at plan time; RFC-21 added the source catalogue and tier-1 cache. With both in place, three gaps remain visible at scale:

- **No machine-readable cross-change memory.** Assignment uses `workspace.md` baseline-spec affinity ("this target already has overlapping specs"), but the framework cannot answer "is `legacy-billing` done?" without scanning archives. The information is scattered across `.specify/archive/plans/<date>-<name>/`, with no index.
- **Source status is implicit.** RFC-21 deliberately punted on the `status` field for `sources[]` because there were no writers. Operators want to know which legacy sources are pending, in flight, or done — and they want the framework to maintain that signal honestly, not by hand-edit.
- **No first-class consolidation/split metadata on slices.** `sources: [k1, k2]` mechanically expresses consolidation, and a single source-key appearing in two slices with different projects mechanically expresses splitting, but neither is *labelled*. Audit and review must reverse-engineer the intent. Survey (RFC-20) recommends mappings; without a place to record them, the recommendations vanish into operator review.

This RFC adds the smallest set of cross-change durable artifacts that fix all three, with the same single-writer invariants the framework already applies to `plan.yaml` and `registry.yaml`.

## Design

### Principles

1. **The ledger is a materialised view.** `.specify/migration-log.yaml` is an append-mostly cache over archive contents. Re-deriving it from archives must always be possible. The archive remains the source of truth.
2. **The CLI is the single writer.** Only `specify plan transition <slice> done` and `specify change finalize` write to the ledger. `specify sources status` is the *only* operator-facing override for the `status` field; otherwise it tracks the ledger.
3. **Schemas are strict.** `additionalProperties: false`, kebab-case identifiers, deny-unknown-fields, byte-stable serialisation. Same posture as `plan.schema.json` and `Registry::validate_shape`.
4. **`mapping` is audit-only.** The slice loop, propose, assignment, and execute do **not** branch on the field. It captures intent for review and regression-detection, not control flow.
5. **Additive throughout.** Every existing `plan.yaml`, `sources.yaml`, and archive layout continues to validate without change.
6. **No cross-platform-repo state.** The ledger is per-platform-repo, mirroring `registry.yaml` and `sources.yaml`.

### `.specify/migration-log.yaml` — the cumulative ledger

A new durable artifact at `.specify/migration-log.yaml`. Schema at `specify-cli/schemas/migration-log/schema.json`.

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/augentic/specify-cli/main/schemas/migration-log/schema.json
version: 1
entries:
  - source_key: legacy-billing
    target_project: billing-svc
    capabilities: [dunning, invoicing]
    change: migrate-billing-2026-q2
    slice: extract-billing-core
    finalized_at: 2026-04-15
  - source_key: legacy-billing
    target_project: billing-svc
    capabilities: [refunds]
    change: extend-billing-refunds
    slice: extract-billing-refunds
    finalized_at: 2026-05-02
```

Schema rules (`additionalProperties: false`):

| Field | Required | Notes |
|---|---|---|
| `version` | yes | `1` only. Future bumps go through an RFC update. |
| `entries[].source_key` | yes | Must match a key in `sources.yaml` at write time. Pinned in the ledger even if the catalogue entry is later removed. |
| `entries[].target_project` | yes | Kebab-case; matches `registry.yaml:projects[].name` at write time. |
| `entries[].capabilities` | yes | Kebab-case capability names; sorted alphabetically; non-empty. Derived from the slice's `specs/<crate>/spec.md` `### Requirement:` block titles, deduplicated and kebab-cased. |
| `entries[].change` | yes | Kebab-case change name. |
| `entries[].slice` | yes | Kebab-case slice name within that change. |
| `entries[].finalized_at` | yes | ISO 8601 date, UTC, day precision (no time component). The deterministic part of finalize time; chosen over a full timestamp to keep the file diff-friendly and preserve the framework's idempotency posture. |

Idempotency: entries are stored sorted by `(source_key, finalized_at, slice)`. Re-running `specify change finalize` on an already-finalised change is a no-op (entries are deduplicated by `(change, slice, source_key, target_project)`).

### Ledger writers

The **only** writers to `migration-log.yaml`:

- **`specify plan transition <slice> done`** — when a slice with `sources: [...]` and `project: <name>` transitions to `done`, appends one entry per `(source_key, target_project)` pair derived from the slice's `sources[]` list. The `capabilities` list is computed from `specs/<crate>/spec.md` requirement titles. For greenfield slices (`sources: []`), nothing is written.
- **`specify change finalize`** — defensive idempotency. Walks every `done` slice in the change, ensures the matching ledger entries exist, and writes any missing ones. Also updates `sources.yaml:sources[].status` (see below).

The ledger never gets a writer in any phase skill (`/spec:define`, `/spec:build`, `/spec:merge`). Plan transitions are the natural single-writer site for cross-change durable state.

### `status` field on `sources.yaml:sources[]`

RFC-21 deferred this field; this RFC introduces it. The schema gains:

```json
"status": {
  "type": "string",
  "enum": ["pending", "in-progress", "migrated", "abandoned"],
  "description": "Lifecycle state. Driven by the migration ledger writers; operator override via `specify sources status`."
}
```

Defaults and transitions:

- **Default on `specify sources add`** — `pending`.
- **`pending` → `in-progress`** — when any active plan slice references the key in its `sources[]` list and the slice is in `pending` or `in-progress` status.
- **`in-progress` → `migrated`** — when the ledger contains at least one entry for the key *and* every active plan slice referencing the key has reached `done` or `skipped`. Computed at `specify change finalize` time.
- **Any state → `abandoned`** — operator-driven only, via `specify sources status <key> abandoned --reason "..."`. Refuses if any active plan slice still references the key.

The transitions are **automatic** for `pending` ↔ `in-progress` ↔ `migrated`. Operators may also force a transition with `specify sources status <key> <value>`, which prompts for confirmation and records the override in `.specify/migration-log.yaml` as a special operator-override entry:

```yaml
- source_key: legacy-billing
  target_project: ""              # empty for status overrides
  capabilities: []                # empty for status overrides
  change: ""                      # empty for status overrides
  slice: ""                       # empty for status overrides
  finalized_at: 2026-05-08
  override:
    status: abandoned
    reason: "Replaced by SaaS vendor; no migration planned."
    operator: "<git config user.email>"
```

The schema permits the `override` block as an optional field on entries with empty `change`/`slice`. This keeps the ledger as the single audit trail for the status field without inventing a parallel store.

### Ledger readers

The ledger has many consumers; none of them write:

- **Survey brief (RFC-20)** — surfaces "previously migrated" rows in its source inventory. The new column is `Migrated` with values `—`, `partial`, or `yes`.
- **Assignment brief (RFC-20)** — when a plan entry's source key has prior `target_project` rows, defaults to that project with high confidence and surfaces `previously migrated to <project> via <change>` as the rationale. The routing-hint precedence (RFC-20) becomes:

  1. Domain-model `target_project` hint (RFC-20).
  2. **Ledger lookup — same source key has prior target.** *(new in this RFC)*
  3. Survey mapping recommendation (RFC-20).
  4. Description match (existing).
  5. Baseline spec affinity (existing).
  6. Capability compatibility (existing).
  7. Ambiguity → human (existing).

- **Propose brief (RFC-20)** — when survey shows a capability whose source has been fully migrated, propose may pre-mark the slice `skipped` with `status-reason: "previously migrated in change <name>"`.
- **`specify sources show`** — joins the catalogue with the ledger to render a per-source migration history.

A new dedicated read verb is also provided:

```bash
specify migration-log show [--source-key <key>] [--target-project <name>] [--change <name>] [--format json]
```

This is read-only; it does not write to the ledger. Useful for ad-hoc audit and operator queries.

### `mapping` field on `planSlice`

An additive field on the existing `planSlice` schema:

```yaml
slices:
  - name: consolidate-identity
    sources: [legacy-a, legacy-b]
    project: identity-svc
    mapping: many-to-one          # new optional field
    description: Consolidate user-registration from legacy-a and legacy-b.
    status: pending
```

Schema (additive change to [`plan.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/plan/plan.schema.json)):

```json
"mapping": {
  "type": "string",
  "enum": ["one-to-one", "many-to-one", "one-to-many", "greenfield"],
  "description": "Optional source-to-target mapping pattern. Audit-only; the slice's actual behaviour is determined by `sources[]` and `project`."
}
```

Rules:

- **Optional.** Every existing plan validates as today.
- **Audit-only.** The slice loop, propose, assignment, and execute do **not** branch on this field. It is metadata for the operator and for survey's recommendations.
- **Cross-slice consistency check.** `specify plan validate` adds advisory findings (warnings, not errors) when:
  - `mapping: one-to-one` is set but `len(sources) != 1`;
  - `mapping: many-to-one` is set but `len(sources) < 2`;
  - `mapping: one-to-many` is set but the same `sources[0]` does not appear in another slice with a different `project` *within the same plan*;
  - `mapping: greenfield` is set but `len(sources) > 0`.
  The `one-to-many` check is intentionally scoped to the current plan; cross-change `one-to-many` is the ledger's concern, not validate's.
- **Survey produces it.** When survey emits a recommendation, propose pre-fills `mapping` per the survey's recommendation vocabulary (`one-to-one`, `candidate-consolidation` → `many-to-one`, `candidate-split` → `one-to-many`, `greenfield`).
- **Operator overrides via `specify plan amend <slice> --mapping <value>`** — same single-writer rule as every other plan field. Passing `""` or `--clear-mapping` removes the field.

### Why `mapping` ships with the ledger and not RFC-21

`mapping` is closely coupled to the survey recommendations (RFC-20) and the cross-change picture the ledger paints (this RFC). Recording `mapping: many-to-one` is uninteresting without an audit trail that ties the slice to ledger entries; recording `mapping: one-to-many` is uninteresting without the cross-plan / cross-change visibility the ledger enables. Putting them in the same RFC keeps the audit story coherent.

### CLI surface summary

Net adds:

- `specify migration-log show` — read-only ledger query.
- `specify sources status <key> <value> [--reason "..."]` — operator-driven status override.
- `specify plan amend <slice> --mapping <value>` — additive flag on the existing verb. Also `--clear-mapping` for removal.

Net schema changes:

- New: `specify-cli/schemas/migration-log/schema.json`.
- Additive: `status` field on `sources[]` in `specify-cli/schemas/sources/sources.schema.json`.
- Additive: `mapping` field on `planSlice` in `specify-cli/schemas/plan/plan.schema.json`.

No verb is renamed, retired, or repurposed. No existing schema field is changed in shape or required-ness.

### Scenario coverage

| Scenario | Pre-RFC-22 (RFCs 20 + 21 landed) | Post-RFC-22 |
|---|---|---|
| 1. Single-repo migration | Survey + synthesise via RFC-20; catalogue + cache via RFC-21. | Same, plus the source's status flips through `pending` → `in-progress` → `migrated` automatically. Single-source `mapping` rarely useful. |
| 2. Multi-repo migration (80+ repos) | Sources declared; cache shared; analyze fans out. No cross-change memory; consolidations/splits not labelled. | Ledger records every finalised migration; assignment routes against ledger; survey labels consolidation/split candidates; `mapping` field carries intent into audit. |
| 3. Greenfield multi-repo | Domain-model-driven topology via RFC-20. | `mapping: greenfield` available for audit. Ledger empty (no sources). |
| 4. Brownfield multi-repo | Routing via baseline + domain-model hints. | Ledger augments routing: previously migrated keys default to their prior target with high confidence. |

## Implementation Plan

1. **Schemas.** Land `migration-log/schema.json`, the additive `status` field on `sources/sources.schema.json`, and the additive `mapping` field on `plan/plan.schema.json`. Update each schema's README. Add JSON Schema fixtures.
2. **Domain types.** Add `MigrationLog`, `MigrationEntry`, `MigrationOverride` types in `specify-domain` (`crates/domain/src/migration_log/`). Mirror the `Registry` posture: `serde(deny_unknown_fields)`, `path()` / `load()` / `append()` helpers, byte-stable sort.
3. **Ledger writers.** Hook `specify plan transition <slice> done` and `specify change finalize` to derive ledger entries. Atomic file-write through the existing `AtomicYaml` trait. Land integration tests under `tests/migration_log.rs`.
4. **`status` on `sources[]`.** Wire `specify change finalize` to update statuses. Add `specify sources status` verb (`src/commands/sources/status.rs`) with the operator-override override-block writer.
5. **`specify migration-log show`.** Read-only verb; small handler with `--source-key`, `--target-project`, `--change` filters and JSON envelope.
6. **`mapping` field plumbing.** Extend `Plan::validate` with the four advisory cross-checks. Extend `specify plan amend` to accept `--mapping` and `--clear-mapping`. Update propose brief to pre-fill `mapping` from survey recommendations.
7. **Routing-hint precedence.** Update `assignment.md` (the brief from RFC-20) to insert the ledger lookup as hint #2. Surface `previously migrated to <project> via <change>` in the assignment-table rationale.
8. **`specify migration-log import --from-archive` (one-shot helper).** Optional helper for operators upgrading from a pre-ledger state. Walks `.specify/archive/plans/<date>-<name>/` directories and synthesises ledger entries from archived `plan.yaml` + slice `specs/`. Refuses to overwrite an existing ledger; appends only missing entries when `--merge` is set.
9. **Tutorials and references.** Update `docs/tutorials/legacy-migration-at-scale.md` and the new `docs/tutorials/multi-repo-legacy-migration.md` (from RFC-21) to use the ledger and `mapping` field. Add a section to `docs/explanation/concepts.md` introducing the ledger.
10. **Acceptance.** Extend the cross-repo Deno acceptance suite with: an N=10 multi-source migration over two consecutive changes asserting ledger writes, status auto-transitions, and assignment's ledger-driven routing; a `mapping` validate-warning fixture; an operator-override `specify sources status … abandoned` flow.

## Migration

This RFC is **strictly additive**. Pre-existing plans, registries, sources, archives, and changes continue to work without change.

For operators:

- The ledger first populates on the next `specify plan transition done` or `specify change finalize` after upgrade. It does not retroactively backfill. Operators who want backfill can run `specify migration-log import --from-archive` once, post-upgrade.
- The `status` field on `sources[]` is optional and defaults to `pending` on existing entries when first read by a writer. The framework auto-promotes `pending` → `in-progress` → `migrated` based on plan / ledger state; manual overrides go through `specify sources status`.
- The `mapping` field on plan slices is optional. Existing plans validate without it; new plans authored after upgrade may set it manually or accept the propose brief's pre-fill from survey recommendations.

For capability authors:

- Survey and synthesise briefs (RFC-20) gain a new readable input (`migration-log.yaml`) and a new column in the source-inventory table (`Migrated`). Existing briefs without survey/synthesise are unaffected.
- The `mapping` enum is shared vocabulary; capability briefs may consume it but are not required to produce it.

For skill authors:

- `specify migration-log show --format json` is a new readable surface with a stable JSON envelope. Treat it like `specify registry show` and `specify plan status`.

There is **no breaking change** to: existing `plan.yaml` files (the `mapping` field is optional), existing `sources.yaml` files (the `status` field is optional with a sensible default), existing `registry.yaml` files (untouched), existing exit codes (new discriminants live within `EXIT_VALIDATION_FAILED=2` and `EXIT_GENERIC_FAILURE=1`), or existing archive layouts (the ledger lives outside the archive directory).

## Alternatives Considered

**Make the migration ledger a derived view rather than a materialised file.** Rejected. Survey, assignment, and propose all consume it on every plan run; computing it from archives every time is O(archives × slices) and re-introduces the very scaling problem this RFC fixes. Materialising it as a small, append-mostly file is the cheaper and clearer answer. The archive remains the source of truth; the ledger is a cache with explicit writers.

**Encode mapping as `tags: [many-to-one]` rather than a typed enum.** Rejected. Free-form tags evade schema validation and the cross-slice consistency check. The closed enum is small, audit-friendly, and matches the framework's posture on every other taxonomy (kinds, statuses, shapes).

**Implicit ledger updates from `/spec:merge` rather than `specify plan transition done`.** Rejected. `/spec:merge` is per-slice and unaware of the plan; it would require new cross-skill coupling. Plan transitions are the natural single-writer site for cross-change durable state.

**Allow `mapping: many-to-many`.** Rejected. The slice loop's invariant is one slice → one project; `many-to-many` cannot exist on a single slice (it requires multiple slices). The four-value enum captures every legal shape.

**Put ledger overrides in a separate `migration-overrides.yaml` file.** Rejected. Two files for one logical audit trail invites drift. The override-block on a ledger entry with empty `change`/`slice` keeps the audit story in one place and is unambiguous to parse (the `override` field is the discriminator).

**Compute `sources[].status` on the fly from the ledger and never persist it.** Rejected. Operators want to query status with a single `specify sources show`, not a join across `sources.yaml` + every active plan + the ledger. Persisting the field is a small denormalisation that pays for itself in operational ergonomics; the writers maintain consistency.

**Make `status` writes synchronous with `specify plan transition`.** Considered and rejected. Status writes happen at `transition` time for `in-progress` (cheap) and at `finalize` time for `migrated` (because that's when "every slice referencing the key is done" is settled). Splitting the write across two verbs avoids a costly cross-plan check on every transition.

**Block `specify change finalize` on any source-key referenced by the change still being `in-progress`.** Considered and rejected. The check is well-defined but punitive — partial progress is a legitimate state for multi-change migrations. Warning, not block; operators can opt to gate via CI if they want stricter posture.

## Non-Goals

- Multi-plan output or parallel changes.
- Cross-platform-repo ledger sharing (the ledger is per-platform-repo).
- A `confidence` field on ledger entries (review findings live in their own surface).
- Driving execution from the ledger (the ledger is read-only for every executor-side path).
- Backstage / external catalogue export (deferred; consistent shape with [RM-12](roadmap.md#rm-12-catalog-import-backstage-adapter)).
- Replacing operator review with ledger-driven decisions in propose. The ledger is advisory throughout.
- A general "migration timeline" UI or report. The JSON envelope on `specify migration-log show` is sufficient for downstream tooling to build that.
- Retroactively rewriting ledger entries when a change is dropped post-finalize (drops happen pre-merge; post-merge edits are out of scope).

## Open Questions

1. Should the ledger record `started_at` (when the slice first transitioned to `in-progress`) in addition to `finalized_at`? Current preference: no — keep the schema minimal; archives carry per-transition timestamps via `journal.yaml` for forensic detail.
2. Should `specify change finalize` block on `sources.yaml:sources[].status: in-progress` for any source key referenced by the change's slices, to enforce status hygiene? Current preference: warning, not block.
3. Should the `mapping: one-to-many` validate-warning be cross-change (i.e., the *other* slice with a different project may live in a separate change)? Current preference: scoped to the current plan; cross-change is the ledger's job, not validate's.
4. Should `specify migration-log import --from-archive` ship in this RFC or as a follow-up? Current preference: in this RFC, gated behind explicit `--merge` to prevent accidental ledger surgery.
5. Should the operator-override entry shape live under `entries[]` (with empty fields) or a separate `overrides[]` collection? Current preference: under `entries[]` for single-source-of-truth; the discriminator is the `override` block.
6. Should the override `operator` field be required, optional, or auto-populated from `git config user.email`? Current preference: auto-populated, with `specify sources status --operator <id>` as an explicit override for CI.
7. Should `specify plan amend --mapping` require `--reason` like the registry-amendment verbs? Current preference: no — the field is audit-only and the change is recoverable via re-amend.
8. How should the ledger handle a change name being reused after archive (e.g., `migrate-billing-2026-q2` finalised then a new change with the same name created)? Current preference: ledger entries are immutable once written; the new change's transitions append fresh entries with the same `change` name and a later `finalized_at`. Survey readers may surface the duplication as a warning.

## References

- [RFC-3a: Monoliths](archive/rfc-3a-monoliths.md) — analyze/extract split this RFC's ledger annotates.
- [RFC-3b: Platform](archive/rfc-3b-platform.md) — assignment and one-slice-one-project invariant the ledger augments.
- [RFC-9: Platform](archive/rfc-9-platform.md) — historical change-lifecycle predecessor; ledger writers fire through `/change:execute` and `specify change finalize` in the current `/change:draft` → `/change:execute` → `/change:finalize` flow.
- [RFC-20: Survey-to-Plan Pipeline](rfc-20-survey.md) — survey, synthesise, propose, and assignment briefs that consume the ledger.
- [RFC-21: Source Catalogue and Tier-1 Cache](rfc-21-catalogue.md) — `sources.yaml` and the cache the ledger annotates.
- [RM-12: Catalog import — Backstage adapter](roadmap.md#rm-12-catalog-import-backstage-adapter) — long-term shape alignment for catalogue export.
- [`docs/explanation/workspace-tiers.md`](../docs/explanation/workspace-tiers.md) — tier-1 / tier-2 boundary the ledger preserves.
- [`docs/tutorials/legacy-migration-at-scale.md`](../docs/tutorials/legacy-migration-at-scale.md) — the canonical multi-source migration walkthrough this RFC updates.
- [`schemas/plan/plan.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/plan/plan.schema.json) — the schema this RFC additively extends with `mapping`.
- [`crates/domain/src/registry/catalog.rs`](https://github.com/augentic/specify-cli/blob/main/crates/domain/src/registry/catalog.rs) — reference implementation for the `Registry` posture the ledger mirrors.
