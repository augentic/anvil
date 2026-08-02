# Emery Workflow

Emery routes workflow state changes through `/emery:*` wrappers over the `emery` CLI. `/emery:init` scaffolds a project. `/emery:plan` authors `change.md` + `plan.yaml` through `emery plan author` and exits so the operator can review them. `/emery:execute` runs `emery plan execute` — running it is the operator's approval of the authored plan — and drives the per-slice loop to completion through that one verb. `/emery:status` is the read-only orientation probe. The per-slice breakouts — `/emery:refine`, `/emery:build`, `/emery:merge`, `/emery:drop` — invoke the matching `emery slice` verbs one slice at a time. Repository publication (commit, push, review, merge) is operator-owned outside Emery; `/emery:finalize` confirms publication is complete, then archives the plan.

Every skill is an ultrathin invoke-and-relay wrapper: it elicits any missing arguments, invokes the corresponding `emery` command, and relays the output verbatim. Workflow sequencing, artifact synthesis, and validation all live in the CLI's orchestrations.

Display ownership is split three ways: the CLI's rendered output owns facts and navigation (phase lines, stop cards, resume commands, the canonical drained line); skills own only argument elicitation and the confirmation gates (plan replace, merge, drop, publication); live tracing is a stderr side channel — each skill selects it with the reserved log flags (bare for the long-running orchestrations' INFO progress, `--quiet` for probes, `--debug` for extra runtime and backend tracing on request) per the plugin rule's *Tracing and output* contract, and never repeats tracing lines or composes replacement summaries.

## Skills

| Skill | Command | Description |
|-------|---------|-------------|
| [init](skills/init/SKILL.md) | `/emery:init` | Initialize Emery in a project (`emery init`) |
| [plan](skills/plan/SKILL.md) | `/emery:plan` | Author `change.md` + `plan.yaml`, stop for operator review (`emery plan author`) |
| [execute](skills/execute/SKILL.md) | `/emery:execute` | Drive the per-slice loop to completion (`emery plan execute` — running it is the approval) |
| [status](skills/status/SKILL.md) | `/emery:status` | Report where the plan stands and the literal next command (`emery plan status`, read-only) |
| [refine](skills/refine/SKILL.md) | `/emery:refine` | Extract, synthesize, validate, and transition one slice to `refined` (`emery slice refine`) |
| [build](skills/build/SKILL.md) | `/emery:build` | Build one slice through its target adapter (`emery slice build`) |
| [merge](skills/merge/SKILL.md) | `/emery:merge` | Fold one slice's deltas into the baseline and archive it (`emery slice merge`) |
| [drop](skills/drop/SKILL.md) | `/emery:drop` | Discard a slice without merging (`emery slice drop`) |
| [finalize](skills/finalize/SKILL.md) | `/emery:finalize` | Confirm publication is complete, then archive the plan (commits, pushes, and PRs are operator-owned, outside Emery) |

The judgment prose the workflow's model legs consume (lead reconciliation, the synthesis playbook, spec formatting) is embedded in the `emery` binary from `crates/slice/prompts/` and `crates/change/prompts/`; it is not plugin material.
