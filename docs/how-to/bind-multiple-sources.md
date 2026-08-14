# Bind multiple sources

Reconcile evidence from more than one source adapter at plan time.

**Prerequisites:** Completed [Quick start](../tutorials/quick-start.md).

## Basic syntax

Sources arrive through the reviewed handoff (`/emery:plan` elicits `--from` / `--wave`). Each imported surface lead in that handoff becomes a slot in `plan.yaml.sources` and contributes [leads](../appendices/glossary.md#l) to `leads.md`. Bind re-resolves the coverage locator and pins the delivery CID; a handoff `observed-cid` is imported provenance only.

Intent is the reserved key with an inline `value`; other sources carry a locator and a CID. There is no `--intent` or `--source` authoring flag.

## Anatomy of a binding

Every bound source is a row under `plan.yaml.sources.<key>`:

- **`<key>`** — a label assigned at binding (kebab-case, e.g. `legacy`, `docs`, reserved `intent`). It becomes the slot name that plan entries and evidence files (`evidence/<key>.yaml`) reference.
- **`adapter`** — the exact pin (`emery:typescript@0.12.0`).
- **`locator` or `value`** — a location-backed CID view, or inline text (used by `intent`).

## Binding forms

| Form | Example | Use when |
| ---- | ------- | -------- |
| Locator | `docs` → `emery:documentation@…` + path/git CID | Filesystem or repository tree |
| Value | reserved `intent` + inline string | Operator intent from the handoff |
| Code | `legacy` → `emery:typescript@…` + tree CID | Legacy codebase survey |

## Multi-source slices

At plan-authoring time, one verb binds the handoff, surveys every source, decomposes the catalog, and writes the slice table:

```bash
emery plan author <name> --from <definition-home> --wave <id>
```

Every slice row carries a required `target` key in `plan.yaml.targets`.

## Uncertain reconciliation

When cross-source grouping is uncertain, `/emery:plan` adds a `## Tentative merges` section to `change.md` (not `leads.md`). Review it and amend before running `emery plan refine`.

When summaries materially disagree on a merged slice, the plan skill adds `## Likely divergences` to `change.md` and invokes `emery plan amend <entry> --divergence likely`.

## Cross-cutting leads

Coverage is at-least-once: a lead may be bound into more than one slice. When a lead is guidance that informs several work leads (e.g. a conventions document), decomposition multi-homes it — binds the same lead into each slice it informs — and lists it in `change.md` under `## Cross-cutting leads`. A slice still carries at most one lead per source — the CLI rejects a duplicate key at decompose (`plan-reconcile-slice-source-collision`), `plan validate`, and `plan amend` (`duplicate-source-key`).

## See also

- [`/emery:plan` skill body](../../plugins/emery/skills/plan/SKILL.md) — `--from` / `--wave` authoring
- [Legacy migration at scale](../explanation/legacy-migration.md) — typescript orientation
- [Resolve spec conflicts](resolve-spec-conflicts.md) — after refine surfaces tags
