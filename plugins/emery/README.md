# Emery Workflow

Emery routes workflow state changes through `/emery:*` wrappers over the `emery` CLI. `/emery:init` scaffolds a project. `/emery:plan` authors `change.md` + `plan.yaml` through `emery plan author` and exits so the operator can review them. `/emery:execute` runs `emery plan execute` — opens the `plan.execute.started` authorization epoch and drives the per-slice refine → build → merge loop under gap gates; when the loop stops, fixing the reported problem and re-running execute is the resume path. `/emery:status` is the read-only orientation probe over computed progress (Ready / Authorized, never `approved`). Repository publication (commit, push, review, merge) is operator-owned outside Emery; `/emery:finalize` confirms publication is complete, then archives the plan.

Every skill is an ultrathin invoke-and-relay wrapper: it elicits any missing arguments, invokes the corresponding `emery` command, and relays the output verbatim. Workflow sequencing, artifact synthesis, and validation all live in the CLI's orchestrations.

Display ownership is split three ways: the CLI's rendered output owns facts and navigation (phase lines, stop cards, resume commands, the canonical drained line); skills own only argument elicitation and the confirmation gates (plan replace, publication); live tracing is a stderr side channel — each skill selects it with the reserved log flags (bare for the long-running orchestrations' INFO progress, `--quiet` for probes, `--debug` for extra runtime and backend tracing on request) per the plugin rule's *Tracing and output* contract, and never repeats tracing lines or composes replacement summaries.

## Skills

| Skill | Command | Description |
|-------|---------|-------------|
| [init](skills/init/SKILL.md) | `/emery:init` | Initialize Emery in a project (`emery init`) |
| [plan](skills/plan/SKILL.md) | `/emery:plan` | Author `change.md` + `plan.yaml`, stop for operator review (`emery plan author`) |
| [execute](skills/execute/SKILL.md) | `/emery:execute` | Drive the per-slice loop to completion (`emery plan execute` — opens the authorization epoch) |
| [status](skills/status/SKILL.md) | `/emery:status` | Report where the plan stands and the literal next command (`emery plan status`, read-only) |
| [finalize](skills/finalize/SKILL.md) | `/emery:finalize` | Confirm publication is complete, then archive the plan (commits, pushes, and PRs are operator-owned, outside Emery) |

The judgment prose the workflow's model legs consume (lead reconciliation, the synthesis playbook, spec formatting) is embedded in the `emery` binary from `crates/slice/prompts/` and `crates/change/prompts/`; it is not plugin material.
