<div class="hero">
<div class="eyebrow">Reference</div>
<h1 class="hero-title">/spec:plan</h1>

Survey bound sources, reconcile leads into `slices[]`, validate the plan, and exit at Gate 1 (`pending`).

<div class="meta-row">

<span class="meta-chip"><strong>Layer</strong> 2 — Change</span>

<span class="meta-chip"><strong>Writes</strong> change.md, plan.yaml, discovery.md</span>

<span class="meta-chip"><strong>Gate</strong> Exits pending</span>

</div>

</div>


<div class="synopsis">
<strong>Synopsis.</strong>


```text
/spec:plan <name> [source <key>=<adapter>:<binding> ...]
```

Agent-driven orchestrator. Deterministic work delegates to `specrun plan *` and `specrun source resolve`. Never writes `approved`.

</div>


## Arguments

| Argument | Required | Description |
| -------- | -------- | ----------- |
| `<name>` | Yes | Kebab-case change name. Becomes `plan.yaml.name`. |
| `source <key>=<adapter>:<binding>` | No | Repeatable source binding. When omitted, the skill elicits a one-line intent and scaffolds with `intent:value:<literal>`. |

### Source binding grammar

```text
source <key>=<adapter>:<path>
source <key>=<adapter>:value:<literal>
```

- `<key>` — kebab-case slot in `plan.yaml.sources.<key>`.
- `<adapter>` — source adapter name (`intent`, `documentation`, `code-typescript`, `screenshots`, …).
- `<binding>` — filesystem path (e.g. `documentation:./design-notes`) or literal form `value:<text>` (e.g. `intent:value:fix typo in user.rs`).

## When to use

- Starting a fresh change from one or more bound sources (or pure intent).
- Re-surveying sources on an existing plan (same-source lead ids replace in place).

Not for continuing an already-approved plan into execution — use [/spec:execute](execute.md).

## Artifacts produced

| Artifact | Location | Content |
| -------- | -------- | ------- |
| Change narrative | `.specify/change.md` | Operator-facing intent, scope, tentative merges |
| Plan | `.specify/plan.yaml` | Sources, `slices[]` rows, `lifecycle: pending` |
| Discovery | `.specify/discovery.md` | Summary, source inventory, lead inventory |

## Behavior

1. **Pre-flight** — validate `<name>` as kebab-case; read `.specify/project.yaml`.
2. **Scaffold** — `specrun plan create <name> --source <key>=<adapter>:<binding> …` writes `change.md` and `plan.yaml` atomically.
3. **Workspace sync** (workspace plans only) — `specrun workspace sync` before survey.
4. **Survey each source** — run each source adapter's `survey` brief; append lead blocks to `discovery.md`.
5. **Write `discovery.md`** — three sections: Summary, Source inventory, Lead inventory.
6. **Propose** — reconcile leads into `slices[]` via `specrun plan add`; annotate tentative merges and `divergence: likely` when reconciliation is uncertain.
7. **Validate** — `specrun plan validate --format json` when multi-slice or workspace plans need doctor output.
8. **Exit at `pending`** — print the closing hint; never call `specrun plan transition`.

A one-slice change uses the same steps as a twelve-slice change: `intent.survey` produces one lead and one slice row.

### Closing hint

```text
Plan `<name>` is at `pending`. Run `specrun plan transition <name> approved` to stamp Gate 1, then `/spec:execute` to drive the slices.
```

The skill never auto-stamps `approved`. The operator runs the literal transition command after inspecting the plan.

## CLI delegation

| Operation | CLI verb |
| --------- | -------- |
| Scaffold plan | `specrun plan create` |
| Reconcile leads → slices | `specrun plan propose --from` |
| Add slice row | `specrun plan add` |
| Amend entry (scalpel) | `specrun plan amend <entry>` |
| Remove entry (defer) | `specrun plan remove <entry>` |
| Validate plan | `specrun plan validate` |

## Error modes

| Error | Cause | Resolution |
| ----- | ----- | ---------- |
| Malformed change name | Non-kebab-case `<name>` | Use kebab-case identifiers |
| Workspace plan active | `/spec:plan` from project root while workspace plan is active | Follow CLI structured error; plan from workspace root |
| Registry invalid | Malformed `registry.yaml` in workspace mode | Fix registry before sync |

## Examples

```text
# Pure intent, one slice (N=1)
/spec:plan fix-typo source intent="fix typo in user.rs"

# Documentation-backed change
/spec:plan account-revamp source docs=documentation:./design-notes/account

# Multi-source at plan time
/spec:plan identity-revamp source legacy=code-typescript:./vendor/monolith source docs=documentation:./design-notes
```

<div class="see-also">
<strong>See also</strong>

- [Amend a plan at Gate 1](../../how-to/amend-plan-at-gate-1.md) — inspect and edit before stamping `approved`
- [Bind multiple sources](../../how-to/bind-multiple-sources.md) — source binding patterns
- [specrun plan](../cli/plan.md) — CLI reference
- [Quick start tutorial](../../tutorials/quick-start.md) — hands-on first change
</div>

