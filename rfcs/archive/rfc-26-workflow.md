# RFC-26: Workflow Collapse (archived — folded into RFC-25)

> Status: Archived — never shipped as a standalone RFC. The workflow-collapse design described below was folded into [RFC-25: Directional Adapters and Workflow Collapse](../rfc-25-workflow.md) at draft stage; that RFC is the live source of truth for everything here. This document is retained for the design history only — every cross-reference into it should be repointed at the relevant §section of RFC-25.
>
> Original status line: Draft — Depends: [RFC-25](../rfc-25-workflow.md). Supersedes: [RFC-23 (archived)](rfc-23-change-lifecycle.md).

## Abstract

Collapse the `/change:*` and `/spec:*` skill families into a single `/spec:*` operator surface. The default rhythm becomes `/spec:plan → /spec:execute → /spec:finalize`, with `/spec:refine`, `/spec:build`, and `/spec:merge` as first-class step-through breakouts. One structural review gate — Gate 1, between planning and execution — is a CLI-stamped lifecycle state (`reviewed`) observable on `plan.yaml`, rather than a skill exit or a `--review-only` flag. `change.md` and `plan.yaml` survive at every slice count, including N=1; the trivial single-slice path runs through the same workflow as a degenerate case.

v1 ships the supervised default loop only — no automation flags. Synthesis review is operator-driven via inline `[conflict]` / `[divergence]` / `[unknown]` tags in `spec.md` — no second parking gate in v1; see §Non-Goals. The CLI substrate barely moves: this is almost entirely a skill / brief redesign on top of [RFC-25](../rfc-25-workflow.md)'s source-adapter contract. **There is no backward compatibility** — `/change:*` skills, `specify change *` verbs, and `/spec:define` all retire in lockstep at the 3.0 hard cut.

## Motivation

The `/change:*` and `/spec:*` skill split is a workflow seam, not an adapter seam. RFC-25 unified inputs as `source` adapters and outputs as `target` adapters; the remaining split exists only because `enumerate` lives in `/change:draft` and `extract` in `/spec:define`, and because `plan.yaml` is where slices are named, source-bound, and project-routed before authoring.

Three problems follow: (1) operators learn a two-namespace surface ("Layer 2" `/change:*` vs "Layer 1" `/spec:*`) with no on-disk reflection; (2) trivial single-slice work bypasses planning via orphan `/spec:define`, creating a third path with its own failure modes; (3) the operator-review pause between draft and execute is enforced by skill exit, not observable lifecycle state — CI and automation cannot key off it.

The collapse promotes planning onto `/spec:*`, keeps `/spec:refine` as the per-slice authoring breakout, and CLI-stamps the review pause as Gate 1 (`plan.lifecycle == reviewed`).

## Design

### Principles

