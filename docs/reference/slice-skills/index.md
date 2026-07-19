# Slice skills

Slice skills operate on a single slice inside `.specify/slices/<name>/`. They cover one-time project setup and the per-slice refine → build → merge loop. The change level ([/spec:plan](../change-skills/plan.md), [specify plan execute](../cli/plan.md#specify-plan-execute), [/spec:finalize](../change-skills/finalize.md)) sequences the same orchestrations inside `specify plan execute`; every step is also reachable as a manual breakout when execute parks.

## The per-slice loop

```text
/spec:init  →  (plan-time)  →  /spec:refine  →  /spec:build  →  /spec:merge
```

`/spec:init` is one-time scaffolding. The loop runs inside `specify plan execute`, but each phase is invokable by hand. See [Drive a slice manually](../../how-to/drive-slice-manually.md).

**Canonical reference.** The authoritative operator surface for every skill — synopsis, arguments, the step-by-step critical path, guardrails, closing hints, and error modes — is its canonical skill body under [`plugins/spec/skills/<phase>/SKILL.md`](../../../plugins/spec/README.md). The sections below are navigation entries and carry no operator steps, so the two surfaces cannot drift.

## Skill summary

| Skill | Purpose | Reads | Writes |
| ----- | ------- | ----- | ------ |
| [/spec:init](#specinit) | One-time project setup | — | `.specify/`, `project.yaml`, cache, `AGENTS.md` |
| [/spec:refine](#specrefine) | Extract per source, synthesize artifacts | Plan bindings, discovery, sources | Slice artifacts, Evidence, `model.yaml` |
| [/spec:build](#specbuild) | Validate artifacts, implement tasks | Slice artifacts, target build prompts | Source code, task checkmarks |
| [/spec:merge](#specmerge) | Apply slice deltas to baseline, archive slice | Slice specs, baseline | Updated baseline, archived slice, per-entry `done` |
| [/spec:drop](#specdrop) | Discard a slice without merging | Slice metadata | Archived slice (dropped) |

## /spec:init

Initialise Specify in a project. Run once before any other `/spec:` skill. Canonical body: [`/spec:init`](../../../plugins/spec/skills/init/SKILL.md).

See also: [Prerequisites](../../orientation/prerequisites.md) — what to install before init · [Directory layout](../directory-layout.md) — what init creates · [Configuration files](../configuration.md) — `project.yaml` format.

## /spec:refine

Refine a plan entry's slice — invoke `specify slice refine`, which runs extract per bound source, synthesizes proposal, spec, design, and tasks, validates, and transitions to `refined`. Canonical body: [`/spec:refine`](../../../plugins/spec/skills/refine/SKILL.md); what the agent writes into the synthesis response is owned by the [synthesis playbook](../../../crates/slice/prompts/synthesize.md).

See also: [Resolve spec conflicts](../../how-to/resolve-spec-conflicts.md) — `[conflict]` and `[divergence]` tags · [Artifact format](../artifact-format.md) — requirement block shape · [Lifecycle](../lifecycle.md) — slice state machine.

## /spec:build

Implement tasks from a refined slice by invoking `specify slice build`, which drives the target adapter's build operation. Canonical body: [`/spec:build`](../../../plugins/spec/skills/build/SKILL.md).

See also: [Drive a slice manually](../../how-to/drive-slice-manually.md) — when execute parks on build · [Artifact format](../artifact-format.md) — skill directive tag syntax.

## /spec:merge

Merge a built slice into the baseline — apply spec deltas, archive the slice, stamp the plan entry `done`. Canonical body: [`/spec:merge`](../../../plugins/spec/skills/merge/SKILL.md).

See also: [Lifecycle](../lifecycle.md) — merged state and archiving · [Directory layout](../directory-layout.md) — archive paths.

## /spec:drop

Discard a slice without merging specs into the baseline. The alternative to [/spec:merge](#specmerge). Canonical body: [`/spec:drop`](../../../plugins/spec/skills/drop/SKILL.md).

See also: [Lifecycle](../lifecycle.md) — the dropped state.

## How skills delegate

Each skill is an ultrathin invoke-and-relay wrapper over one guest-routed `specify` verb. See [AGENTS.md § Skill / CLI responsibility split](../../../AGENTS.md) for the contract and each phase's [`SKILL.md`](../../../plugins/spec/README.md) for the authoritative steps.

## See also

- [Change skills](../change-skills/index.md) — plan, execute, finalize
- [Lifecycle](../lifecycle.md) — slice and per-entry state machines
- [Quick reference card](../quick-reference.md) — all skills at a glance
