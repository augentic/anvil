# Bind multiple sources

Reconcile evidence from more than one source adapter at plan time.

**Prerequisites:** Completed [Quick start](../tutorials/quick-start.md).

## Basic syntax

Append multiple `source` positionals after the change name:

```text
/spec:plan identity-revamp source legacy=code-typescript:./vendor/monolith source docs=documentation:./design-notes
```

Each binding creates a slot in `plan.yaml.sources` and contributes leads to `discovery.md`.

## Binding forms

| Form | Example | Use when |
| ---- | ------- | -------- |
| Path binding | `docs=documentation:./design-notes` | Filesystem tree |
| Value binding | `intent=intent:value:fix typo in user.rs` | Inline operator intent |
| Code binding | `legacy=code-typescript:./src` | Legacy codebase survey |

## Multi-source slices

At propose time, reconcile leads across sources through the D2 envelope:

```bash
specrun plan propose --dry-run --format json
# agent authors response.json
specrun plan propose --from response.json
```

Single-source intent slices may omit `project` when only one project exists; the kernel auto-binds and normalises `sources: [intent]`.

## Uncertain reconciliation

When cross-source grouping is uncertain, `/spec:plan` adds a `## Tentative merges` section to `change.md` (not `discovery.md`). Review at Gate 1; amend before stamping `approved`.

When summaries materially disagree on a merged slice, the plan skill adds `## Likely divergences` to `change.md` and invokes `specrun plan amend <plan> <slice> --divergence likely`.

## See also

- [/spec:plan](../reference/change-skills/plan.md) — source binding grammar
- [Legacy migration at scale](../tutorials/legacy-migration-at-scale.md) — code-typescript orientation
- [Resolve spec conflicts](resolve-spec-conflicts.md) — after refine surfaces tags