1. **Collapse the operator vocabulary, not the planning contract.** Source `enumerate` and source `extract` stay separate; `plan.yaml` stays single-writer; the operator-review pause stays a structural seam. What changes is that the operator types `/spec:` for everything.
2. **On-disk state is the resume mechanism.** `/spec:execute` carries no in-memory state across invocations. Re-running it re-reads `plan.yaml.lifecycle` and slice `.metadata.yaml` and dispatches to the next phase. There is no `--continue` flag, no session token, no in-flight handoff.
3. **The plan gate is a CLI-stamped lifecycle state, not a flag.** Crossing Gate 1 means running `specify plan transition <change> reviewed`; `/spec:execute` refuses to run until set. v1 ships exactly one structural gate; review of synthesis output is operator-driven through inline `[conflict]` / `[divergence]` / `[unknown]` tags in `spec.md` rather than a second parking state or build precondition. See §The plan gate.
4. **Single writer.** The CLI remains the only writer of `plan.yaml`, `.metadata.yaml`, archive paths, and lifecycle transitions. Phase skills (`/spec:plan`, `/spec:execute`, `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:finalize`) drive the agent-side work; deterministic transitions go through the CLI.
5. **Always plan, always enumerate.** Every change runs `enumerate` and produces a `plan.yaml`. N=1 is degenerate, not absent. There is no shortcut path that skips the loop verb.
6. **Breakouts are first-class.** `/spec:refine`, `/spec:build`, and `/spec:merge` are documented step-through verbs the operator reaches for when `/spec:execute` parks on a stop, or when they want to inspect a slice mid-flight. They are not "manual mode" or legacy. The same skill body is invoked from `/spec:execute`'s loop and from a direct operator call.
7. **Project routing is uniform across breakouts and the loop.** In a hub, every breakout verb performs the same project-routing fan-out as `/spec:execute`: it resolves the active slice's `project:` field via `registry.yaml`, ensures the slot is materialised, acquires the plan lock at the hub root, then `chdir`s into `.specify/workspace/<project>/`. The operator runs breakouts from the hub root, exactly where they ran `/spec:execute`.
8. **Supervised by default; no automation flags in v1.** `/spec:execute` ships without `--yes-plan`, `--yes-gate2`, `--one`, `--until`, `--dry-run`, or `--continue-on-build-fail`. The default supervised loop is the contract; flags reappear when a real automation consumer (CI, hosted runner) asks for them.

### Vocabulary

| Term | Role | Meaning |
|---|---|---|
| **change** | noun | On-disk umbrella: `change.md`, `plan.yaml`, `archive/<change>/`. Not a slash command. |
| **plan** | verb / noun | `/spec:plan`, `specify plan *`, Gate 1. "I'm planning a change" = running `/spec:plan <scope>`. |
| **slice** | noun | One define → build → merge unit under a change. |
| **refine** | verb | Per-slice extract + synthesise (`/spec:refine`). |
| **execute** | verb | Supervised multi-slice driver (`/spec:execute`). |
| **gate** | concept | A CLI-stamped lifecycle transition the operator clears before the next stage runs. v1 ships one gate (Gate 1) between planning and execution. |
| **breakout verb** | concept | `/spec:refine`, `/spec:build`, `/spec:merge` — step-through verbs run directly when `/spec:execute` parks on a stop, or for mid-flight inspection. |
| **active slice** | concept | The slice whose plan entry is currently `in-progress`, regardless of which command put it there. |
| **plan lifecycle** | enum | On `plan.yaml`: `pending → reviewed → in-progress → drained`. `/spec:plan` writes `pending`; the operator stamps `reviewed`; `/spec:execute` (or manual `specify plan next`) advances to `in-progress`; the last per-entry `done` leaves the plan `drained`. |
| **per-entry lifecycle** | enum | On each `plan.yaml` entry: `pending → in-progress → done`. Build failures and merge conflicts leave the active entry `in-progress`; they do not stamp a separate blocked state in v1. |
| **slice lifecycle** | enum | On each `.specify/slices/<name>/.metadata.yaml`: `defining → defined → built → merged`. |

The slice-vs-change distinction in [`.cursor/rules/project.mdc`](../../.cursor/rules/project.mdc) survives on disk; only the slash-command layer collapses to `/spec:*`.

### Operator surface

The collapsed surface is three default verbs plus three step-through verbs. The default rhythm is `/spec:plan -> (review) -> /spec:execute -> (review on each stop) -> /spec:finalize`. The step-through verbs are always available; an operator reaches for them when `/spec:execute` stops on a gate or failure, or when they want to inspect a slice before continuing.

**Default verbs (always run, in order):**

| Stage | Command | Replaces |
|---|---|---|
| **Plan** (enumerate -> propose -> validate -> Gate 1) | `/spec:plan <scope> [source <key>=<path-or-url> ...]` | `/change:draft` (and `/change:survey` and `/change:analyze`, both retired by RFC-25) |
| **Drive the plan** (per slice: extract -> synthesise -> build -> merge; stops on build failure, merge conflict, or plan drained) | `/spec:execute` | `/change:execute loop` |
| **Push + observe PRs + archive plan** | `/spec:finalize <name>` | `/change:finalize` |

