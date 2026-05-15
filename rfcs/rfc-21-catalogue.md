# RFC-21: Source Catalogue and Tier-1 Cache

> Status: Draft - Depends: [RFC-3a](archive/rfc-3a-monoliths.md), [RFC-3b](archive/rfc-3b-platform.md), [RFC-9](archive/rfc-9-platform.md), [RFC-14](archive/rfc-14-workspace.md), [RFC-20](rfc-20-survey.md)

## Abstract

Add a durable, platform-level catalogue of legacy source repositories (`sources.yaml`), a shared tier-1 cache for their clones (`.specify/.cache/sources/<key>/`), and a `--source @<key>` selector form for `/change:plan`. Together these let a platform repo declare dozens of legacy sources once and re-use them across many changes, without re-cloning each time and without conflating sources (planner-time inputs) with the existing target-project registry (executor-time outputs).

This RFC adds:

1. **`sources.yaml`** — a platform-repo catalogue of legacy sources, mirroring the role `registry.yaml` plays for target projects.
2. **`specify sources {add, remove, show, list, validate, sync}`** — a new CLI verb family, structurally parallel to `specify registry`.
3. **`.specify/.cache/sources/<key>/`** — a durable, shared tier-1 cache that survives `specify change plan archive`. Per-change `analyze/<key>/` slots become read-only symlinks into the cache.
4. **`--source @<key>`** — a new selector form on `/change:plan` (and any caller that accepts `--source`) that resolves the key against `sources.yaml`.
5. **`--analyze-concurrency <N>`** — a brief-level fan-out knob on `/change:plan` for parallel `/change:analyze` invocations across many sources.

These additions are **strictly additive**: the existing `--source <key>=<path-or-url>` form, the `/change:analyze` per-input contract, the workspace-tier separation, and every existing schema continue to work unchanged. The cumulative migration ledger and the `mapping` field on `planSlice` are deferred to RFC-22.

## Motivation

The framework already supports the *mechanics* of multi-source migration: `plan.yaml` slices accept `sources: [k1, k2, …]`; `/spec:extract` walks the union of sources at define time; assignment routes each entry to one project. What it does not provide — and what becomes prohibitive at 80+ source repositories — is the **declaration and caching layer** beneath those mechanics:

- **Sources are declared every change.** Operators repeat `--source <k>=<url>` for every plan invocation. Forty repos times two re-plans is eighty CLI flags. There is no artifact saying "these are the legacy sources we are migrating", separate from any one change.
- **Tier-1 clones are per-change and ephemeral.** Each `--source <k>=<git-url>` clones into `.specify/plans/<change>/analyze/<key>/` and is swept by `specify change plan archive`. Re-planning the same source means re-cloning. The survey brief from RFC-20 compounds the cost when run across many sources, because each survey re-iteration starts from scratch.
- **Discovery fan-out is sequential.** RFC-20's discovery brief invokes `/change:analyze` once per source in CLI declaration order. With 80 sources, the wall-clock hit is real, even though each invocation is cheap and independent.
- **Sources and targets get mixed.** Without a sources artifact, operators are tempted to record legacy URLs in `change.md` or as comments — both unsearchable and not validated. Without a clear separation, the workspace-tier boundary in [`docs/explanation/workspace-tiers.md`](../docs/explanation/workspace-tiers.md) blurs.

This RFC is the smallest set of additions that fix all four issues without violating Specify's existing posture (`registry.yaml` is target-only; archives are immutable; `/change:analyze` is per-input).

## Design

### Principles

1. **Sources are platform state, not change state.** `sources.yaml` lives at the platform-repo / hub root alongside `registry.yaml`. Like the registry, a missing file is *not* an error — it activates only when used.
2. **The CLI is the single writer.** `specify sources {add, remove, sync}` are the only writers to `sources.yaml`. No skill hand-edits the file; this mirrors the writer rules for `registry.yaml` and `plan.yaml`.
3. **The tier-1 boundary is preserved.** `.specify/.cache/sources/<key>/` is read-only with respect to `/change:analyze` and every other planner-time skill. Nothing in this RFC writes into a source clone.
4. **Schemas are strict.** `additionalProperties: false`, kebab-case identifiers, deny-unknown-fields, byte-stable serialisation. Same posture as `plan.schema.json` and `Registry::validate_shape`.
5. **Composition over invention.** One new verb family (`specify sources`) and one new flag form (`--source @<key>`); everything else reuses existing primitives.
6. **No cross-change durable state.** This RFC stops at "sources are declared and cached." Cross-change memory (the migration ledger) and slice-level mapping metadata are RFC-22 concerns.

