# Emery Workflow

Emery routes workflow state changes through `/emery:*` wrappers over the `emery` CLI. `/emery:init` scaffolds a project. When an engagement starts from an existing estate, the RFC-104 definition loop runs upstream over a hand-authored definition home: `/emery:system-survey` recovers the `as-is` architecture, `/emery:system-plan` proposes the target and projects wave handoffs, and `/emery:system-review` records architectural authority over one exact handoff. `/emery:plan` authors the change from that reviewed handoff (`--from` / `--wave`) through `emery plan author` and exits so the operator can review topology. Later skills inherit the Cursor workspace cwd as the change root and may elicit `--change-dir`. `/emery:refine` runs `emery plan refine` — the serial refinement drain that extracts, synthesizes, and writes each slice's refinement manifest before any code work; when the drain stops, fixing the reported problem and re-running refine is the resume path. `/emery:execute` runs `emery plan execute` — opens the `plan.execute.started` authorization epoch over the exact refinement digests and drives the per-slice build → merge loop under gap gates; when the loop stops, fixing the reported problem and re-running execute is the resume path. `/emery:status` is the read-only orientation probe over computed progress (Ready / Authorized, never `approved`). Repository publication (commit, push, review, merge) is operator-owned outside Emery — the execute drain materializes each publication member's `change/<plan>` worktree, and the operator commits with both `Emery-Change` trailers, pushes, and opens the pull requests; `/emery:finalize` confirms publication is complete, then archives the plan (`emery plan archive` verifies the publication set against the forge and refuses `publication-unverified` until every member has merged; `--unverified` is the journaled bypass).

Every skill is an ultrathin invoke-and-relay wrapper: it elicits any missing arguments, invokes the corresponding `emery` command, and relays the output verbatim. Workflow sequencing, artifact synthesis, and validation all live in the CLI's orchestrations.

Display ownership is split three ways: the CLI's rendered output owns facts and navigation (phase lines, stop cards, resume commands, the canonical drained line); skills own only argument elicitation and the confirmation gates (plan replace, publication); live tracing is a stderr side channel — each skill selects it with the reserved log flags (bare for the long-running orchestrations' INFO progress, `--quiet` for probes, `--debug` for extra runtime and backend tracing on request) per the plugin rule's *Tracing and output* contract, and never repeats tracing lines or composes replacement summaries.

## Skills

| Skill | Command | Description |
|-------|---------|-------------|
| [init](skills/init/SKILL.md) | `/emery:init` | Initialize Emery in a project (`emery init`) |
| [plan](skills/plan/SKILL.md) | `/emery:plan` | Author from a reviewed handoff (`--from` / `--wave`), stop for operator review (`emery plan author`) |
| [refine](skills/refine/SKILL.md) | `/emery:refine` | Drain specification refinement over the closed plan, stop before code work (`emery plan refine`) |
| [execute](skills/execute/SKILL.md) | `/emery:execute` | Drive the per-slice build → merge loop to completion (`emery plan execute` — opens the authorization epoch) |
| [status](skills/status/SKILL.md) | `/emery:status` | Report where the plan stands and the literal next command (`emery plan status`, read-only) |
| [finalize](skills/finalize/SKILL.md) | `/emery:finalize` | Confirm publication is complete, then archive the plan — archive verifies each member's pull request on the forge (commits, pushes, and PRs are operator-owned, outside Emery) |
| [system-survey](skills/system-survey/SKILL.md) | `/emery:system-survey` | Survey a definition home's declared coverage and correlate the `as-is` architecture (`emery system survey`) |
| [system-plan](skills/system-plan/SKILL.md) | `/emery:system-plan` | Propose the initial architecture once, then reproject views and wave handoffs (`emery system plan`) |
| [system-review](skills/system-review/SKILL.md) | `/emery:system-review` | Record architectural authority over one exact wave handoff (`emery system review`) |

The judgment prose the workflow's model legs consume (lead reconciliation, the synthesis playbook, spec formatting) is embedded in the `emery` binary from `crates/slice/prompts/` and `crates/change/prompts/`; it is not plugin material.
