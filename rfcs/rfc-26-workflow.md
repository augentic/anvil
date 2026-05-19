# RFC-26: Workflow Collapse

> Status: Draft - Depends: [RFC-25](rfc-25-adapters.md). Supersedes: [RFC-23 (archived)](archive/rfc-23-change-lifecycle.md).

## Abstract

Collapse the `/change:*` and `/spec:*` skill families into a single `/spec:*` operator surface. The default rhythm becomes `/spec:plan → /spec:execute → /spec:finalize`, with `/spec:refine`, `/spec:build`, and `/spec:merge` as first-class step-through breakouts. Two structural review gates — Gate 1 between planning and execution, Gate 2 between synthesis and build — are CLI-stamped lifecycle states (`reviewed`, `defined_provisional`) observable through `specify plan status`, rather than skill exits or `--review-only` flags. `change.md` and `plan.yaml` survive at every slice count, including N=1; the trivial single-slice path runs through the same workflow as a degenerate case.

The CLI substrate barely moves: this is almost entirely a skill / brief redesign on top of [RFC-25](rfc-25-adapters.md)'s symmetric source-adapter contract. **There is no backward compatibility** — `/change:*` skills, `specify change *` verbs, and `/spec:define` all retire in lockstep at the 3.0 hard cut.

## Motivation

The `/change:*` and `/spec:*` skill split is a workflow seam, not an adapter seam. RFC-25 already removed the adapter seam - every input is a `source` and every output is a `target` through one resolver. The remaining seam exists only because today `enumerate` (plan time) lives in `/change:draft` and `extract` (slice time) lives in `/spec:define`, and because `plan.yaml` is the only place where a slice can be named, scoped, source-bound, and project-routed before authoring begins.

Three concrete problems follow.

**The two-namespace surface forces operators to learn a layer split that has no on-disk reflection.** Today's vocabulary distinguishes "Layer 2" (umbrella, `/change:*`) from "Layer 1" (per-slice, `/spec:*`). Both layers write to the same `.specify/` tree; both call the same CLI substrate; the only thing the layer split tracks is "is the operator drafting a change or working on a slice". After RFC-25 makes enumerate and extract symmetric, that distinction is purely vocabulary.

**Trivial single-slice work bypasses the planning vocabulary entirely.** Operators routinely call `/spec:define` directly with no `plan.yaml`. Enumeration never runs; the slice is intent-only. That is a third path on top of the layer split: `/change:draft + /change:execute + /spec:define` for multi-slice work, `/spec:define` standalone for single-slice work. Each path has its own ergonomic edges and its own failure modes. RFC-25 already tightened the inline-intent path so that `intent.enumerate` always runs at the adapter level; this RFC lifts that uniformity into the operator surface so the workflow is uniform too.

**The operator-review pause is implicit.** RFC-23's "explicit human seam" between `/change:draft` and `/change:execute` is enforced by skill exit, not by lifecycle state. `specify plan status` does not show whether the operator has reviewed the plan; CI cannot key off it; an automated driver reading the plan cannot tell whether it is safe to execute.

The collapse fixes all three by promoting the planning pipeline onto the `/spec:*` surface, keeping `/spec:refine` as a distinct step-through verb that consumes one reviewed plan entry, and CLI-stamping the operator-review pause as a structural gate.

## Design

### Principles

