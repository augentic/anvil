# Initiative Skills (Layer 3)

Layer 3 skills coordinate multi-change programs through `.specify/plan.yaml`. They sit above the [change lifecycle skills](../change-skills/index.md) and invoke them per-change.

## The plan-execute flow

```text
/spec:plan <name> --source legacy=./path  -->  /spec:execute --loop
```

`/spec:plan` produces the plan. `/spec:execute` consumes it by running the define-build-merge loop for each change in dependency order.

## Skill summary

| Skill | Purpose | Reads | Writes |
|-------|---------|-------|--------|
| [/spec:plan](plan.md) | Author `plan.yaml` from inputs | Sources, docs, registry, baseline specs | `plan.yaml`, `discovery.md`, `proposal.md`, optional `workspace.md`; for multi-project plans, amends entries with `--project` via the assignment step |
| [/spec:execute](execute.md) | Drive the plan through define-build-merge | `plan.yaml` | Plan status transitions (via CLI); CWD-routes into workspace clones for multi-project plans; merge may auto-commit `.specify/` in clones |
| [/spec:analyze](analyze.md) | Plan-time capability inference | Source code or documentation | `discovery.md`, optional `metadata.json` |

## Three layers, independently useful

Layer 3 skills are optional. You can use the define-build-merge loop without ever touching plans. But when you do need them, they compose with the lower layers:

- **Layer 3 alone (`/spec:plan`)** -- author a plan, then drive it manually with Layer 1 CLI commands.
- **Layer 3 + Layer 2 (`/spec:plan` then `/spec:execute`)** -- author a plan, then automate execution.
- **Layer 2 alone** -- skip plans entirely, define and build changes one at a time.

The Layer 1 CLI commands (`specify plan ...`) remain available as manual fallback at every level.
