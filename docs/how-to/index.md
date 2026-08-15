# How-to guides

Task-oriented recipes for common Emery operator situations. Each guide assumes you completed the [Quick start](../tutorials/quick-start.md) unless noted otherwise.

| Guide | When to use it |
| ----- | -------------- |
| [Drop down a layer](drop-down-a-layer.md) | Automation failed and you need manual CLI control |
| [Amend a plan before executing](amend-a-plan.md) | Inspect or edit the plan before executing it |
| [Resolve spec conflicts](resolve-spec-conflicts.md) | `[conflict]` or `[divergence]` tags in `spec.md` |
| [Interpret validate findings](interpret-validate-findings.md) | A validate verb exited 2, or execute reports `stuck` |
| [Bind multiple sources](bind-multiple-sources.md) | Reconcile legacy code and documentation at plan time |
| [Recover from a stale guest lock](recover-from-a-stale-guest-lock.md) | `guest-marker-held` with no live driver session |
| [Upgrade adapters](upgrade-adapters.md) | Pick up a newer published adapter version |
| [Publish a change](publish-a-change.md) | Commit and push a materialized publication worktree |

For conceptual background, see [Understanding Emery](../explanation/concepts.md). For precise flags and schemas, see [Reference](../reference/index.md). Unfamiliar terms are defined in the [Glossary](../appendices/glossary.md).