**Step-through verbs (breakouts; operate on the active slice):**

| Stage | Command | When reached for |
|---|---|---|
| **Refine one slice** (extract -> synthesise) | `/spec:refine` | To inspect / hand-edit `spec.md` before building when `/spec:execute`'s synthesis emitted `[conflict]` or `[unknown]` tags; or to author a slice manually. Renames today's `/spec:define`. |
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
         runs /spec:execute to drive the plan - or runs specify plan next
         followed by /spec:refine to step into the first slice manually)

/spec:execute                                  ---- SUPERVISED LOOP (DEFAULT) ------
  |-- refuse unless plan.lifecycle == reviewed
  |-- acquire plan lock (at the hub root in hub mode)
  |-- loop:
  |     specify plan next -> active slice, or next pending slice (entry -> in-progress)
  |     [hub only] resolve <project> via registry.yaml
  |     [hub only] sync workspace slot if missing
  |     [hub only] specify workspace prepare-branch <project> --change <scope>
  |     [hub only] chdir .specify/workspace/<project>/
  |     if slice lifecycle < defined:        invoke /spec:refine
  |                                          (writes spec.md with inline [conflict] / [divergence] /
  |                                          [unknown] tags when synthesis surfaces them; loop continues)
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
  |-- require an active slice already in-progress from `specify plan next`
  |     (refine never auto-selects or writes in-progress itself)
  |-- slice create .specify/slices/<name>/    (idempotent - no-op if present)
  |-- bound source.extract -> evidence/<source-key>.yaml
  |-- synthesise per RFC-25 §Synthesis contract; specify slice validate
  |     (synthesis writes [conflict] / [divergence] / [unknown] tags inline in spec.md when needed;
  |      no parking state, no synthesis halt — operator may review and hand-edit)
  +-- slice transition defined

/spec:build                                    ---- IMPLEMENTATION ------------------
  |-- refuse unless slice lifecycle is defined
  |-- do not refuse on unresolved [conflict] / [divergence] / [unknown] tags; they are review signals
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

1. **`specify plan next` is the only writer of the per-entry `in-progress` transition.** Both `/spec:execute`'s loop and an operator stepping in manually call it; `/spec:refine` never does. If an entry is already `in-progress`, `plan next` returns that active entry and does not advance. Only when no entry is active does it transition the next eligible `pending` entry to `in-progress`. This lets `/spec:refine` operate uniformly on "the active slice" without selecting work implicitly.
2. **`/spec:merge` is the only writer of the per-entry `done` transition.** Per-slice closure lives with the verb that produces the terminal state, not the loop driver, so a manual `/spec:merge` leaves the plan in exactly the state `/spec:execute` would have left it in - and a subsequent `/spec:execute` invocation just pulls the next entry.

**Stop / resume.** `/spec:execute` is a state-machine driver, not a session — it re-reads `plan.yaml.lifecycle`, calls `specify plan next`, and reads the active slice's `.metadata.yaml` on every invocation. Build failures and merge conflicts leave the entry `in-progress`; the next `/spec:execute` call sees the same active entry and resumes from its slice lifecycle. Breakout verbs leave all artifacts and transitions observable to the next `/spec:execute` call; there is no `--continue` flag.

| Trigger | What `/spec:execute` does | Operator next step |
|---|---|---|
| `/spec:build` returns non-zero | Exits with the failing task id and build log path | Fix and re-run `/spec:execute`, or step in with `/spec:build` |
| `/spec:merge` reports a baseline conflict | Exits with conflicting spec paths | Resolve, then `/spec:execute` |
| `specify plan next` reports drained | Exits cleanly; notes `/spec:finalize` ready | Run `/spec:finalize` |

Synthesis tags (`[conflict]` / `[divergence]` / `[unknown]`) do not stop the loop and do not cause `/spec:build` to refuse. They are printed in the per-slice transition message and emitted as journal events; the operator may interrupt and hand-edit before build, but v1 does not add a second gate or an `--allow-unresolved` flag.