### `sources.yaml` — the source-repo catalogue

A new file at the platform-repo root, sibling to `registry.yaml`. Schema lives at `specify-cli/schemas/sources/sources.schema.json`.

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/augentic/specify-cli/main/schemas/sources/sources.schema.json
version: 1
sources:
  - key: legacy-billing
    url: git@github.com:org/legacy-billing.git
    language: typescript
    description: 2018 billing monolith; subscription, invoicing, dunning.
    target_projects: [billing-svc]   # optional routing hint(s)
  - key: legacy-identity
    url: git@github.com:org/legacy-identity.git
    language: typescript
    description: User registration and authentication.
    target_projects: [identity-svc]
  - key: legacy-shared
    url: git@github.com:org/legacy-shared.git
    language: typescript
    description: Shared utilities used by the billing and identity monoliths.
    target_projects: [billing-svc, identity-svc]    # legitimate fan-out for shared sources
```

Schema rules:

| Field | Required | Notes |
|---|---|---|
| `version` | yes | `1` only. |
| `sources[].key` | yes | Kebab-case, unique within the file. Used as `<source-key>` everywhere (the `--source-key` positional on `/change:analyze`, the `<k>` segment in `analyze/<k>/metadata.json`, etc.). |
| `sources[].url` | yes | Same shape as `registry.yaml:projects[].url` — `.`, repo-relative path, `git@host:path`, `http(s)://`, `ssh://`, `git+http(s)://`, `git+ssh://`. Stored verbatim. |
| `sources[].language` | no | Free-form kebab-case (`typescript`, `python`, `csharp`, …). Advisory; surfaces in survey. |
| `sources[].description` | no | Single-line free text. |
| `sources[].target_projects` | no | Kebab-case names that should match `registry.yaml:projects[].name` when the registry exists. Routing hints, not bindings. Multiple values are legitimate for sources whose code splits across targets. |

