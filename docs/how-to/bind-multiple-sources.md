# Bind multiple sources

Reconcile evidence from more than one source adapter at plan time.

**Prerequisites:** Completed [Quick start](../tutorials/quick-start.md).

## Basic syntax

Append multiple `source` positionals after the change name:

```text
/emery:plan identity-revamp source legacy=typescript:./vendor/monolith source docs=documentation:./design-notes
```

Each binding creates a slot in `plan.yaml.sources` and contributes [leads](../appendices/glossary.md#l) to `discovery.md`.

## Binding forms

| Form | Example | Use when |
| ---- | ------- | -------- |
| Path binding | `docs=documentation:./design-notes` | Filesystem tree |
| Value binding | `intent=intent:value:fix typo in user.rs` | Inline operator intent |
| Code binding | `legacy=typescript:./src` | Legacy codebase survey |

## Multi-source slices

At plan-authoring time, one verb surveys every source, reconciles the leads, and writes the slice table:

```bash
emery plan author
```

Single-source intent slices may omit `project` when only one project exists; the kernel auto-binds and normalises `sources: [intent]`.

## Uncertain reconciliation

When cross-source grouping is uncertain, `/emery:plan` adds a `## Tentative merges` section to `change.md` (not `discovery.md`). Review at [Gate 1](../appendices/glossary.md#g); amend before running `emery plan execute`.

When summaries materially disagree on a merged slice, the plan skill adds `## Likely divergences` to `change.md` and invokes `emery plan amend <entry> --divergence likely`.

## Cross-cutting leads

Coverage is at-least-once: a lead may be bound into more than one slice. When a lead is guidance that informs several work leads (e.g. a conventions document), the propose step multi-homes it — binds the same lead into each slice it informs — and lists it in `change.md` under `## Cross-cutting leads`. A slice still carries at most one lead per source — the CLI rejects a duplicate key at propose (`plan-reconcile-slice-source-collision`), `plan validate`, and `plan amend` (`duplicate-source-key`).

## See also

- [`/emery:plan` skill body](../../plugins/emery/skills/plan/SKILL.md) — source binding grammar
- [Legacy migration at scale](../explanation/legacy-migration.md) — typescript orientation
- [Resolve spec conflicts](resolve-spec-conflicts.md) — after refine surfaces tags
