# How-to guides

Task-oriented recipes for common Emery operator situations. Each guide assumes you completed the [Quick start](../tutorials/quick-start.md) unless noted otherwise.

| Guide | When to use it |
| ----- | -------------- |
| [Drop down a layer](drop-down-a-layer.md) | Automation failed and you need manual CLI control |
| [Drive a slice manually](drive-slice-manually.md) | `emery plan execute` [parked](../appendices/glossary.md#p) mid-loop |
| [Amend a plan at Gate 1](amend-plan-at-gate-1.md) | Inspect or edit the plan before executing it ([Gate 1](../appendices/glossary.md#g)) |
| [Undo a plan entry](undo-a-plan-entry.md) | An entry's status is ahead of reality — walk it back a rung |
| [Drop a slice](drop-a-slice.md) | A slice should not land — archive it without merging |
| [Resolve spec conflicts](resolve-spec-conflicts.md) | `[conflict]` or `[divergence]` tags in `spec.md` |
| [Interpret validate findings](interpret-validate-findings.md) | A validate verb exited 2, or execute reports `stuck` |
| [Bind multiple sources](bind-multiple-sources.md) | Reconcile legacy code and documentation at plan time |
| [Recover from a stale guest lock](recover-from-a-stale-guest-lock.md) | `guest-marker-held` with no live driver session |
| [Upgrade adapters](upgrade-adapters.md) | Pick up a newer published adapter version |

For conceptual background, see [Understanding Emery](../explanation/concepts.md). For precise flags and schemas, see [Reference](../reference/index.md). Unfamiliar terms are defined in the [Glossary](../appendices/glossary.md).