`additionalProperties: false` everywhere; `serde(deny_unknown_fields)` in the Rust parser, mirroring [`Registry`](https://github.com/augentic/specify-cli/blob/main/crates/domain/src/registry/catalog.rs). Duplicate `key` values fail validation. Kebab-case violations fail validation.

> **Note.** The `status` field on `sources[]` (with values `pending` / `in-progress` / `migrated` / `abandoned`) lands in **RFC-22** alongside the migration ledger. Without a ledger, status would be writer-less and operator-maintained, and the framework's posture is to avoid hand-edited state. Operators who need an early signal can use `description` or wait for RFC-22.

### `specify sources` CLI verb family

A new top-level verb family, structurally parallel to `specify registry`:

```bash
specify sources add <key> --url <url> [--language <lang>] [--description "<desc>"] [--target-project <name>]...
specify sources remove <key>
specify sources show [<key>] [--format json]
specify sources list [--format json]                         # alias for `show` with no positional
specify sources validate [--format json]
specify sources sync [<key>]...                              # tier-1 cache materialisation
```

Behaviour:

- **`add`** — appends an entry. Fails on duplicate `key`. Validates kebab-case, URL shape, and `target_projects` references against `registry.yaml` when present (warning, not error, since target projects may be added later).
- **`remove`** — removes an entry. Fails if any *active* (non-archived) plan slice still references the key in its `sources[]` list. Also removes the matching cache entry under `.specify/.cache/sources/<key>/`.
- **`show` / `list`** — JSON-emitting reads, same envelope shape as `specify registry show`.
- **`validate`** — schema validation, duplicate-key detection, URL-shape checks, and (when `registry.yaml` exists) `target_projects` cross-reference warnings.
- **`sync`** — materialises tier-1 clones into the shared cache (see *Tier-1 caching* below). With no positional, syncs every entry; with one or more positionals, syncs only those keys.

The CLI exit-code surface mirrors `specify registry`: `0` success, `1` generic, `2` validation. New error discriminants (kebab-case): `sources-key-duplicate`, `sources-key-unknown`, `sources-url-invalid`, `sources-key-in-use`, `sources-target-project-unknown` (warning only when `registry.yaml` is present).

### Tier-1 caching at `.specify/.cache/sources/<key>/`

Today, tier-1 clones live at `.specify/plans/<change>/analyze/<key>/` (per-change, swept by `specify change plan archive`). This RFC moves the *clone* to a **durable, shared cache** at `.specify/.cache/sources/<key>/` and replaces the per-change directory with a **read-only symlink** into the cache.

Lifecycle:

| Verb | Effect |
|---|---|
| `specify sources sync` | Idempotent. For each entry: clone into `.specify/.cache/sources/<key>/` if missing; `git fetch` if present and remote; no-op for symlink/local URLs. |
| `/change:analyze` (legacy-code branch) | Reads from `.specify/.cache/sources/<key>/` via the per-change symlink. Writes nothing into the cache. |
| `specify change create` (when invoked with `--source @<key>`) | Materialises `.specify/plans/<change>/analyze/<key>/` as a symlink to `.specify/.cache/sources/<key>/`, calling `specify sources sync <key>` first if the cache slot is missing. |
| `specify change plan archive` | Sweeps `.specify/plans/<change>/analyze/<key>/` (the symlink) into the archive. Does **not** touch `.specify/.cache/sources/<key>/`. The cache outlives the change. |
| `specify sources remove <key>` | Removes the cache entry alongside the catalogue entry. Refuses if any active plan slice still references the key. |

The downstream `/change:analyze` skill reads through the symlink unchanged; its idempotency contract is unaffected. Archives preserve the symlink target by recording a small sidecar at `.specify/archive/plans/<date>-<name>/.snapshot.yaml`:

```yaml
version: 1
sources:
  - key: legacy-billing
    url: git@github.com:org/legacy-billing.git
    head_sha: 7c3f9a2b1d…       # commit SHA at the time the change was archived
    materialised_at: 2026-04-15  # date only
```

The `.snapshot.yaml` records what was on disk at archive time so audit value is preserved without copying gigabytes per change. Operators who want a full byte-snapshot of the source tree can run `git clone --shared` against the cache before archive; this is a deliberate operator opt-in, not the default.

This is a strict refinement of [`docs/explanation/workspace-tiers.md`](../docs/explanation/workspace-tiers.md): tier-1 now has a durable cache and a per-change symlink view, but the **role separation** (tier-1 = read-only analyze input; tier-2 = read-write executor target) is unchanged.

### `--source @<key>` selector

`/change:plan` (and any caller that consumes `--source`) accepts a new prefix-form:

| Form | Meaning |
|---|---|
| `--source <key>=<path-or-url>` | Existing inline form. The catalogue is not consulted. |
| `--source @<key>` | Resolve `<key>` against `sources.yaml`. Use the catalogue's URL, language, and target hints. |
| `--source @<key>:<kind>` | As above, with explicit kind override (`legacy-code`, `documentation`, `domain-model` per RFC-20). |

The closed-enum kind validation is unchanged. Unknown `<key>` against the catalogue is a hard exit (`sources-key-unknown`). The catalogue lookup is a CLI-side concern; downstream skills receive the resolved local path on disk, exactly as today.

This is what makes Scenario 2 tractable: declare 80 repos once via `specify sources add` (or generate from a manifest), then plan with `--source @legacy-billing --source @legacy-identity --source @legacy-shared` instead of three URL-bearing flags.

The RFC-20 `change.md:inputs[]` schema gains a sibling form too: an entry with `source_key: <key>` (instead of `path: …`) resolves through the catalogue. The closed-enum kind suffix remains the same.

### Scaling the analyze fan-out (`--analyze-concurrency`)

With `sources.yaml` and the shared tier-1 cache in place, `/change:plan`'s discovery brief (RFC-20 step 3a) gains a *parallel* fan-out: `/change:analyze` invocations per source are independent (they write to disjoint `analyze/<key>/` slots and append to a shared `discovery.md` whose merge contract is owned by `/change:analyze`). The brief may dispatch up to `--analyze-concurrency <N>` (default `4`, capped at `min(8, num_cpus)`) invocations concurrently.

The byte-stable output contract is unchanged: `/change:analyze` already sorts capability blocks alphabetically within each source-key block, and the discovery brief sorts source-key blocks alphabetically before flushing `discovery.md`. Concurrent invocations cannot produce non-deterministic output as long as each writes its own `<key>/` sidecar (which they already do) and the brief defers the final merge until all invocations have completed.

This is a brief-level change, not a CLI change; the existing `/change:analyze` contract is unmodified. The `--analyze-concurrency` knob lives on `/change:plan` (and is propagated through orchestration mode).

### CLI surface summary

Net adds:

- `specify sources` — `add`, `remove`, `show`, `list`, `validate`, `sync` (one new verb family).
- `--source @<key>` selector form on `/change:plan`.
- `--analyze-concurrency <N>` flag on `/change:plan`.

Net schema changes:

- New: `specify-cli/schemas/sources/sources.schema.json`.
- No changes to `plan.schema.json`, `registry.yaml` shape, or any existing JSON Schema.

No verb is renamed, retired, or repurposed. No existing schema field is changed in shape or required-ness.

### Scenario coverage

| Scenario | Pre-RFC-21 (assuming RFC-20 landed) | Post-RFC-21 |
|---|---|---|
| 1. Single-repo migration | Works; survey + synthesise via RFC-20. | Unchanged. `sources.yaml` optional with one entry; `--source @<key>` works the same as the inline form. |
| 2. Multi-repo migration (80+ repos) | Survey works, but every change re-clones every source; declarations repeat per change. | Sources declared once in `sources.yaml`; tier-1 cache shared across changes; analyze fans out concurrently. (Cross-change ledger and `mapping` field deferred to RFC-22.) |
| 3. Greenfield multi-repo | Domain-model-driven topology via RFC-20. | Unchanged; `sources.yaml` typically absent. |
| 4. Brownfield multi-repo | Routing via baseline + domain-model hints. | Unchanged; if legacy sources are involved, `sources.yaml` records them. |

## Implementation Plan

1. **Schema and validator.** Land `specify-cli/schemas/sources/sources.schema.json` and `specify-cli/schemas/sources/README.md`. Add a `Sources` validator in `specify-validate`.
2. **Domain types.** Add `Sources`, `SourceEntry` types in `specify-domain` (`crates/domain/src/sources/`). Mirror the `Registry` posture: `serde(deny_unknown_fields)`, `path()` / `load()` helpers, `validate_shape()`. `specify-error` gains `sources-*` discriminants.
3. **`specify sources` verb family.** Add `src/commands/sources/{cli,add,remove,show,list,validate,sync}.rs`. Each verb gets a JSON envelope mirroring `specify registry`. Land integration tests under `tests/sources.rs`.
4. **Tier-1 cache lifecycle.** Implement `.specify/.cache/sources/<key>/` materialisation in `specify sources sync`. Update `.gitignore` defaults (already covered by [`Registry::ensure_specify_gitignore_entries`](https://github.com/augentic/specify-cli/blob/main/crates/domain/src/registry/gitignore.rs); extend to add `.specify/.cache/`).
5. **Symlink view for `analyze/<key>/`.** When `/change:plan` resolves `--source @<key>`, materialise `.specify/plans/<change>/analyze/<key>/` as a symlink to the cache slot. Land integration tests for the symlink lifecycle.
6. **Archive snapshot.** Update `specify change plan archive` to write `.specify/archive/plans/<date>-<name>/.snapshot.yaml`. Define schema at `specify-cli/schemas/archive-snapshot/schema.json`.
7. **`--source @<key>` selector parsing.** Update `/change:plan` invocation grammar in [`plan-invocation.md`](../plugins/change/references/plan-invocation.md) and the CLI flag handler. Hard-fail on unknown keys.
8. **`change.md:inputs[].source_key` form.** Additive schema update to `brief/schema.json`. Update brief readers.
9. **`--analyze-concurrency` knob.** Brief-level change: update `discovery.md` brief to fan out via a small concurrency primitive. Default `4`. Document trade-offs (network bandwidth, CPU) in `references/runbook.md`.
10. **Tutorials and references.** New tutorial `docs/tutorials/multi-repo-legacy-migration.md`. Update `docs/explanation/workspace-tiers.md` to describe the shared tier-1 cache and the symlink view.
11. **Acceptance.** Extend the cross-repo Deno acceptance suite with: an N=10 multi-source plan using `--source @<key>` (asserting catalogue lookup, cache materialisation, symlink view, and analyze concurrency); a `specify sources remove` refusal when the key is in use.

## Migration

This RFC is **strictly additive**. Pre-existing plans, registries, changes, and `/change:analyze` invocations continue to work without change.

For operators:

- Continue using `--source <key>=<path-or-url>` if you prefer per-change declarations. Adopt `sources.yaml` only when you have more than a handful of legacy sources or want to avoid re-cloning.
- After upgrade, the tier-1 cache directory `.specify/.cache/sources/` will appear when you next run `specify sources sync` or a change with `--source @<key>`. Add `.specify/.cache/` to `.gitignore` (the `specify init` defaults will be updated to include it).
- The symlink view for `.specify/plans/<change>/analyze/<key>/` is transparent to `/change:analyze`; nothing changes for skill consumers.

For capability authors:

- No required changes. The discovery brief and `/change:analyze` skill contracts are unchanged.
- Survey briefs that want to read `sources.yaml` may do so; it is now a stable file under the platform-repo root. Existing survey briefs that ignore it continue to work.

For skill authors:

- `specify sources show --format json` is a new readable surface with a stable JSON envelope. Treat it like `specify registry show`.

There is **no breaking change** to: existing `plan.yaml` files, existing `registry.yaml` files, existing `/change:analyze` invocations, existing exit codes (new discriminants live within `EXIT_VALIDATION_FAILED=2` and `EXIT_GENERIC_FAILURE=1`), or existing archive layouts (the `.snapshot.yaml` file is additive inside the archive directory).

## Alternatives Considered

**Extend `registry.yaml` with a `sources:` block instead of a separate file.** Rejected. Sources and targets have different lifecycles, validation rules, materialisation strategies (read-only cache vs read-write working tree), and audiences (planner-time vs executor-time). Mixing them violates the registry's existing role and the workspace-tier separation.

**Promote `specify sources sync` into `specify workspace sync`.** Rejected. `workspace sync` materialises tier-2 (executor) clones into `.specify/workspace/`; `sources sync` materialises tier-1 (analyze) clones into `.specify/.cache/sources/`. Conflating them re-introduces the workspace-tier confusion that [`workspace-tiers.md`](../docs/explanation/workspace-tiers.md) was written to dispel.

**Snapshot the entire tier-1 clone into archives instead of recording a snapshot reference.** Rejected. With 80+ repos and frequent re-plans, copying gigabytes per archive is impractical. The recorded `.snapshot.yaml` (commit SHA, source URL, materialisation date) preserves the audit trail at constant cost; operators who genuinely need byte-snapshots can opt in by hand.

**Put the cache under `.specify/sources/` rather than `.specify/.cache/sources/`.** Rejected. The leading dot makes it clear the directory is framework-managed scratch (like `.specify/.cache/`, the existing capability resolver cache). Operators expect non-dot directories under `.specify/` to be authored or curated state.

**Auto-populate `sources.yaml` from a Backstage import.** Deferred to future RFC alignment with [RM-12 Catalog import: Backstage adapter](roadmap.md#rm-12-catalog-import-backstage-adapter). The shape of `sources.yaml` is consistent with that direction; the import path is orthogonal.

**Include a `status` field in `sources.yaml`.** Deferred to RFC-22. Without a ledger, status would be operator-maintained and writer-less, which the framework does not do for any other state. RFC-22 introduces the writers (`specify change plan transition done`, `specify change finalize`) that make `status` honest.

**Run analyze concurrency through a CLI verb (`specify analyze fan-out`).** Rejected. The existing `/change:analyze` skill is already per-input and idempotent; concurrency is a brief-level scheduling decision, not a new contract. The `--analyze-concurrency` flag on `/change:plan` is the natural place because the brief already orchestrates the fan-out.

## Non-Goals

- Cross-change durable state — the migration ledger (covered by RFC-22).
- A `mapping` field on plan slices (covered by RFC-22).
- A `status` field on `sources[]` entries (covered by RFC-22).
- Source-tree mutation (tier-1 stays read-only).
- Cross-platform-repo source sharing (sources are per-platform-repo).
- Backstage / external catalogue import (deferred; consistent shape with [RM-12](roadmap.md#rm-12-catalog-import-backstage-adapter)).
- Tier-1 cache eviction policies beyond `specify sources remove` (operators may delete `.specify/.cache/sources/<key>/` by hand if they need to).
- Driving execution from `sources.yaml` (the catalogue is read-only for every executor-side path).
- Parallel multi-plan output.

## Open Questions

1. Should `specify sources sync` accept `--depth <n>` for shallow clones? Current preference: yes, with a default of `1` for remotes (matching `specify workspace sync`'s posture for tier-2).
2. How should the tier-1 cache handle stale clones? Current preference: `specify sources sync` is `git fetch` for remotes (no merge, no rebase); operators get a warning if `HEAD` differs from the `head_sha` recorded in any open plan's `.snapshot.yaml`.
3. Should `--source @<key>` accept the kind suffix only, or fall back to `sources.yaml:sources[].language` to infer kind? Current preference: explicit suffix only; `language` is advisory.
4. Should `specify sources` validation check URL reachability? Current preference: no — keep validation offline; reachability surfaces during `specify sources sync`.
5. What is the `--analyze-concurrency` default and cap? Current preference: default `4`, hard cap `min(8, num_cpus)`. Revisit once the scaling acceptance suite has data.
6. Should `specify sources remove --force` exist for catalogue cleanup against archived plans? Current preference: no in v1; archive sweep already detaches the symlink, so `remove` only refuses on *active* plans.
7. Should the cache namespace `<key>` mangle URL identity (e.g., a checksum) so two catalogue entries with the same URL but different keys share storage? Current preference: no; keep `<key>` literal so removal is unambiguous.

## References

- [RFC-3a: Monoliths](archive/rfc-3a-monoliths.md) — analyze/extract split this RFC's tier-1 cache refines.
- [RFC-3b: Platform](archive/rfc-3b-platform.md) — `registry.yaml` posture this RFC's `sources.yaml` mirrors.
- [RFC-9: Platform](archive/rfc-9-platform.md) — orchestration umbrella; the new `--source @<key>` selector and `--analyze-concurrency` flag flow through it.
- [RFC-14: Workspace](archive/rfc-14-workspace.md) — workspace-tier separation this RFC preserves while adding tier-1 caching.
- [RFC-20: Survey-to-Plan Pipeline](rfc-20-survey.md) — survey, synthesise, and assignment briefs that consume the new catalogue.
- [RM-12: Catalog import — Backstage adapter](roadmap.md#rm-12-catalog-import-backstage-adapter) — long-term shape alignment for source catalogue import.
- [`docs/explanation/workspace-tiers.md`](../docs/explanation/workspace-tiers.md) — tier-1 / tier-2 boundary the cache refinement preserves.
- [`docs/tutorials/legacy-migration-at-scale.md`](../docs/tutorials/legacy-migration-at-scale.md) — the canonical multi-source migration walkthrough this RFC updates.
- [`crates/domain/src/registry/catalog.rs`](https://github.com/augentic/specify-cli/blob/main/crates/domain/src/registry/catalog.rs) — reference implementation for the `Registry` posture `Sources` mirrors.