1. **Collapse the operator vocabulary, not the planning contract.** Source `enumerate` and source `extract` stay separate; `plan.yaml` stays single-writer; the operator-review pause stays a structural seam. What changes is that the operator types `/spec:` for everything.
2. **On-disk state is the resume mechanism.** `/spec:execute` carries no in-memory state across invocations. Re-running it re-reads `plan.yaml.lifecycle` and slice `.metadata.yaml` and dispatches to the next phase. There is no `--continue` flag, no session token, no in-flight handoff.
3. **Gates are CLI-stamped lifecycle states, not flags.** Crossing Gate 1 means running `specify plan transition <change> reviewed`; reaching Gate 2 means a slice transitioned to `defined_provisional`. Neither uses a `--review-only` mode flag. Gate 2 triggers are normative in [RFC-25 §Synthesis ↔ Gate 2 policy](rfc-25-adapters.md#synthesis--gate-2-policy).
4. **Single writer.** The CLI remains the only writer of `plan.yaml`, `.metadata.yaml`, archive paths, and lifecycle transitions. Phase skills (`/spec:plan`, `/spec:execute`, `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:finalize`) drive the agent-side work; deterministic transitions go through the CLI.
5. **Always plan, always enumerate.** Every change runs `enumerate` and produces a `plan.yaml`. N=1 is degenerate, not absent. There is no shortcut path that skips the loop verb.
6. **Breakouts are first-class.** `/spec:refine`, `/spec:build`, and `/spec:merge` are documented step-through verbs the operator reaches for when `/spec:execute` parks on a stop, or when they want to inspect a slice mid-flight. They are not "manual mode" or legacy. The same skill body is invoked from `/spec:execute`'s loop and from a direct operator call.
7. **Project routing is uniform across breakouts and the loop.** In a hub, every breakout verb performs the same project-routing fan-out as `/spec:execute`: it resolves the active slice's `project:` field via `registry.yaml`, ensures the slot is materialised, acquires the plan lock at the hub root, then `chdir`s into `.specify/workspace/<project>/`. The operator runs breakouts from the hub root, exactly where they ran `/spec:execute`.

### Vocabulary

| Term | Role | Meaning |
|---|---|---|
| **change** | noun | On-disk umbrella: `change.md`, `plan.yaml`, `archive/<change>/`. Not a slash command. |
| **plan** | verb / noun | `/spec:plan`, `specify plan *`, Gate 1, `specify plan status`. "I'm planning a change" = running `/spec:plan <scope>`. |
| **slice** | noun | One define → build → merge unit under a change. |
| **refine** | verb | Per-slice extract + synthesise (`/spec:refine`). |
| **execute** | verb | Supervised multi-slice driver (`/spec:execute`). |
| **gate** | concept | A CLI-stamped lifecycle transition the operator clears before the next stage runs. Gate 1 sits between planning and execution; Gate 2 between synthesis and build. |
| **breakout verb** | concept | `/spec:refine`, `/spec:build`, `/spec:merge` — step-through verbs run directly when `/spec:execute` parks on a stop, or for mid-flight inspection. |
| **active slice** | concept | The slice whose plan entry is currently `in-progress`, regardless of which command put it there. |
| **plan lifecycle** | enum | On `plan.yaml`: `pending → reviewed → in-progress → drained`. `/spec:plan` writes `pending`; the operator stamps `reviewed`; `/spec:execute` (or manual `specify plan next`) advances to `in-progress`; the last per-entry `done` leaves the plan `drained`. |
| **per-entry lifecycle** | enum | On each `plan.yaml` entry: `pending → in-progress → done`, or `→ blocked`. |
| **slice lifecycle** | enum | On each `.specify/slices/<name>/.metadata.yaml`: `defining → defined_provisional → defined → built → merged`. `defined_provisional` is Gate 2's parking state; promoted to `defined` by `/spec:refine --resume`. |

The slice-vs-change distinction in [`.cursor/rules/project.mdc`](../.cursor/rules/project.mdc) survives on disk; only the slash-command layer collapses to `/spec:*`.

### Operator surface

The collapsed surface is three default verbs plus three step-through verbs. The default rhythm is `/spec:plan -> (review) -> /spec:execute -> (review on each stop) -> /spec:finalize`. The step-through verbs are always available; an operator reaches for them when `/spec:execute` stops on a gate or failure, or when they want to inspect a slice before continuing.

**Default verbs (always run, in order):**

| Stage | Command | Replaces |
|---|---|---|
| **Plan** (enumerate -> propose -> validate -> Gate 1) | `/spec:plan <scope> [source <key>=<path-or-url> ...]` | `/change:draft` (and `/change:survey` and `/change:analyze`, both retired by RFC-25) |
| **Drive the plan** (per slice: extract -> synthesise -> build -> merge; stops on Gate 2, build failure, merge conflict, or plan drained) | `/spec:execute` | `/change:execute loop` |
| **Push + observe PRs + archive plan** | `/spec:finalize <name>` | `/change:finalize` |

**Step-through verbs (breakouts; operate on the active slice):**

| Stage | Command | When reached for |
|---|---|---|
| **Refine one slice** (extract -> synthesise -> Gate 2) | `/spec:refine` | When `/spec:execute` parks on Gate 2 and the operator wants to take over synthesis; or to inspect / hand-edit before building. Renames today's `/spec:define`. |
| **Build one slice** | `/spec:build` | When `/spec:execute` parks on a build failure; or to step into implementation explicitly. |
| **Merge one slice** | `/spec:merge` | Rare - usually the tail of `/spec:execute`'s per-slice loop. Useful when the operator wants to land one slice manually before resuming. |

The default rhythm is uniform at every scale: N=1 plans run through `/spec:execute` exactly the same way N=12 plans do. The step-through verbs exist so the operator can drop out, do surgery, and resume `/spec:execute` without any "continue" flag - re-entry is driven entirely by on-disk plan and slice state.

### Internal structure

**Default flow** (`/spec:plan -> /spec:execute -> /spec:finalize`):

```text
/spec:plan <scope> [source <key>=<v> ...]      ---- PLANNING ------------------------
  |-- pre-flight (project root; hub or regular; kebab-case scope)
  |-- scaffold (atomic write of change.md + plan.yaml; N=1 plans are normal)
  |-- registry validate (hub only)
  |-- [sync-workspace] (hub only - some sources are workspace-resident)
  |-- enumerate (per bound source -> merged candidate inventory in discovery.md)
  |-- propose (operator interaction: accept / edit / reject / split)
  |-- [assignment] (hub only - per-candidate --project)
  |-- plan validate
  +-- === GATE 1 === specify plan transition <scope> reviewed
        (skill exits; operator reviews change.md + plan.yaml, then
         runs /spec:execute to drive the plan - or /spec:refine to step
         into the first slice manually)

/spec:execute                                  ---- SUPERVISED LOOP (DEFAULT) ------
  |-- refuse unless plan.lifecycle == reviewed
  |-- acquire plan lock (at the hub root in hub mode)
  |-- loop:
  |     specify plan next -> slice <name> + project <project>   (entry -> in-progress)
  |     [hub only] resolve <project> via registry.yaml
  |     [hub only] sync workspace slot if missing
  |     [hub only] specify workspace prepare-branch <project> --change <scope>
  |     [hub only] chdir .specify/workspace/<project>/
  |     if slice lifecycle < defined:        invoke /spec:refine
  |       -> on defined_provisional:         -- stop -- (Gate 2 hit)
  |     if slice lifecycle < built:          invoke /spec:build
  |       -> on non-zero exit:               -- stop -- (build failure)
  |     if slice lifecycle < merged:         invoke /spec:merge
  |       -> on baseline conflict:           -- stop -- (merge conflict)
  |     [hub only] commit non-baseline residue as `specify: residue <name>`
  |     [hub only] chdir back to hub root
  |     (plan entry -> done as a side-effect of /spec:merge)
  +-- plan drained:                          -- stop -- notes "/spec:finalize ready"

/spec:finalize <scope>                         ---- DELIVERY -------------------------
  |-- refuse unless every plan entry is done
  |-- push branches:
  |     regular project -> one branch
  |     hub             -> one branch per affected workspace slot
  |-- observe PRs (poll until every PR is MERGED)
  +-- specify plan finalize -> archive change.md + plan.yaml under .specify/archive/
```

**Breakout verbs** (operate on the active slice; same skill bodies invoked by `/spec:execute`):

```text
/spec:refine                                   ---- SLICE AUTHORING -----------------
  |-- refuse unless plan.lifecycle == reviewed
  |-- active slice: already in-progress from `specify plan next`, OR operator
  |     ran `specify plan next` first (refine never writes in-progress itself)
  |-- slice create .specify/slices/<name>/    (idempotent - no-op if present)
  |-- for each bound source: source.extract -> evidence/<source-key>.yaml
  |-- synthesise per RFC-25 §Synthesis contract; specify slice validate
  |-- Gate 2 per RFC-25 §Synthesis ↔ Gate 2 policy:
  |     [conflict] or Status: conflict -> defined_provisional (always stop)
  |     Status: divergence or unknown -> defined_provisional (supervised default)
  |     clean -> slice transition defined
  |-- /spec:refine --resume: promote defined_provisional -> defined after edits
  +-- (normal path) slice transition defined when synthesis clean

/spec:build                                    ---- IMPLEMENTATION ------------------
  |-- refuse unless slice lifecycle is defined (not defined_provisional)
  |-- run tasks.md tasks in order (resume from last failed task on re-entry)
  +-- slice transition built

/spec:merge                                    ---- LANDING -------------------------
  |-- refuse unless slice lifecycle is built
  |-- fold slice deltas into baseline specs
  |-- on baseline conflict: -- stop -- (operator resolves; re-invoke)
  |-- slice transition merged; archive .specify/slices/<name>/ -> .specify/archive/
  +-- specify plan transition <name> done
```

Two responsibility rules keep the breakout / loop paths consistent:

1. **`specify plan next` is the only writer of the per-entry `in-progress` transition.** Both `/spec:execute`'s loop and an operator stepping in manually call it; `/spec:refine` never does. This lets `/spec:refine` operate uniformly on "the active slice" regardless of who selected it.
2. **`/spec:merge` is the only writer of the per-entry `done` transition.** Per-slice closure lives with the verb that produces the terminal state, not the loop driver, so a manual `/spec:merge` leaves the plan in exactly the state `/spec:execute` would have left it in - and a subsequent `/spec:execute` invocation just pulls the next entry.

### The two gates

| Gate | Position | Reviewed | Mechanism | Skip |
|---|---|---|---|---|
| **Gate 1 - plan** | After `plan validate`, before any `extract` | Slice boundaries, `sources[]` per entry, `project` assignment, descriptions | `specify plan transition <scope> reviewed` (CLI-stamped; refuses progress until set) | `--yes-plan` for trusted automation; emits loud journal entry |
| **Gate 2 - artifacts** | After synthesis per slice, before `/spec:build` | `spec.md` / `design.md` markers and `Status:` lines per [RFC-25 §Synthesis ↔ Gate 2 policy](rfc-25-adapters.md#synthesis--gate-2-policy) | Slice lifecycle `defined_provisional`; promoted to `defined` by `/spec:refine --resume` | Auto-promote to `defined` when synthesis is clean (`Status: agreed`, no `[conflict]`). `--yes-gate2` auto-promotes non-conflict stops only (divergence/unknown), never `[conflict]` |

Gate 1 is the structural successor to RFC-23's "explicit human seam" - same logical spot, now CLI-stamped and observable in `specify plan status` rather than implicit in skill exit. Gate 2 lets synthesis ask for review without reintroducing a `/change:*` family of skills.

**Stepping in without `/spec:execute`:** After Gate 1, an operator may run `specify plan transition <scope> reviewed`, then `specify plan next`, then `/spec:refine` directly — without ever invoking `/spec:execute`. `plan next` owns the `in-progress` transition; refine only consumes the active slice.

### Combined lifecycle (RFC-25 + RFC-26)

```text
PLAN (plan.yaml)          SLICE (.metadata.yaml)           STAGE
──────────────────────────────────────────────────────────────────
pending                   —                              /spec:plan
  │ (operator)            —                              Gate 1: plan transition reviewed
reviewed                  —                              /spec:execute allowed
in-progress (plan)        defining                       extract + synthesise
  │                       defined_provisional            Gate 2 (supervised stop)
  │                       defined                        /spec:refine --resume or clean synth
  │                       built                          /spec:build
  │                       merged                         /spec:merge → plan entry done
drained                   —                              /spec:finalize
```

RFC-25 Specify 2.x may still emit `awaiting-review` instead of `defined_provisional` until the 3.0 upgrade normalises enums.

### Operator modes and the `/spec:execute` stop/resume contract

The three-verb default rhythm is one of three modes the same surface supports:

| Mode | Shape | When used |
|---|---|---|
| **Default (supervised)** | `/spec:plan -> (review) -> /spec:execute -> (review on each stop) -> /spec:finalize` | Day-one operator workflow. Gates 1 and 2 are honoured; build / merge failures pause for inspection. |
| **Breakout (step-through)** | At any `/spec:execute` stop, run `/spec:refine` / `/spec:build` / `/spec:merge` directly on the active slice; then call `/spec:execute` again to resume the loop | When a slice needs human authoring beyond `[conflict]` resolution, or when the operator wants to confirm one slice lands cleanly before continuing. |
| **Full automation** | `/spec:plan --yes-plan ... && /spec:execute --yes-gate2 && /spec:finalize` | Trusted CI / cron pipelines. Gate 1 auto-clears with `--yes-plan`; Gate 2 auto-promotes only non-conflict stops with `--yes-gate2` (see RFC-25 policy table). |

All three modes share one substrate: `/spec:execute` is a state-machine driver, not a session. It carries no in-memory state across invocations.

**Stop conditions (defaults):**

| Trigger | What `/spec:execute` does | Operator next step |
|---|---|---|
| Slice transitions to `defined_provisional` (Gate 2) | Exits with hint naming the slice, `Status:` lines, and any `[conflict]` / `[divergence]` markers | Resolve artifacts; `/spec:refine --resume` promotes to `defined`; then `/spec:execute` |
| `/spec:build` returns non-zero | Exits with the failing task id and the slice's build log path | Fix and run `/spec:execute` (re-runs build from last failed task), or step in with `/spec:build` |
| `/spec:merge` reports a baseline conflict | Exits with the conflicting spec paths | Resolve, then `/spec:execute` to resume |
| `specify plan next` reports drained | Exits cleanly, prints "Plan complete. Run `/spec:finalize <name>`." | Run `/spec:finalize` |

**Resume contract:** `/spec:execute` re-reads `plan.yaml.lifecycle`, `specify plan next`, and the active slice's `.metadata.yaml` on every invocation, then dispatches to the next phase. The breakout invariant that makes this safe: **anything a step-through verb writes — artifacts, `.metadata.yaml` transitions, task progress — is observable to the next `/spec:execute` invocation.** Breakouts never hand back; `/spec:execute` just re-reads state. No mode handoff, no session token, no `--continue` flag.

**Operator dials on `/spec:execute`:**

| Flag | Purpose | Risk |
|---|---|---|
| `--one` | Process exactly one slice (refine -> build -> merge), then exit | Low - useful for "let me see the first one land before continuing" |
| `--until <slice>` | Loop up to and including `<slice>`, then exit | Low |
| `--dry-run` | Print plan walkthrough; no writes | None |
| `--yes-gate2` | Auto-promote `defined_provisional` when no `[conflict]` / `Status: conflict` (loud journal entry); never resolves conflicts | High - trusted automation only |
| `--continue-on-build-fail` | Mark failed slice `blocked`, continue with `specify plan next` | High - batch processing of independent slices only |

Defaults give the operator the supervised loop: fail-fast, no surprises. Flags exist for the long tail.

### Trivial single-slice path

`/spec:plan` always runs `enumerate` and always writes a `plan.yaml`, even at N=1. For greenfield work with no legacy repo, `intent` is bound implicitly, `intent.enumerate` emits one candidate from the operator's brief, propose auto-accepts (still calling `specify plan add`, so `plan.yaml` exists with one `pending` entry), and Gate 1 shows a one-line `Y` / `edit` / `n` prompt. Enumeration is **never skipped** — for trivial work it is **degenerate**, not absent — so there is no special-case code path and the operator's mental model is uniform at every scale.

**Headless trivial path:** `specify plan create <scope>` + `specify plan add` + `specify plan transition reviewed` + `/spec:execute` for CI that cannot run interactive propose.

### `plan.yaml` and the implicit umbrella

`plan.yaml` survives at every slice count. Three reasons:

1. **Single writer.** The single-writer invariant on `plan.yaml` ([`plugins/change/references/plan-single-writer.md`](../plugins/change/references/plan-single-writer.md)) is one of the framework's load-bearing simplicity wins. The collapse keeps it intact.
2. **Auditability.** Every change leaves a `change.md` + `plan.yaml` pair in `archive/`. Skipping it for N=1 means two archive shapes.
3. **Future-proofing.** A N=1 case in this turn may become N=3 next turn after the operator edits at Gate 1.

A one-slice plan is simply small:

```yaml
version: 1
name: add-search-filter
sources:
  intent:
    adapter: intent
    value: "Add a search filter to the user list."
slices:
  - name: add-search-filter
    target: omnia
    sources: [intent]
    candidate: add-search-filter
    status: pending
```

The CLI surface `specify plan {add,amend,transition,next,validate,...}` is unchanged.

### `change.md` when the umbrella is implicit

Today `change.md` is the operator-facing brief (intent, motivation, links) and `plan.yaml` is the machine-readable slate. The collapse does not change that split; it changes only who scaffolds the pair.

- The "change name" is whatever the operator passes to `/spec:plan`. There is no separate `/change:draft <name>` step.
- `change.md` is auto-scaffolded from the operator's brief at scaffold time. If the operator passed a description, that description seeds `change.md`. The operator may edit it at Gate 1 (it appears alongside `plan.yaml`).
- For N=1, `change.md` is essentially the operator's one-liner. That is fine - the file's value is in the audit trail, not its length.

`change.md` is **not** renamed to `brief.md` or moved under `.specify/plans/<id>/`. The cost of churning every reference and downstream consumer outweighs the vocabulary cleanup. The operator's mental model is "I'm refining slice X within change Y"; the disk layout retains the explicit umbrella so `specify plan lock`, `specify plan status`, and CI keyed off these files keep working.

### Single-repo vs multi-repo

`project.yaml: hub:` is the only context discriminator the collapsed workflow needs:

| `hub:` | Behaviour in `/spec:plan` |
|---|---|
| `false` (regular project) | Single project root; `planSlice.project` omitted or defaulted; `sync-workspace` and `assignment` substeps skipped. |
| `true` (registry-only platform hub) | Reads `registry.yaml`; runs `sync-workspace` before enumerate (some sources are workspace-resident); `propose` asks per-candidate `--project` assignment. |

`specify workspace sync` stays as today; it is called from `/spec:plan`'s enumerate substage for hubs, before `source.enumerate` runs.

**Standalone vs hub-driven for the same project.** A registered project can also be driven standalone by running the default workflow from the project root - the hub is one way to drive a project, not the only way. Each project repo's `.specify/` is independent: the hub *references* it via `registry.yaml`, it does not own it. The cost of supporting both modes is that the same project can carry an active hub-driven plan and an active standalone plan simultaneously; today's `specify plan lock` covers the per-root case but does not coordinate across roots. Operators who drive a project both ways are responsible for not racing themselves; the framework will surface drift through `specify workspace sync`'s `stale-workspace-clone` warning on the next hub-side sync.

### Where pipeline stages live after the collapse

| Stage | Specify 2.x (RFC-25) | Specify 3.x (RFC-26) |
|---|---|---|
| `source.enumerate` | `/change:draft` | `/spec:plan` enumerate substage |
| `source.extract` | `/spec:define` | `/spec:refine` extract substage (after Gate 1) |
| Core synthesis (`proposal`, `specs`, `design`, `tasks`) | `/spec:define` via `specify slice synthesize pipeline` | `/spec:refine` synthesise substage |
| `target.shape` | Loaded during core synthesis | Same |
| `target.build` | `/spec:build` via `specify target pipeline build` | `/spec:build` (unchanged) |
| `target.merge` | `/spec:merge` via `specify target pipeline merge` | `/spec:merge` (unchanged) |

Target adapters no longer own define-phase artifact briefs ([RFC-25](rfc-25-adapters.md)). The collapse is on the operator surface and planning seam; build/merge pipelines are unchanged.

### Hub routing and plan lock

In hub mode, breakout verbs and `/spec:execute` share one routing contract (the same fan-out shown inside the `/spec:execute` loop in §Internal structure): acquire the plan lock at the hub root, resolve the active slice's project, sync the slot if missing, `chdir` into `.specify/workspace/<project>/`, run phase work, then return.

| Path | Location |
| ---- | -------- |
| `change.md`, `plan.yaml`, `discovery.md` | Hub root |
| `slices/<name>/`, `evidence/`, `journal.yaml` | Workspace slot `.specify/` |
| Plan lock | Hub root only |
| `specify slice validate` | Run against slot CWD after chdir |

Acceptance scenarios #11–#12 exercise this contract; operators never manually `chdir` into slots for breakouts.

### CLI surface

| Verb (RFC-25) | RFC-26 |
|---|---|
| `specify change draft <name>` | **Renamed** `specify plan create <name>` (low-level primitive; `/spec:plan` skill calls it) |
| `specify change show <name>` | **Renamed** `specify plan show <name>` |
| `specify change finalize <name>` | **Renamed** `specify plan finalize <name>` |
| `specify plan {add,amend,transition,next,validate,lock,doctor,archive,...}` | **Unchanged** - the substrate |
| `specify workspace {sync,prepare-branch,push}` | **Unchanged** - context + delivery verbs |
| `specify slice {create,transition,touched-specs,validate,merge,overlap,journal,...}` | **Unchanged** |
| `specify slice synthesize pipeline` | **Unchanged** (RFC-25) — `/spec:refine` calls it for synthesis substep order |
| `specify target pipeline {build,merge}` | **Unchanged** (RFC-25 split) — `define` phase retired |
| `specify source {resolve,list,validate}` / `specify target {resolve,list,validate}` | **Unchanged** (RFC-25); `/spec:plan` and `/spec:refine` dispatch through them |

New plan-lifecycle transition keyword:

| Verb | Purpose |
|---|---|
| `specify plan transition <change> reviewed` | Stamp Gate 1 cleared; `/spec:execute` refuses to run until set |

The headline: **the CLI substrate barely moves.** The collapse is almost entirely a skill / brief redesign. That is a strong signal it is feasible; RFC-25's single-writer discipline already factored out the deterministic primitives.

`specify change *` verbs do not ship as deprecation aliases - RFC-26 is a hard cut at 3.0, consistent with RFC-25's no-backcompat stance.

### Skill / SKILL.md changes

Skill bodies follow §Operator surface and §Internal structure; this table is the mechanical move-and-rename list.

| File | Action |
|---|---|
| `plugins/spec/skills/plan/SKILL.md` | **New.** Absorbs enumerate, propose, assignment, plan validate, Gate 1; lifts brief topology from `plugins/change/skills/draft/`. |
| `plugins/spec/skills/execute/SKILL.md` | **New.** Default driver renamed from `/change:execute loop`; stop-condition surface per §Operator modes. |
| `plugins/spec/skills/refine/SKILL.md` | **Renamed from `define/`**, rewritten as a step-through breakout sharing one skill body with `/spec:execute`'s loop. Driver-supplied source arguments retired in favour of plan-resolved bindings. |
| `plugins/spec/skills/build/SKILL.md` | **Step-through breakout.** Refuses unless slice is `defined` (not `defined_provisional`). |
| `plugins/spec/skills/merge/SKILL.md` | **Step-through breakout.** Notes `/spec:finalize` ready when last entry reaches terminal. |
| `plugins/spec/skills/finalize/SKILL.md` | **New** — renamed `/change:finalize`, no behaviour change. |
| `plugins/spec/skills/init/SKILL.md` | Tiny edit: AGENTS.md scaffolding mentions `/spec:plan` instead of `/change:draft`. |
| `plugins/spec/skills/drop/SKILL.md` | Unchanged. |
| `plugins/spec/skills/define/SKILL.md` | **Retired** (renamed to `refine/`). |
| `plugins/change/skills/{draft,execute,finalize}/SKILL.md` | **Retired.** `draft` references move to `plugins/spec/skills/plan/references/`; `execute` per-slice algorithm survives under `plugins/spec/skills/execute/references/`. |
| `plugins/change/skills/{analyze,survey}/SKILL.md`, `plugins/spec/skills/extract/SKILL.md` | Already retired by RFC-25. |

Net change: `plugins/change/` empties entirely and is removed.

### `.specify/` directory layout

Almost unchanged. `slices/<name>/` is untouched; the only additions come from RFC-25 (`evidence/<source-key>.yaml`). Layout differs between regular projects and hubs: in a hub, `change.md` / `plan.yaml` / `discovery.md` stay at the hub root while slice artifacts live one level deeper inside each workspace slot.

**Regular project** (`project.yaml.hub: false`):

```text
.specify/
|-- project.yaml
|-- change.md                 # always written, even N=1
|-- plan.yaml                 # always written, even N=1
|-- discovery.md              # transient, during /spec:plan enumerate
|-- slices/<name>/
|   |-- proposal.md
|   |-- spec.md
|   |-- design.md
|   |-- tasks.md
|   |-- evidence/<source-key>.yaml   # RFC-25
|   |-- .metadata.yaml
|   +-- journal.yaml
+-- archive/
    +-- <change>/<slice>/
```

**Platform hub** (`project.yaml.hub: true`):

```text
.specify/                                       # at hub root
|-- project.yaml                                # hub: true
|-- registry.yaml
|-- workspace.md
|-- change.md                                   # at hub root
|-- plan.yaml                                   # at hub root
|-- discovery.md                                # at hub root, during /spec:plan
|-- workspace/
|   +-- <project>/                              # one slot per registered project
|       +-- .specify/
|           |-- project.yaml                    # project's own (hub: false)
|           |-- slices/<name>/                  # slice artifacts live here
|           |   |-- proposal.md
|           |   |-- spec.md
|           |   |-- design.md
|           |   |-- tasks.md
|           |   |-- evidence/<source-key>.yaml
|           |   |-- .metadata.yaml
|           |   +-- journal.yaml
|           +-- archive/
|               +-- <change>/<slice>/
+-- archive/
    +-- <change>/                               # hub-root archive holds plan/change only
        |-- change.md
        +-- plan.yaml
```

The hub's own `slices/` directory is unused (the hub never authors slices directly). The fan-out is "one plan at the hub, N slice trees across N workspace slots", which is what makes `/spec:execute`'s per-slice `chdir` and the breakout verbs' project routing load-bearing.

## Workflow changes

The collapse maps a four-verb 2.x flow onto a three-verb 3.x flow:

| Stage | RFC-25 (Specify 2.x) | RFC-26 (Specify 3.x) |
| ----- | -------------------- | -------------------- |
| Planning | `/change:draft` runs `enumerate` and propose | `/spec:plan` runs the same, then parks at Gate 1 (`plan.lifecycle == reviewed`) |
| Per-slice drive | `/change:execute loop` calls `/spec:define`, `/spec:build`, `/spec:merge` per slice | `/spec:execute` calls `/spec:refine`, `/spec:build`, `/spec:merge` per slice; stops on Gate 2, build failure, or merge conflict |
| Delivery | `/change:finalize` pushes, observes, archives | `/spec:finalize` — same behaviour, renamed |

The named verbs `/change:draft`, `/change:execute`, `/change:finalize`, and `/spec:define` are removed. Their behaviour lives inside `plugins/spec/skills/{plan,execute,finalize,refine}/`, invoked uniformly through the workflow. The looping driver remains non-negotiable for the same reason as before — an operator with a 12-slice change should not type 36 verb invocations — and is now uniform at N=1 too, so the mental model holds at every scale.

## Implementation Plan

Strictly incremental on top of RFC-25. Land RFC-25 acceptance fixtures (especially synthesis, provenance, and conflict/divergence) **before** step 4 below.

1. **Land RFC-25.** The collapse depends on symmetric `enumerate` / `extract`, core synthesis, and [RFC-25 §Synthesis ↔ Gate 2 policy](rfc-25-adapters.md#synthesis--gate-2-policy).
2. **Promote the review seam inside `/change:draft`** as a no-behaviour-change refactor. Add `reviewed` to plan lifecycle; `specify plan transition <change> reviewed` stamps Gate 1.
3. **Parallel plugin channel for 2.x early adopters (not an in-repo feature flag).** Ship `/spec:plan` and `/spec:refine` skills on a `specify-3.0-preview` marketplace tag while 2.x defaults keep `/change:*`. Operators opt in by plugin pin; 3.0 hard cut removes `/change:*` entirely — consistent with RFC-25's no-graceful-degradation stance.
4. **Rename `/change:execute loop` -> `/spec:execute` and add the §Operator modes stop/resume contract.** The loop algorithm is unchanged; what is new is that the skill stops on Gate 2, build failure, and merge conflict with operator-facing hints, and resumes by re-reading on-disk state on the next invocation. `/change:finalize` becomes `/spec:finalize` (no behaviour change). **This is the load-bearing step** - the collapsed default workflow becomes `/spec:plan -> /spec:execute -> /spec:finalize` only once this step lands.
5. **Make `/spec:plan`, `/spec:execute`, `/spec:finalize` the documented default workflow** and `/spec:refine`, `/spec:build`, `/spec:merge` the documented step-through breakouts. Rewrite `AGENTS.md`, `.cursor/rules/project.mdc`, the README, the marketplace manifest, and the tutorial walkthrough. `/change:*` and `/spec:define` move to a "removed" section.
6. **Delete `/change:draft`, `/change:execute`, `/change:finalize`, and `/spec:define`.** The `plugins/change/` directory is removed. `plugins/spec/skills/define/` is renamed to `plugins/spec/skills/refine/`.

### Acceptance scenarios

Run these against the collapsed skills before step 6. Each is an honest stress test of where the collapse can fail.

| # | Scenario | What it stress-tests |
|---|---|---|
| 1 | **Pure intent, one slice.** Operator runs `/spec:plan fix-typo "fix typo in user.rs"`. | Degenerate `intent.enumerate`; Gate 1 ergonomics on trivial work; `change.md` + `plan.yaml` justifiability at N=1. |
| 2 | **Documentation, one slice.** Operator binds a single docs path. | `documentation.enumerate` correctness at the new entry point. |
| 3 | **Documentation, multi-slice.** Operator binds docs that map to N candidates. | Propose/edit/reject loop; Gate 1 amendment flow. |
| 4 | **Legacy-code, multi-slice.** Operator binds a legacy repo. | `legacy-code-*.enumerate`; survey/repair loop under the new skill; under-slicing failure mode. |
| 5 | **Multi-source conflict and divergence.** Two sources disagree about a candidate. | RFC-25 `[conflict]` / `[divergence]` policy; Gate 2; `defined_provisional` -> `defined` promotion. |
| 6 | **Multi-repo assignment from a hub.** Operator runs `/spec:plan` in a hub. | `hub:` discriminator; per-candidate `--project` at propose; workspace sync timing. |
| 7 | **Operator amends one-slice plan into two slices at Gate 1.** | Plan amendment via `specify plan amend`; re-entry to Gate 1 after amend. |
| 8 | **Step-through breakout mid-execute.** Operator starts `/spec:execute`; on the second slice they cancel, run `/spec:build` directly to investigate, then re-invoke `/spec:execute`. | Stop/resume contract; that step-through verbs leave on-disk state consistent for `/spec:execute` to resume without flags. |
| 9 | **`/spec:execute` parks on Gate 2, operator resolves and resumes.** A multi-source slice synthesises with `[conflict]` markers; operator edits `spec.md`, runs `/spec:refine --resume`, then `/spec:execute`. | Gate 2 stop hint; provisional -> defined promotion; re-entry into the loop at the correct phase. |
| 10 | **`/spec:execute` parks on a build failure, operator fixes, resumes.** Slice's `cargo test` fails; operator patches the crate; runs `/spec:execute`. | Build-failure stop hint; build resumes from the failed task; loop continues to merge. |
| 11 | **Hub `/spec:execute` across two projects.** Plan with slices targeting `project-a` and `project-b`; operator runs `/spec:execute` from the hub root. | Per-slice project routing; slot materialisation; `prepare-branch`; `chdir` + residue commit; plan-lock semantics at the hub root while phase work runs in slots. |
| 12 | **Hub breakout after Gate 2 in a slot.** `/spec:execute` parks on `auth-rotate` (in `project-a`); operator stays at hub root and runs `/spec:refine --resume`. | Project-routing rule for breakout verbs; active-slice resolution across the hub/slot boundary; correct chdir without operator intervention. |

If any of #1-4 fail the ergonomics test (operator confusion, lost time, surprised state), revisit the trivial-path optimisation in §Trivial single-slice path before pushing through step 6.

## Migration

Ships as Specify 3.0 with **no backward compatibility**, following [RFC-25 §Migration](rfc-25-adapters.md#migration)'s rationale: half-renames produce a confusing transitional vocabulary that costs more than a clean cut. Operators upgrading from 1.x go through both RFCs in sequence — or, in practice, install 3.0 directly once both have landed.

For operators upgrading from 2.x, `specify upgrade` is a one-shot migration that:

- Renames `plugins/change/skills/{draft,execute,finalize}/` → `plugins/spec/skills/{plan,execute,finalize}/`, and `plugins/spec/skills/define/` → `plugins/spec/skills/refine/`.
- Renames `specify change {draft,show,finalize}` → `specify plan {create,show,finalize}`.
- Adds `reviewed` to the plan-lifecycle enum and `defined_provisional` to the slice-lifecycle enum (retires `awaiting-review`).
- Re-fetches operator briefs from upgraded plugin repositories and removes the `plugins/change/` entry from `.cursor-plugin/` marketplace manifests.
- May be combined with the RFC-25 upgrade path when jumping 1.x → 3.0 in one step.

**Skill authors:** `/change:*` references in plugin manifests retire; calls to `/spec:define` become `/spec:refine`; supervised-loop callers use `/spec:execute` and let stop conditions surface to the operator rather than retrying internally.

**Automation consumers:** the rename surface is the workspace and plan namespaces. Plan envelopes gain `lifecycle: reviewed`; slice envelopes gain `lifecycle: defined_provisional` (wire alias `defined:provisional` permitted). CI treats Gate 1 as `plan.lifecycle == reviewed` and Gate 2 as a failed-gate signal on `defined_provisional` unless promotion to `defined` is recorded in `journal.yaml`.

## Alternatives Considered

**Fold RFC-26 into RFC-25 as one big-bang 2.0.** Rejected. The two RFCs are independent — RFC-25 changes the adapter axis, RFC-26 changes the operator surface. Bundling them forces RFC-25 to wait for the full workflow-collapse acceptance surface before it can ship. Sequencing reduces blast radius.

**Overload a phase verb with the loop (`/spec:refine --loop`, auto-loop inside `/spec:refine`).** Rejected. The framework keeps loop drivers and per-slice phases distinct; `/spec:execute` is the supervised loop. An operator should know which verb is which.

**Hide `/spec:execute` and chain `/spec:refine → /spec:build → /spec:merge` as the default.** Rejected. The operator then has to know when to stop typing — N=1 chains differ from N=12 chains — and the loop driver disappears from the documented surface even though it still exists in the agent's head.

**Collapse `/spec:build` or `/spec:merge` into `/spec:refine`.** Rejected. Build and merge have their own halt conditions (tests fail, baseline drift, PR review) unrelated to synthesis. `/spec:refine` ends at synthesis (or Gate 2).

**`/spec:plan` as a shell wrapper around `/change:draft + /change:execute`.** Rejected. Two skills behind one command is worse than the status quo; the collapse is a real refactor of the brief pipeline, not a thin shim.

**Ship `/change:execute` and `/change:finalize` as deprecation aliases for one release.** Rejected. RFC-25's no-graceful-degradation stance applies uniformly; deprecation aliases under a 3.0 hard cut undermine the rename's clarity.

**Keep `/spec:define` as the slice authoring verb.** Rejected. After the collapse, "defining" is what `/spec:plan` does (proposing slices); per-slice synthesis is downstream of that — refining a slice the operator already named at planning time. `/spec:refine` reads honestly; `/spec:define` reads as a redundant second naming step.

See also §Non-Goals — the items below are not just rejected once, they are *out of scope* for this RFC and future ones.

## Non-Goals

- `/spec:execute` session tokens or a `--continue` flag. On-disk state (`plan.yaml.lifecycle`, slice `.metadata.yaml`) is the only resume mechanism; re-running with no flags is the contract.
- Folding `/spec:finalize` into `/spec:merge`. They stay distinct to avoid the merge-overload failure mode RFC-23 removed.
- Deleting or renaming `change.md` / `plan.yaml`. The single-writer invariant on these files is load-bearing; the collapse keeps both intact.
- Auto-resolution of Gate 2 `[conflict]` markers. The operator decides; `--yes-gate2` only auto-promotes non-conflict stops per [RFC-25 §Synthesis ↔ Gate 2 policy](rfc-25-adapters.md#synthesis--gate-2-policy).
- Cross-root coordination of simultaneous standalone-and-hub plans. Operators are responsible for not racing themselves; the framework surfaces drift via `specify workspace sync`'s `stale-workspace-clone` warning.
- A "manual mode" where `/spec:execute` does not exist. The supervised loop is the documented default at every slice count, including N=1.
- Backward compatibility with 2.x `/change:*` skills, `specify change *` verbs, or `/spec:define` invocations. All retire at the 3.0 hard cut.

## Open Questions

1. **Gate 1 default - blocking or auto-continue?** **Resolved:** blocking by default; `--yes-plan` for automation (§The two gates).
2. **Slice directory creation timing.** Create `.specify/slices/<name>/` at `plan add` (Gate 1 sees real dirs) or only when `/spec:refine` starts extract (Gate 1 is plan-pure)? Current preference: at extract, to keep Gate 1 plan-pure.
3. **Finalize / merge separation.** Always separate, or fold `/spec:finalize` into `/spec:merge` when the merged slice is the last in the plan? Current preference: always separate (see §Alternatives Considered).
4. **Operator vocabulary for "change".** **Resolved in §Vocabulary** — `change` on disk, `plan` / `refine` / `execute` as verbs.

5. **Lifecycle enum wire format.** Current preference: snake_case in `.metadata.yaml` and JSON (`defined_provisional`); human docs may show `defined:provisional` as an alias only.

## Observability ([RFC-19](rfc-19-observability.md))

Emit journal events (implementation alongside RFC-25 step 14 where possible, complete by 3.0):

| Event | When |
| ----- | ---- |
| `plan.transition.reviewed` | Gate 1 cleared |
| `slice.transition.defined_provisional` | Gate 2 stop |
| `slice.transition.defined` | Gate 2 cleared or clean synthesis |
| `slice.synthesis.conflict` | `[conflict]` markers written |

Enables CI and hosted runners to observe gates without parsing skill exit codes.

## References

- [RFC-19: Observability](rfc-19-observability.md) — journal events for gates and synthesis outcomes (§Observability).
- [RFC-25: Directional Adapters](rfc-25-adapters.md) — adapter-axis prerequisite; §Synthesis contract, §Synthesis ↔ Gate 2 policy, and pipeline split.
- [RFC-23: Change Lifecycle (archived)](archive/rfc-23-change-lifecycle.md) - the three-skill model RFC-26 supersedes. Gate 1 is the structural successor to RFC-23's "explicit human seam"; the seam itself survives, the verb names do not.
- [RFC-22: Migration Ledger and Slice Mapping](rfc-22-ledger.md) - unaffected by the collapse; the per-change ledger continues to live alongside `change.md` and `plan.yaml`.
- [RFC-24: Omnia Plan Composition](rfc-24-omnia.md) - unaffected; per-slice composition lives on `planSlice` regardless of which verb wrote it.
- [`plugins/spec/skills/init/SKILL.md`](../plugins/spec/skills/init/SKILL.md) - `hub:` discriminator, the only context primitive the collapse needs.
- [`plugins/change/references/plan-single-writer.md`](../plugins/change/references/plan-single-writer.md) - the single-writer invariant preserved by the collapse. Note: this reference will move when `plugins/change/` is removed by step 6 of §Implementation Plan; the invariant survives, the path does not.
- [`AGENTS.md`](../AGENTS.md) §Plan-driven loop - vocabulary this RFC substantially rewrites.
- [`.cursor/rules/project.mdc`](../.cursor/rules/project.mdc) §Vocabulary - slice vs change distinction the collapse blurs at the verb level but preserves on disk.
