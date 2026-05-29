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

At propose time, reconcile leads across sources into one slice row:

```bash
specrun plan add <slice> --sources legacy=<lead-id> --sources docs=<lead-id>
```

Single-source intent slices may use the shorthand `sources: [intent]`.

## Uncertain reconciliation

When leads align loosely, `/spec:plan` may annotate `tentative: true` in `discovery.md` and add a `## Tentative merges` section to `change.md`. Review at Gate 1; amend before stamping `approved`.

When summaries materially disagree, the plan skill may stamp `divergence: likely` on affected slice rows.

## See also

- [/spec:plan](../reference/change-skills/plan.md) — source binding grammar
- [Legacy migration at scale](../tutorials/legacy-migration-at-scale.md) — code-typescript orientation
- [Resolve spec conflicts](resolve-spec-conflicts.md) — after refine surfaces tags