### The plan gate

| Gate | Position | Reviewed | Mechanism | Skip |
|---|---|---|---|---|
| **Gate 1 - plan** | After `plan validate`, before any `extract` | Slice boundaries, `sources` per entry, `project` assignment, descriptions | `specify plan transition <scope> reviewed` (CLI-stamped; refuses progress until set) | None in v1 — supervised gate, no automation override flag |

Gate 1 is the structural successor to RFC-23's "explicit human seam" — same logical spot, now CLI-stamped on `plan.yaml.lifecycle`.

**No Gate 2 in v1.** Synthesis tags in `spec.md` are operator-review signals; lifecycle goes straight `defined → built`, and `/spec:build` refuses only on slice lifecycle preconditions. A structural second gate (`defined_provisional` parking state) returns when operator demand for a discrete review-then-promote ergonomics — automation hooks, CI gating, parking semantics for `[conflict]` and `[divergence]` tags — surfaces in real workflows. See §Non-Goals.

**Stepping in without `/spec:execute`:** After Gate 1, run `specify plan next`, then `/spec:refine` directly. `plan next` owns the `in-progress` transition; refine only consumes the active slice and exits if no entry is active.

### Combined lifecycle (RFC-25 + RFC-26)

```text
PLAN (plan.yaml)          SLICE (.metadata.yaml)           STAGE
──────────────────────────────────────────────────────────────────
pending                   —                              /spec:plan
  │ (operator)            —                              Gate 1: plan transition reviewed
reviewed                  —                              /spec:execute allowed
in-progress (plan)        defining                       extract + synthesise
  │                       defined                        synth wrote spec.md (with inline tags if any)
  │                       built                          /spec:build
  │                       merged                         /spec:merge → plan entry done
drained                   —                              /spec:finalize
```

### Planning at every scale

`/spec:plan` always runs `enumerate` and writes `plan.yaml`, even at N=1 — enumeration is **degenerate**, not absent. For greenfield work, `intent` binds implicitly, `intent.enumerate` emits one candidate, propose auto-accepts via `specify plan add`, and Gate 1 shows a one-line `Y` / `edit` / `n` prompt.

**Headless trivial path:** `specify plan create <scope>` + `specify plan add` + `specify plan transition reviewed` + `/spec:execute`.

`plan.yaml` and `change.md` survive at every slice count: the single-writer invariant on `plan.yaml` ([`plugins/change/references/plan-single-writer.md`](../../plugins/change/references/plan-single-writer.md)), audit trail in `archive/`, and the ability to grow N=1 into N=3 at Gate 1. A one-slice plan is simply small:

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

`change.md` is auto-scaffolded from the operator's brief at `/spec:plan` scaffold time and may be edited at Gate 1. For N=1 it may be a one-liner; the file's value is the audit trail. The change name is whatever the operator passes to `/spec:plan` — there is no separate `/change:draft <name>` step.

### Single-repo vs multi-repo

`project.yaml: hub:` is the only context discriminator the collapsed workflow needs:

| `hub:` | Behaviour in `/spec:plan` |
|---|---|
| `false` (regular project) | Single project root; `planSlice.project` omitted or defaulted; `sync-workspace` and `assignment` substeps skipped. |
| `true` (registry-only platform hub) | Reads `registry.yaml`; runs `sync-workspace` before enumerate (some sources are workspace-resident); `propose` asks per-candidate `--project` assignment. |

`specify workspace sync` stays as today; it is called from `/spec:plan`'s enumerate substage for hubs, before `source.enumerate` runs.

