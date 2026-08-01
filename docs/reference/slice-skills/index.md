# Slice skills

Slice skills operate on a single slice inside `.emery/slices/<name>/`. They cover one-time project setup and the per-slice refine → build → merge loop. The change level ([Change skills](../change-skills/index.md), [emery plan execute](../cli/plan.md#emery-plan-execute)) sequences the same orchestrations inside `emery plan execute`; every step is also reachable as a manual breakout when execute parks.

## The per-slice loop

```text
/emery:init  →  (plan-time)  →  /emery:refine  →  /emery:build  →  /emery:merge
```

`/emery:init` is one-time scaffolding. The loop runs inside `emery plan execute`, but each phase is invokable by hand. See [Drive a slice manually](../../how-to/drive-slice-manually.md).

**Canonical reference.** The authoritative operator surface for every skill — synopsis, arguments, the step-by-step critical path, guardrails, closing hints, and error modes — is its canonical skill body under [`plugins/emery/skills/<phase>/SKILL.md`](../../../plugins/emery/README.md). The sections below are navigation entries and carry no operator steps, so the two surfaces cannot drift.

## Skill summary

| Skill | Purpose | Reads | Writes |
| ----- | ------- | ----- | ------ |
| [/emery:init](#emeryinit) | One-time project setup | — | `.emery/`, `project.yaml`, cache, `AGENTS.md` |
| [/emery:refine](#emeryrefine) | Extract per source, synthesize artifacts | Plan bindings, discovery, sources | Slice artifacts, Evidence, `model.yaml` |
| [/emery:build](#emerybuild) | Validate artifacts, implement tasks | Slice artifacts, target build prompts | Source code, task checkmarks |
| [/emery:merge](#emerymerge) | Apply slice deltas to baseline, archive slice | Slice specs, baseline | Updated baseline, archived slice, per-entry `done` |
| [/emery:drop](#emerydrop) | Discard a slice without merging | Slice metadata | Archived slice (dropped) |

## /emery:init

Initialise Emery in a project. Run once before any other `/emery:` skill. Canonical body: [`/emery:init`](../../../plugins/emery/skills/init/SKILL.md).

See also: [Prerequisites](../../orientation/prerequisites.md) — what to install before init · [Directory layout](../directory-layout.md) — what init creates · [Configuration files](../configuration.md) — `project.yaml` format.

## /emery:refine

Refine a plan entry's slice — invoke `emery slice refine`, which runs extract per bound source, synthesizes proposal, spec, design, and tasks, validates, and transitions to `refined`. Canonical body: [`/emery:refine`](../../../plugins/emery/skills/refine/SKILL.md); what the agent writes into the synthesis response is owned by the [synthesis playbook](../../../crates/slice/prompts/synthesize.md).

See also: [Resolve spec conflicts](../../how-to/resolve-spec-conflicts.md) — `[conflict]` and `[divergence]` tags · [Artifact format](../artifact-format.md) — requirement block shape · [Lifecycle](../lifecycle.md) — slice state machine.

## /emery:build

Implement tasks from a refined slice by invoking `emery slice build`, which drives the target adapter's build operation. Canonical body: [`/emery:build`](../../../plugins/emery/skills/build/SKILL.md).

See also: [Drive a slice manually](../../how-to/drive-slice-manually.md) — when execute parks on build · [Artifact format](../artifact-format.md) — skill directive tag syntax.

## /emery:merge

Merge a built slice into the baseline — apply spec deltas, archive the slice, stamp the plan entry `done`. Canonical body: [`/emery:merge`](../../../plugins/emery/skills/merge/SKILL.md).

See also: [Lifecycle](../lifecycle.md) — merged state and archiving · [Directory layout](../directory-layout.md) — archive paths.

## /emery:drop

Discard a slice without merging specs into the baseline. The alternative to [/emery:merge](#emerymerge). Canonical body: [`/emery:drop`](../../../plugins/emery/skills/drop/SKILL.md).

See also: [Lifecycle](../lifecycle.md) — the dropped state.

## How skills delegate

Each skill is an ultrathin invoke-and-relay wrapper over one guest-routed `emery` verb. See [AGENTS.md § Skill / CLI responsibility split](../../../AGENTS.md) for the contract and each phase's [`SKILL.md`](../../../plugins/emery/README.md) for the authoritative steps.

## See also

- [Change skills](../change-skills/index.md) — plan, execute, finalize
- [Lifecycle](../lifecycle.md) — slice and per-entry state machines
- [Quick reference card](../quick-reference.md) — all skills at a glance