**One driving mode per project in v1.** A project is *either* hub-driven *or* standalone, not both. A project registered in `registry.yaml` is driven through the hub; running `/spec:plan` from its project root while a hub-driven plan is active is refused at plan-create time. This cuts an entire class of cross-root coordination edge cases (the "operators are responsible for not racing themselves" disclaimer, the `stale-workspace-clone` warning surface, the lock-holder PID visibility) from v1. The plan lock (held internally by `/spec:execute` and the breakout verbs) covers the per-root case. Cross-mode driving for the same project returns when a real consumer asks for it.

### Where pipeline stages live

| Stage | Before (Specify 1.x / pre-collapse) | After (Specify 3.0) |
|---|---|---|
| `source.enumerate` | `/change:draft` | `/spec:plan` |
| `source.extract` + core synthesis | `/spec:define` | `/spec:refine` |
| `target.build` / `target.merge` | `/spec:build`, `/spec:merge` | unchanged |

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

RFC-26 deltas on top of [RFC-25 §CLI surface](../rfc-25-workflow.md#cli-surface):

- `specify change draft` → `specify plan create`; `specify change finalize` → `specify plan finalize`
- **New transition:** `specify plan transition <change> reviewed` — Gate 1; `/spec:execute` refuses until set
- `specify change *` retires with no deprecation aliases (hard cut at 3.0)

The v1 floor is 18 verbs; every cut and deferred verb is listed in [`commands.md`](../commands.md). The collapse is almost entirely a skill / brief redesign — the CLI substrate barely moves and shrinks.

### Skill / SKILL.md changes

Skill bodies follow §Operator surface and §Internal structure; this table is the mechanical move-and-rename list.

| File | Action |
|---|---|
| `plugins/spec/skills/plan/SKILL.md` | **New.** Absorbs enumerate, propose, assignment, plan validate, Gate 1; lifts brief topology from `plugins/change/skills/draft/`. |
| `plugins/spec/skills/execute/SKILL.md` | **New.** Default driver renamed from `/change:execute loop`; stop/resume per §Internal structure. |
| `plugins/spec/skills/refine/SKILL.md` | **Renamed from `define/`**, rewritten as a step-through breakout sharing one skill body with `/spec:execute`'s loop. Driver-supplied source arguments retired in favour of plan-resolved bindings. |
| `plugins/spec/skills/build/SKILL.md` | **Step-through breakout.** Refuses unless slice is `defined`. |
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

## Implementation Plan

Strictly incremental on top of RFC-25. Land RFC-25 acceptance fixtures (especially synthesis and provenance) **before** step 3 below.

1. **Land RFC-25.** The collapse depends on symmetric `enumerate` / `extract`, core synthesis (including the authority hierarchy, `claim-id` fusion, and `[divergence]` emission), and the multi-source v1 floor. RFC-25 and RFC-26 ship in lockstep as Specify 3.0; this step is intra-release ordering, not a separate release.
2. **Promote the review seam inside `/change:draft`** as a no-behaviour-change refactor. Add `reviewed` to plan lifecycle; `specify plan transition <change> reviewed` stamps Gate 1.
3. **Rename `/change:execute loop` -> `/spec:execute` and add the §Internal structure stop/resume contract.** The loop algorithm is unchanged; what is new is that the skill stops on build failure and merge conflict with operator-facing hints, and resumes by re-reading on-disk state on the next invocation. `/change:finalize` becomes `/spec:finalize` (no behaviour change). **This is the load-bearing step** - the collapsed default workflow becomes `/spec:plan -> /spec:execute -> /spec:finalize` only once this step lands.
4. **Make `/spec:plan`, `/spec:execute`, `/spec:finalize` the documented default workflow** and `/spec:refine`, `/spec:build`, `/spec:merge` the documented step-through breakouts. Rewrite `AGENTS.md`, `.cursor/rules/project.mdc`, the README, the marketplace manifest, and the tutorial walkthrough. `/change:*` and `/spec:define` move to a "removed" section. Acceptance scenario #1 (Pure intent, one slice) is a release-blocker for this step — see §Acceptance scenarios — because single-release collapse means N=1 `/spec:plan` ergonomics surface to every operator at once with no 2.x discovery window.
5. **Delete `/change:draft`, `/change:execute`, `/change:finalize`, and `/spec:define`.** The `plugins/change/` directory is removed. `plugins/spec/skills/define/` is renamed to `plugins/spec/skills/refine/`.

### Acceptance scenarios

Run these against the collapsed skills before step 5. Each is an honest stress test of where the collapse can fail.

| # | Scenario | What it stress-tests |
|---|---|---|
| 1 | **Pure intent, one slice.** Operator runs `/spec:plan fix-typo "fix typo in user.rs"`. | Degenerate `intent.enumerate`; Gate 1 ergonomics on trivial work; `change.md` + `plan.yaml` justifiability at N=1. |
| 2 | **Documentation, one slice.** Operator binds a single docs path. | `documentation.enumerate` correctness at the new entry point. |
| 3 | **Documentation, multi-slice.** Operator binds docs that map to N candidates. | Propose/edit/reject loop; Gate 1 amendment flow. |
| 4 | **Legacy-code, multi-slice.** Operator binds a legacy repo. | `legacy-code-typescript.enumerate`; survey/repair loop under the new skill; under-slicing failure mode. |
| 5 | **Synthesis surfaces `[conflict]` inline.** Single-source slice where synthesis cannot reconcile an intra-pack contradiction. | `[conflict]` written into `spec.md`; lifecycle still transitions to `defined`; operator can hand-edit and run `/spec:build` without a parking-state ceremony. |
| 5a | **Combined evidence (legacy-code + documentation), one slice.** Operator binds a legacy repo and a design-notes path on the same slice. | RFC-25 §Synthesis contract end-to-end: serial `extract` per binding; `EvidenceSet` cardinality 2; `Sources:` line carrying both keys; `claim-id` correlation produces deterministic fusion; lifecycle reaches `defined` cleanly when sources agree. |
| 5b | **`[divergence]` from authority resolution.** Combined-evidence slice where docs and legacy code disagree at different authority classes (e.g. docs say "30 minutes" expiry, code observed 24 hours). | `Status: divergence` written; design-spec winner becomes the operative requirement; observed-behaviour preserved as inline commentary; lifecycle transitions to `defined`; operator may hand-edit before build. |
| 5c | **`[conflict]` from same-authority disagreement.** Combined-evidence slice where two `documentation` sources disagree on the same claim. | `Status: conflict` written with both values preserved as inline commentary; lifecycle still transitions to `defined`; operator must reconcile by editing or amending bindings before the requirement is meaningful. |
| 5d | **Optional binding fail-soft.** Combined-evidence slice with one `optional: true` binding whose `extract` fails. | Synthesis proceeds with the surviving packs; structured warning emitted; `Sources:` lines reflect surviving contributors only. |
| 6 | **Multi-repo assignment from a hub.** Operator runs `/spec:plan` in a hub. | `hub:` discriminator; per-candidate `--project` at propose; workspace sync timing. |
| 7 | **Operator amends one-slice plan into two slices at Gate 1.** | Plan amendment via `specify plan amend`; re-entry to Gate 1 after amend. |
| 8 | **Step-through breakout mid-execute.** Operator starts `/spec:execute`; on the second slice they cancel, run `/spec:build` directly to investigate, then re-invoke `/spec:execute`. | Stop/resume contract; that step-through verbs leave on-disk state consistent for `/spec:execute` to resume without flags. |
| 9 | **`/spec:execute` parks on a build failure, operator fixes, resumes.** Slice's `cargo test` fails; operator patches the crate; runs `/spec:execute`. | Build-failure stop hint; build resumes from the failed task; loop continues to merge. |
| 10 | **Hub `/spec:execute` across two projects.** Plan with slices targeting `project-a` and `project-b`; operator runs `/spec:execute` from the hub root. | Per-slice project routing; slot materialisation; `prepare-branch`; `chdir` + residue commit; plan-lock semantics at the hub root while phase work runs in slots. |
| 11 | **Hub breakout after build failure in a slot.** `/spec:execute` parks on `auth-rotate` (in `project-a`); operator stays at hub root and runs `/spec:build`. | Project-routing rule for breakout verbs; active-slice resolution across the hub/slot boundary; correct chdir without operator intervention. |
| 12 | **Dual-driving refused.** Project registered in a hub; operator runs `/spec:plan` from the project root with a hub-driven plan active. | One-driving-mode-per-project invariant (§Single-repo vs multi-repo). |

If any of #1-4 fail the ergonomics test (operator confusion, lost time, surprised state), revisit §Planning at every scale before pushing through step 5.

## Migration

Ships as Specify 3.0 with **no backward compatibility** ([RFC-25 §Migration](../rfc-25-workflow.md#migration)). Operators on 1.x install 3.0 directly; there is no 2.x intermediate release to pin against.

`migrate-to-3.0.sh` (release notes) absorbs both the RFC-25 adapter-axis renames and this RFC's workflow collapse in one pass: mechanical renames against `project.yaml`, `registry.yaml`, `plan.yaml`, `sources.yaml`, `.specify/.cache/`, and `.specify/archive/` (`yq` + `sed`); skill-directory renames (`change/*` → `spec/{plan,execute,finalize}`, `define` → `refine`); bumps `specify_version` to `3.0.0`; adds `reviewed` to plan lifecycle on first 3.0 read; updates marketplace manifest. Plugin cache re-fetches on next invocation. There is **no** `specify upgrade` verb. Dry-run the combined script against a real 1.x consumer fixture before tagging — the single-release blast radius is larger than either RFC alone.

**Skill authors:** `/change:*` and `/spec:define` retire; use `/spec:execute` and let stop conditions surface to the operator.

**Automation consumers:** Gate 1 = `plan.lifecycle == reviewed`. Synthesis warnings: read `[conflict]` / `[unknown]` from `spec.md` or subscribe to journal events (§Observability).

## Alternatives Considered

- **Ship RFC-25 as 2.0 first, RFC-26 as 3.0 second, with a `specify-3.0-preview` parallel plugin channel during 2.x** — earlier rejected on independent-concerns / blast-radius grounds; **reconsidered and reversed**. The "sequencing reduces blast radius" argument relied on a 2.x adopter population that would discover regressions early; with no such population in evidence, the two-release plan only doubled the migration script, kept a preview channel alive for nobody, and forced the "skill authors should not invest in `/change:*` changes during 2.x" caveat (RFC-25 §Migration, previous draft) — which is itself the strongest evidence the 2.x line had no productive lifetime. Single-release collapse preserves the intra-release ordering (RFC-25 implementation plan steps 1–13 precede this RFC's collapse) while removing the doubled scripts and the preview channel.
- **Overload a phase verb with the loop (`/spec:refine --loop`)** — loop drivers and per-slice phases stay distinct; `/spec:execute` is the supervised loop.
- **`/spec:plan` as a shell wrapper around `/change:draft + /change:execute`** — the collapse is a real brief refactor, not a shim.
- **Keep `/spec:define` as the slice authoring verb** — after collapse, "defining" is planning; per-slice work is refining a named slice.
- **Deprecation aliases for `/change:*`** — rejected; hard cut preserves rename clarity (RFC-25 stance).

See §Non-Goals for out-of-scope items.

## Non-Goals

- `/spec:execute` session tokens or a `--continue` flag. On-disk state (`plan.yaml.lifecycle`, slice `.metadata.yaml`) is the only resume mechanism; re-running with no flags is the contract.
- Folding `/spec:finalize` into `/spec:merge`. They stay distinct to avoid the merge-overload failure mode RFC-23 removed.
- Deleting or renaming `change.md` / `plan.yaml`. The single-writer invariant on these files is load-bearing; the collapse keeps both intact.
- Auto-resolution of `[conflict]` markers in `spec.md`. The operator decides.
- A "manual mode" where `/spec:execute` does not exist. The supervised loop is the documented default at every slice count, including N=1.
- Backward compatibility with pre-3.0 `/change:*` skills, `specify change *` verbs, or `/spec:define` invocations. All retire at the 3.0 hard cut.

**Deferred from v1, reinstated when a real caller asks:**

- A second structural gate between synthesis and build (`defined_provisional` parking state, `/spec:refine --resume` promotion verb, `--yes-gate2` automation flag). Multi-source synthesis (RFC-25) emits `[conflict]` and `[divergence]` tags inline today; the second gate returns when operator demand for a parking state — discrete review-then-promote ergonomics, automation hooks, CI gating — surfaces in real workflows.
- `/spec:execute` automation flags: `--yes-plan` (Gate 1 auto-clear), `--one`, `--until <slice>`, `--dry-run`, `--continue-on-build-fail`. v1 ships the supervised default loop with no flags.
- Cross-mode driving for the same project (a registered project driven both via its hub and standalone from its project root). v1 refuses the second mode at plan-create time. The `stale-workspace-clone` warning surface, the lock-holder PID visibility on a non-existent `specify plan status`, and the "operators are responsible for not racing themselves" disclaimer all return with the cross-mode feature, not before.

## Open Questions

1. **Slice directory creation timing.** Create `.specify/slices/<name>/` at `plan add` or only when `/spec:refine` starts extract? Current preference: at extract, to keep Gate 1 plan-pure.
2. **Lifecycle enum wire format.** Current preference: snake_case in `.metadata.yaml` and JSON (`defining`, `defined`, `built`, `merged`).

## Observability ([RFC-19](../rfc-19-observability.md))

Emit journal events (complete by 3.0):

| Event | When |
| ----- | ---- |
| `plan.transition.reviewed` | Gate 1 cleared |
| `slice.transition.defined` | Synthesis completed |
| `slice.synthesis.conflict` | `[conflict]` markers written into `spec.md` |
| `slice.synthesis.divergence` | `[divergence]` markers written into `spec.md` |
| `slice.synthesis.unknown` | `[unknown]` markers written into `spec.md` |

Enables CI and hosted runners to observe planning and synthesis without parsing skill exit codes.

## References

- [RFC-19: Observability](../rfc-19-observability.md) — journal events for the plan gate and synthesis outcomes (§Observability).
- [RFC-25: Directional Adapters](../rfc-25-workflow.md) — adapter-axis prerequisite; §Synthesis contract, multi-source v1 floor, and pipeline split. The deferred second gate is decoupled from multi-source — see §Non-Goals.
- [RFC-23: Change Lifecycle (archived)](rfc-23-change-lifecycle.md) - the three-skill model RFC-26 supersedes. Gate 1 is the structural successor to RFC-23's "explicit human seam"; the seam itself survives, the verb names do not.
- [RFC-22: Migration Ledger and Slice Mapping](../rfc-22-ledger.md) - unaffected by the collapse; the per-change ledger continues to live alongside `change.md` and `plan.yaml`.
- [RFC-24: Omnia Plan Composition](../rfc-24-omnia.md) - unaffected; per-slice composition lives on `planSlice` regardless of which verb wrote it.
- [`plugins/spec/skills/init/SKILL.md`](../../plugins/spec/skills/init/SKILL.md) - `hub:` discriminator, the only context primitive the collapse needs.
- [`plugins/change/references/plan-single-writer.md`](../../plugins/change/references/plan-single-writer.md) - the single-writer invariant preserved by the collapse. Note: this reference will move when `plugins/change/` is removed by step 5 of §Implementation Plan; the invariant survives, the path does not.
- [`AGENTS.md`](../../AGENTS.md) §Plan-driven loop - vocabulary this RFC substantially rewrites.
- [`.cursor/rules/project.mdc`](../../.cursor/rules/project.mdc) §Vocabulary - slice vs change distinction the collapse blurs at the verb level but preserves on disk.
