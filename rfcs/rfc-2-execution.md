# RFC-2: Execution

> Status: Draft · Depends: [RFC-1](rfc-1-cli.md)

## Abstract

Drive complex, multi-change initiatives through Specify's define-build-merge loop using a **Plan** (`plan.yaml`) — an ordered, dependency-aware list of changes with status tracking and progressive baseline accumulation. The plan format supports greenfield builds, legacy migrations, and platform modernisations; the only difference is where the input to `/spec:define` comes from.

This RFC is structured in two layers. **Layer 1** (the MVP) delivers the plan format and CLI commands — enough for a human to drive a plan-based initiative using the existing skill chain. **Layer 2** adds the `/spec:execute` driver skill that automates the loop. Layer 1 is immediately useful without Layer 2, and the manual Layer 1 commands remain available as fallback under Layer 2.

## Execution Model Overview

![Specify Framework execution model](assets/specify-framework.png)

Specify at runtime is a three-phase loop (**define → build → merge**) driven by the `/spec:execute` skill over a long-lived **Plan** (`plan.yaml`). Per change, `/spec:execute` performs `get next change`, invokes the three phase skills in sequence, and updates `status` on the currently-active change entry. Each phase runs a *brief pipeline* declared by the active `schema.yaml`, and each brief delegates to one or more plugin skills. When a phase needs to add or modify change *entries* in the Plan, it invokes `/spec:plan` directly. In Layer 1 (no `/spec:execute`), the human plays the driver role and runs `specify plan create` / `specify plan amend` on the CLI instead. Both paths funnel into the same library entrypoints (`Plan::create`, `Plan::amend`), so the single-writer-of-entries property holds across layers.

The diagram above is schema-agnostic: the `<briefs>` stacks and `<skills>` stacks inside each phase box are placeholders that a schema fills in. A concrete instantiation for the Omnia schema is shown in the table further down this section. Swapping the schema swaps the brief set, which swaps the plugin skill delegations inside each phase box; the surrounding structure is invariant.

The diagram applies to both layers of this RFC. In Layer 1 a human plays the `execute` role and the `create/amend` role by running `/spec:define`, `/spec:build`, `/spec:merge`, `specify plan transition`, and (when a new entry is needed) `specify plan create`/`specify plan amend` directly. There is no `/spec:plan` skill in Layer 1 — the CLI commands *are* the Layer 1 surface of the `create/amend` box. In Layer 2, `/spec:execute` performs the same loop automatically and phases invoke `/spec:plan` mid-run; the manual commands remain available as fallback. RFC-2's trajectory is toward full automation with manual fallback preserved.

Phase outcomes (`success`/`failure`/`deferred`) are implicit in the return path of the `execute change` edge and are not drawn; their on-disk transport (a terminal `type: outcome` entry in `journal.yaml`) is specified in [§Phase Outcome Contract](#phase-outcome-contract). `create/amend` is a named skill invoked by phases during a phase run and carries no outcome. Artifact flow between phases (define's outputs → build's inputs → merge's inputs) is a separate concern, covered in [§Context Threading](#context-threading).

### The six moving parts

- **Plan (`plan.yaml`).** The ordered, dependency-aware list of changes with status. Specified in full in §"The Plan" below. The Plan is the only artifact that persists across changes.
- **Driver skill (`/spec:execute`).** Reads the Plan, selects the next eligible change, invokes the three phase skills in sequence, and updates `status` on the currently-active change entry via `specify plan transition`. Specified in full in §"Layer 2: Automated Execution" below. Does not create or amend change *entries* — that is done by `/spec:plan` (Layer 2) or by humans running the `specify plan create` / `specify plan amend` CLI commands (Layer 1).
- **Plan-mutation skill (`/spec:plan`, Layer 2).** The skill phases invoke during a phase run when they need to add or modify change entries (for example, when define discovers a neighbouring defect). Writes via the same `Plan::create` / `Plan::amend` library entrypoints as the Layer 1 CLI, so the single-writer-of-entries property holds regardless of caller.
- **Drop skill (`/spec:drop`).** A peer control-plane skill, invoked by `/spec:execute` on `failure` or `deferred` to clean up partial artifacts for the currently-active change. It is not on the framework diagram (see §"Diagram label → skill/CLI counterpart"), not a phase skill, not a brief, and not invoked by phases directly.
- **Phase skills (`/spec:define`, `/spec:build`, `/spec:merge`).** Each phase loads the brief pipeline named by the active `schema.yaml` (see [RFC-1](rfc-1-cli.md) §`brief.rs` and `PipelineView`) and runs every brief in declared order, honouring each brief's `needs` dependencies.
- **Briefs and plugin skills.** A brief is a markdown file with YAML frontmatter (`id`, `needs`, `generates`, `tracks`) that configures one step of a phase. A brief's body is instructions for the agent — typically "invoke these plugin skills in this order." For example, the Omnia `build.md` brief delegates to `guest-writer`, `crate-writer`, `test-writer`, and `code-reviewer`; the Vectis `build.md` brief delegates to the equivalent Vectis writers and reviewers. The driver skill, plan-mutation skill, drop skill, and phase skills are unchanged across schemas.

### Instantiating the diagram (Omnia example)

Inside each phase box, the `<briefs>` stack represents the briefs that make up that phase's brief pipeline and the `<skills>` stack represents the plugin skills those briefs delegate to. For the Omnia schema these resolve as:

| Phase | Briefs (pipeline) | Plugin skills invoked by the briefs |
|---|---|---|
| define | `proposal.md`, `specs.md`, `design.md`, `tasks.md` | `/spec:extract` (when `sources` present; invoked from `proposal.md` / `specs.md`, which in turn uses `git-cloner` and `analyze`) |
| build | `build.md` | `/omnia:guest-writer`, `/omnia:crate-writer`, `/omnia:test-writer`, `/omnia:code-reviewer` |
| merge | `merge.md` | — (no plugin skills; the brief body drives git operations directly) |

Omnia's `pipeline.define` has four briefs, while `pipeline.build` and `pipeline.merge` each have a single brief. The same document name can appear both as a brief (under `schemas/<name>/briefs/`) and as an artifact (under `.specify/changes/<name>/`) — the brief's `generates` frontmatter names the artifact it produces.

The dashed `schema.yaml` arrows on the diagram are the binding that tells each phase skill which briefs (and, transitively, which plugin skills) to load. Swapping the schema is how the same diagram instantiates a different stack — Omnia today, Vectis or any future schema tomorrow — while the Plan and the skills around it are invariant.

> **Note on extract.** Extraction is not a fourth phase. It is work done inside the define phase — in the Omnia instantiation above it appears as the `git-cloner` and `analyze` plugin skills invoked by the define briefs (via `/spec:extract`) when the plan entry has `sources`. On the schema-agnostic diagram this is simply one of the `<skills>` invoked within define.

#### Diagram label → skill/CLI counterpart

Each diagram label has an explicit skill and/or CLI counterpart:

| Diagram label     | Skill counterpart                              | CLI counterpart                                                                                                    |
| ----------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `get next change` | (argument to `/spec:execute`)                  | `specify plan next`                                                                                                |
| `execute`         | `/spec:execute`                                | — (Layer 1 humans run the phase skills manually)                                                                   |
| `execute change`  | `/spec:define` → `/spec:build` → `/spec:merge` | —                                                                                                                  |
| `create/amend`    | `/spec:plan` (Layer 2 only)                    | `specify plan create`, `specify plan amend` (Layer 1 surface; and `specify plan transition` for state updates on the current entry) |
| Phase boxes       | `/spec:define`, `/spec:build`, `/spec:merge`   | —                                                                                                                  |
| `schema.yaml`     | —                                              | — (read by phase skills at load time)                                                                              |
| (not drawn)       | `/spec:drop`                                   | — (Layer 1: invoked by humans on failure/deferral; Layer 2: invoked by `/spec:execute`)                            |

### Data flow

The Plan flows downward (from the Plan box into the Workflow box on the framework diagram): `/spec:execute` reads it (dashed `get next change` arrow on the diagram), picks a change, and runs define → build → merge in turn. Each phase reads the artifacts of the previous phase and writes new artifacts into `.specify/changes/<name>/` according to its briefs' `generates` globs.

The Plan also flows upward (from the Workflow box back into the Plan box, via the `create/amend` arrow on the diagram). In Layer 2 this goes through a single named skill: any phase that needs to add a new change entry or amend an existing one invokes `/spec:plan` directly, and `/spec:plan` writes `plan.yaml` synchronously during the phase run. In Layer 1 the same write goes through `specify plan create` / `specify plan amend` run by a human. Both paths funnel into the same `Plan::create` / `Plan::amend` library functions, so the single-writer-of-entries property is enforced at the library layer regardless of who calls it. State updates on the currently-active change entry (e.g. `in-progress → done`) are performed by `/spec:execute` (Layer 2) or by a human (Layer 1) via `specify plan transition`. No other code path writes `plan.yaml`. The `registration-duplicate-email-crash` entry elsewhere in this RFC is an example of a phase-invoked `/spec:plan` call that adds a new entry; see [§Worked Example](#worked-example-phase-invoked-specplan) for the end-to-end trace.

### Why this matters for the RFC

Three invariants implied by the diagram remove ambiguity before implementation:

1. **`/spec:execute`'s contract is with phases, not briefs.** It supplies arguments to `/spec:define`, `/spec:build`, and `/spec:merge` — not to individual briefs or plugin skills. Decisions inside a brief (e.g. which plugin skill to re-enter during a repair loop) are the phase's problem.
2. **Phases own their verify-repair loops; only phase-level outcomes cross the phase boundary.** A brief-level failure (for instance a failed `cargo test` inside the Omnia `build.md` verify-repair loop) does not surface to `/spec:execute` until the phase skill has exhausted its own repair budget. `/spec:execute` sees exactly one of `success`, `failure`, or `deferred` per phase invocation, with the phase responsible for summarising what went wrong.
3. **Entry writes go through one library entrypoint; state writes go through another.** Change *entries* are added or amended via `Plan::create` / `Plan::amend` — surfaced as `specify plan create` / `specify plan amend` on the CLI and (in Layer 2) as the `/spec:plan` skill that phases invoke during a phase run. `status` updates on the currently-active entry are made via `Plan::transition` — surfaced as `specify plan transition` and used by humans (Layer 1) and `/spec:execute` (Layer 2). The single-writer property of `plan.yaml` is enforced at the library layer, not by a propose/apply discipline, and applies equally to Layer 1 and Layer 2.

## Motivation

Complex initiatives — greenfield builds, legacy migrations, platform modernisations — lack a coordination artifact. The agent rediscovers scope, ordering, and dependencies on every iteration. There's no persistent plan that tracks what's done, what's next, and what's blocked.

The define-build-merge loop already works for individual changes. What's missing is the layer above: a plan that sequences changes, tracks dependencies between them, and lets progress accumulate in the baseline across iterations. Without it, every iteration starts from scratch — the agent doesn't know what came before, what's in flight, or what's blocked.

By expressing the initiative as an ordered list of changes with dependency constraints, the plan turns a sprawling effort into a series of self-contained Specify changes, each building on the baseline left by the last.

## Dependency on RFC-1

Plan parsing, validation, and status transitions are deterministic operations that belong in the CLI ([RFC-1](rfc-1-cli.md)). The skill-level loop (define → build → merge) already works today; what this RFC adds is the plan-driven coordination layer, implemented as `specify plan` subcommands on top of the CLI foundation.

---

## Layer 1: Plan Format + CLI (MVP)

### The Plan

A plan is an ordered list of the changes to implement, along with their dependencies and status. It is the initiative's table of contents: it tells the loop what to do next without requiring the agent to rediscover scope on every iteration.

```yaml
# .specify/plan.yaml
name: platform-v2

# Optional — only for migration/extraction use cases.
# Named source repositories. Changes reference these by key in their
# `sources` list. File-level scoping within a source is deferred to
# the define step (using extract skill).
# NOTE: source-aware execution is a Layer 2 capability. The Layer 1
# MVP parses and validates source references but does not wire them
# into the define step automatically; Layer 2's /spec:execute
# resolves the paths and hands them to /spec:define.
sources:
  monolith: /path/to/legacy-codebase
  orders: git@github.com:org/orders-service.git
  payments: git@github.com:org/payments-service.git
  frontend: git@github.com:org/web-app.git

changes:
  - name: user-registration
    sources: [monolith]              # which sources to analyze
    status: done                     # pending | in-progress | done | blocked | failed | skipped

  - name: email-verification
    sources: [monolith]
    depends-on: [user-registration]
    status: in-progress

  - name: registration-duplicate-email-crash
    affects: [user-registration]           # which changes/capabilities this touches
    description: >
      Duplicate email submission returns 500 instead of 409.
      Discovered during email-verification extraction.
    status: pending

  - name: notification-preferences
    depends-on: [user-registration]        # no sources → greenfield
    description: >
      Greenfield — user-facing notification channel and frequency settings.
    status: pending

  - name: extract-shared-validation
    affects: [user-registration, email-verification]
    description: >
      Pull duplicated input validation into a shared validation crate
      before building checkout-flow.
    depends-on: [email-verification]
    status: pending

  - name: product-catalog
    sources: [monolith]
    depends-on: [extract-shared-validation]
    status: pending

  - name: shopping-cart
    sources: [orders]
    depends-on: [product-catalog, user-registration]
    status: pending

  - name: checkout-api
    sources: [payments]
    depends-on: [shopping-cart]
    status: failed
    status-reason: >
      Type mismatch between cart line-item schema and payment gateway contract.
      Needs design revision after shopping-cart specs are updated.

  - name: checkout-ui
    sources: [frontend]
    depends-on: [checkout-api]
    status: pending
```

### Status State Machine

```
                            ┌─────────┐
              ┌── flag ────►│         │◄── defer ─── in-progress
              │             │ blocked │              (via /spec:drop)
              │    ┌────────┤         │
              │    │ unflag └─────────┘
              │    ▼
    ┌─────────┴────┐  select    ┌─────────────┐  merge   ┌──────┐
───►│   pending    ├───────────►│ in-progress ├─────────►│ done │
    └──────┬───────┘            └──────┬──────┘          └──────┘
           │  ▲                        │
           │  │ retry                  │ drop (failure)
           │  │                        ▼
           │  │                 ┌──────────┐
           │  └─────────────────┤  failed  │
           │                    └─────┬────┘
           │  exclude    abandon      │
           │  ┌───────────────────────┘
           ▼  ▼
    ┌───────────┐
    │  skipped  │──── re-include ────► pending
    └───────────┘
```

- **`pending`** — not started; eligible for selection by `plan next` if all `depends-on` entries are `done`
- **`in-progress`** — a Specify change has been created; define/build/merge is underway
- **`done`** — change merged successfully; specs are in baseline. **Terminal**: no transitions leave `done`. Corrections to merged behaviour are made by adding a new plan entry with `affects` referencing the `done` entry.
- **`blocked`** — manually flagged as unable to proceed (with a reason in `status-reason`). Dependency ordering is *not* modelled as `blocked` — `plan next` enforces `depends-on` at query time by only returning `pending` changes whose dependencies are all `done`
- **`failed`** — attempted but unsuccessful; the Specify change was dropped. Distinct from `skipped`, which is a deliberate exclusion
- **`skipped`** — deliberately excluded from this initiative (with a reason in `status-reason`); never attempted or no longer needed

#### Transition Rules


| Transition | Trigger | Who |
| ---------- | ------- | --- |
| `pending → in-progress` | Specify change directory created for this entry | `specify plan transition` (user or `/spec:execute`) |
| `pending → blocked`     | Flagged with a reason — design uncertainty, external dependency, etc. | `specify plan transition` (manual) |
| `pending → skipped` | Deliberately excluded before attempting | `specify plan transition` (manual) |
| `blocked → pending` | Flag removed | `specify plan transition` (manual) |
| `in-progress → done` | Specify change reaches `merged` (`/spec:merge` completes) | `specify plan transition` (user or `/spec:execute`) |
| `in-progress → failed` | Build or test failure; Specify change is dropped (`/spec:drop`) | `specify plan transition` (user or `/spec:execute`) |
| `in-progress → blocked` | Needs human decision mid-change; Specify change is dropped (`/spec:drop`). Layer 1: human parks the change. Layer 2: `/spec:execute` defers automatically. | `specify plan transition` (user or `/spec:execute`) |
| `failed → pending` | User decides to retry; a fresh Specify change will be created on next selection | `specify plan transition` (manual) |
| `failed → skipped` | User decides not to retry | `specify plan transition` (manual) |
| `skipped → pending` | Previously excluded change re-included | `specify plan transition` (manual) |


Only **one** change may be `in-progress` at a time per plan (single-threaded loop). `plan next` refuses to return a new change while one is already `in-progress`.

On failure, the Specify change is **dropped** via `/spec:drop`, cleaning up partial artifacts. On retry (`failed → pending`), a fresh change is created when the entry is next selected.

#### Mapping to Specify LifecycleStatus

The plan tracks coarse outcome; the Specify change tracks internal lifecycle (the `LifecycleStatus` enum defined in the `specify-change` crate — see [RFC-1](rfc-1-cli.md) §`lifecycle.rs` for the authoritative value list). When `/spec:execute` reads a change's `LifecycleStatus` to decide which loop step to run next, the plan only records whether the change is finished.


| Plan status | Specify change state |
| ----------- | -------------------- |
| `pending` / `skipped` | No Specify change on disk |
| `blocked` / `failed` | No *active* Specify change — prior attempts (if any) live under `.specify/changes/archive/<name>-<timestamp>/` and are consulted on retry (see [§Prior-attempt context on retry](#failure-and-resumption)) |
| `in-progress` | Change exists — `LifecycleStatus` ∈ {`defining`, `defined`, `building`, `complete`} |
| `done` | Change reached `merged` and was archived |

#### `affects` vs `depends-on`

- **`depends-on`** — ordering constraint. "Don't start this until those are `done`." Consumed by `specify plan next`.
- **`affects`** — impact annotation. "This change modifies behaviour defined by those changes." In the MVP, `affects` is parsed, validated (targets must exist as plan entries), and reported by `specify plan status` for impact visibility. Wiring `affects` into the define step as automatic delta-target resolution is a Layer 2 capability.

**Scope.** `affects` targets are *plan entries* only — names that appear elsewhere in `changes`. Baseline capabilities that were merged by a prior plan (or never went through a plan at all) cannot be referenced from `affects` in this RFC; delta targeting against baseline-only capabilities is a future capability. `specify plan validate` enforces this: an `affects` target that does not resolve to a plan entry is a hard error.

**Targeting pre-plan baseline (workaround).** If an initiative needs to modify a baseline capability that predates the current plan, add a synthetic `status: done` entry to `changes` with the capability's name, then reference it from `affects`. The entry stands in for the external baseline; because it never transitions out of `done` it never triggers selection by `plan next`. This pattern is advisory only — there is no schema flag distinguishing synthetic-baseline entries from real ones, and `specify plan validate` does not warn on orphan-change-dir for `done` entries (see [§`specify plan validate`](#specify-plan-validate)).

**Status of `affects` targets.** `specify plan validate` emits a *warning* (not an error) when an `affects` entry points at a target whose status is anything other than `done` or `in-progress`. Targeting a `skipped`, `failed`, `blocked`, or `pending` entry is semantically odd — the referenced behaviour isn't in the baseline yet — but the annotation is allowed because authors often draft plans top-down. The warning exists so reviewers can triage the intent.

**Relationship to `.metadata.yaml:touched_specs`.** `affects` is a plan-level impact annotation; `touched_specs` is a per-change metadata field populated by define based on which baseline specs the generated artifacts actually edit. The two are related but not automatically synchronised in MVP — define is not required to seed `touched_specs` from `affects`, and `specify plan validate` does not cross-check them. Automatic seeding is a Layer 2 / Future concern.

#### Fields

| Field | Required | Purpose |
| ----- | -------- | ------- |
| `name` | Yes | Kebab-case identifier; becomes the Specify change directory name. Must be unique across the entire plan. |
| `status` | Yes | Current state in the status state machine |
| `depends-on` | No | List of change names that must be `done` before this change is eligible |
| `description` | No | Free-text scoping hint; guides the define step when scoping. Distinct from the operational `status-reason` field below. |
| `sources` | Yes | Which source repos to analyze; keys reference the top-level `sources` map. Absent or `[]` → greenfield (both forms are equivalent; validate does not distinguish them). Parsed and validated in Layer 1; source-aware execution in Layer 2. |
| `affects` | No | Which existing changes or capabilities are touched. Parsed and validated in Layer 1; automatic delta-target wiring in Layer 2. |
| `status-reason` | No | Why the change failed/is blocked/is skipped; populated when `status = failed`/`blocked`/`skipped` |

`status-reason` holds the operational explanation for the current non-terminal/terminal status (`failed`, `blocked`, or `skipped`) and is overwritten on each status transition. `description` is kept exclusively for scoping intent so the define step has a stable hint that is not clobbered by operational bookkeeping. `specify plan transition --reason "..."` writes to `status-reason`.

### The Loop (Human-Driven)

In Layer 1, the human plays the `/spec:execute` driver role (there is no `/spec:plan` skill yet — its CLI counterparts `specify plan create` / `specify plan amend` stand in). The CLI provides the coordination primitives; the human drives the skill chain:

```text
specify plan status                              # where are we?
specify plan next                                # what's eligible?
specify plan transition <name> in-progress       # claim it

/spec:define <name>                              # existing skill
/spec:build <name>                               # existing skill
/spec:merge <name>                               # existing skill

specify plan transition <name> done              # record completion
```

On failure:

```text
/spec:drop <name>                                # existing skill
specify plan transition <name> failed --reason "..."
```

On deferral (the human hits an ambiguous requirement or design question mid-change and wants to park it for later resolution):

```text
/spec:drop <name>                                # same drop path as failure
specify plan transition <name> blocked --reason "Needs channel scope decision before spec is complete"
```

`blocked` differs from `failed` only in intent — a blocked change expects a human decision (then `blocked → pending` to retry), while `failed` expects remediation of an error (then `failed → pending`).

When a phase uncovers a neighbouring change that should be added to the plan (or an edit to an existing entry), the human runs the `specify plan create` / `specify plan amend` CLI commands directly (these are the Layer 1 surface of the single-writer-of-entries property; Layer 2 adds the `/spec:plan` skill that phases invoke with the same contract):

```text
specify plan create registration-duplicate-email-crash \
    --affects user-registration \
    --description "Duplicate email submission returns 500 instead of 409."
```

Each iteration is a self-contained Specify change. The user runs the same `/spec:define` → `/spec:build` → `/spec:merge` chain they would run for any single change — the only difference is that the plan decides what to do next and progress is tracked across iterations. The diagram in §"Execution Model Overview" applies here as well: Layer 1 is simply the variant in which the human plays `execute` and runs the `create/amend` CLI commands manually.

When no plan exists, the loop runs in **ad-hoc mode**: the user picks the next change interactively at the start of each iteration, just like picking what to `/spec:define` next in normal development.

### Progressive Baseline Accumulation

The key mechanism is **baseline growth through merge**. After each iteration:

- The completed change's specs join `.specify/specs/` as baseline
- Subsequent iterations can reference these specs (e.g., the cart feature can reference the product-catalog specs that were merged in a prior iteration)
- The `touched_specs` conflict detection in `.metadata.yaml` prevents two in-flight changes from stomping on each other
- The archived changes in `.specify/changes/archive/` provide a complete audit trail

```text
Iteration 1:  baseline = {}
              define(user-registration) → build → merge
              baseline = { user-registration }

Iteration 2:  baseline = { user-registration }
              define(registration-duplicate-email-crash) → build → merge
              baseline = { user-registration (patched) }

Iteration 3:  baseline = { user-registration }
              define(notification-preferences) → build → merge
              baseline = { user-registration, notification-preferences }

Iteration 4:  baseline = { user-registration, notification-preferences }
              define(product-catalog) → build → merge
              baseline = { user-registration, notification-preferences, product-catalog }

...

Iteration N:  baseline = { all changes }
              Initiative complete. Every change under spec governance.
```

This works identically for all changes. New capabilities add specs to the baseline, while changes with `affects` produce delta specs against existing baseline entries. The baseline doesn't care where the specs originated — it only cares that they passed through the define-build-merge loop.

### CLI Commands

#### `specify plan validate`

Structural validation of `plan.yaml`. This command is the CLI surface over `Plan::validate` — it rolls both structural checks and plan-to-change consistency into a single pass.

- **Duplicate names** — every `name` must be unique
- **Cycle detection** — the `depends-on` graph must be a DAG (topological sort via `petgraph`)
- **Referential integrity** — every `depends-on` target, every `affects` target, and every `sources` key must reference an existing entry
- **Status values** — every `status` must be a valid state machine value
- **Single in-progress** — at most one change may have status `in-progress`
- **Plan-to-change consistency** — any `in-progress` entry must have a corresponding `.specify/changes/<name>/` directory; report orphaned changes (directories without plan entries) as warnings. Skipped automatically when `validate` is run without access to the workspace changes directory (e.g. schema-only checks in CI).

**Not validated by MVP** (deliberately left out of Layer 1 to prevent scope creep in build):

- Whether `affects` targets are `done` or later — `affects` is an annotation, not an ordering constraint, and is purely informational at MVP.
- Whether `sources` keys resolve to reachable paths — path resolution is a Layer 2 concern (source-aware execution).
- Whether `affects` annotations agree with `.metadata.yaml:touched_specs` — see §`affects` vs `depends-on`.
- Reconciling `done`/`failed`/`skipped`/`blocked` entries against their change directories. Only `in-progress` entries are reconciled; `done` entries with missing directories are not an error (they should be archived under `.specify/changes/archive/`).

**Output.** Human-readable text by default; structured JSON with `--format json`. The JSON shape is a flat array of `{level: "error"|"warning", code: "...", message: "...", entry: "..."?}` records. The `code` vocabulary (`duplicate-name`, `cycle`, `unknown-dep`, `unknown-affects`, `affects-status`, `unknown-source`, `invalid-status`, `multiple-in-progress`, `missing-change-dir`, `orphan-change-dir`) is stable and machine-consumable.

**Plan schema.** Two JSON Schemas are published alongside the CLI release and kept versioned in-tree under `schemas/plan/`:

1. `plan.schema.json` — the schema for `.specify/plan.yaml` itself, for editor integration (`# yaml-language-server: $schema=...` header) and author-time validation.
2. `plan-validate-output.schema.json` — the schema for the `--format json` output above, for CI and dashboard consumers.

#### `specify plan next`

Return the next eligible change: a `pending` change whose `depends-on` entries are all `done`. Selection among multiple eligible changes follows list order (first eligible wins).

- If a change is `in-progress`, refuse and report which change is active.
- If no changes are eligible, report whether this means "all done" or "remaining changes are blocked/failed/pending-on-dependencies."

#### `specify plan status`

Initiative progress report:

- Total changes, grouped by *every* status in the state machine (`done`, `in-progress`, `pending`, `blocked`, `failed`, `skipped`); zero-counts are shown so downstream consumers can rely on a fixed shape
- Current `in-progress` change (if any), with its Specify `LifecycleStatus` from `.metadata.yaml`
- Blocked/failed entries with their reasons
- Next eligible changes (what `plan next` would return)
- Impact report: which `done` changes are referenced by `affects` entries still pending
- Display in dependency order (topological sort), not list order

**Cycle handling.** `specify plan status` is a diagnostic tool and must work even when `specify plan validate` reports a cycle. When the `depends-on` graph is cyclic, `status` falls back to list order, adds a prominent banner at the top (`⚠ plan has a dependency cycle — running in list order; run \`specify plan validate\` to see which entries are involved`), and includes the offending entry names in the JSON output under `cycle: [<name>, ...]`. Any other structural error short-circuits with a clear pointer to `validate`.

**Output.** Human-readable text by default; structured JSON with `--format json`. The JSON shape mirrors the sections above so CI and dashboards can consume it without parsing prose.

#### `specify plan create` and `specify plan amend`

```
specify plan create <name> [--depends-on <name>...] [--affects <name>...] \
    [--sources <key>...] [--description "..."] [--kind <label>]
specify plan amend  <name> [--depends-on <name>...] [--affects <name>...] \
    [--sources <key>...] [--description "..."] [--kind <label>]
```

CLI counterparts of the `/spec:plan` skill. They are the only commands (other than `specify plan transition`) that write `plan.yaml`. `create` adds a new entry with `status: pending`; `amend` edits non-status fields on an existing entry. Both validate the resulting plan structurally before writing.

#### `specify plan transition`

```
specify plan transition <name> <target-status> [--reason "..."]
```

Validated status transitions. The command:

1. Reads `plan.yaml`
2. Validates the transition is legal per the state machine
3. Updates the entry's `status`
4. If `--reason` is provided, writes it to `status-reason` (valid when the target status is `failed`, `blocked`, or `skipped`). `description` is never touched by `transition`.
5. Writes the plan atomically
6. Outputs the new state

All plan *status* mutations go through this command, ensuring the state machine is always enforced. `/spec:execute` uses the same command (or the underlying `specify-change` crate function) rather than editing YAML directly.

### Library Implementation

The plan state machine is encoded in the `specify-change` crate (see [RFC-1](rfc-1-cli.md) `Workspace Layout`), alongside the existing `LifecycleStatus`:

```rust
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanStatus {
    Pending,
    InProgress,
    Done,
    Blocked,
    Failed,
    Skipped,
}

impl PlanStatus {
    pub fn can_transition_to(&self, target: &Self) -> bool {
        use PlanStatus::*;
        matches!(
            (self, target),
            (Pending, InProgress)
                | (Pending, Blocked)
                | (Pending, Skipped)
                | (InProgress, Done)
                | (InProgress, Failed)
                | (InProgress, Blocked)
                | (Blocked, Pending)
                | (Failed, Pending)
                | (Failed, Skipped)
                | (Skipped, Pending)
        )
    }

    pub fn transition(
        &self,
        target: PlanStatus,
    ) -> Result<PlanStatus, Error> {
        if self.can_transition_to(&target) {
            Ok(target)
        } else {
            Err(Error::PlanTransition {
                from: self.clone(),
                to: target,
            })
        }
    }
}
```

Dependency resolution uses `petgraph` for topological sort and cycle detection. The `plan.rs` module (in `specify-change`, alongside the lifecycle state machine) provides:

```rust
pub struct Plan {
    pub name: String,
    pub sources: BTreeMap<String, String>,
    pub changes: Vec<PlanChange>,
}

pub struct PlanChange {
    pub name: String,
    pub status: PlanStatus,
    pub depends_on: Vec<String>,
    pub affects: Vec<String>,
    pub sources: Vec<String>,
    pub description: Option<String>,
    pub failure_reason: Option<String>,
    pub block_reason: Option<String>,
    pub skip_reason: Option<String>,
    pub kind: Option<String>,
}

/// Patch applied by `Plan::amend` to an existing entry. Every field is
/// `Option<T>`; `None` means "leave unchanged", `Some(v)` means "replace
/// with v". `status` is deliberately absent — status transitions are made
/// via `Plan::transition`, never through `amend`.
#[derive(Debug, Default, Clone)]
pub struct PlanChangePatch {
    pub depends_on: Option<Vec<String>>,
    pub affects: Option<Vec<String>>,
    pub sources: Option<Vec<String>>,
    pub description: Option<Option<String>>,
    pub kind: Option<Option<String>>,
}

/// Severity of a validation finding. `Error` means the plan is ill-formed;
/// `Warning` means the plan is structurally valid but flags a known
/// transient or advisory condition (e.g. an orphan change directory).
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationLevel { Error, Warning }

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub level: ValidationLevel,
    pub code: &'static str,
    pub message: String,
    pub entry: Option<String>,
}

impl Plan {
    pub fn load(path: &Path) -> Result<Self, Error>;
    pub fn save(&self, path: &Path) -> Result<(), Error>;

    /// Structural + optional consistency validation.
    ///
    /// When `changes_dir` is `Some`, plan-to-change consistency checks are
    /// run in addition to structural ones (orphan directories, missing
    /// directories for `in-progress` entries). When `None`, only
    /// structural checks are performed — this is the mode CI uses when
    /// only the plan file is available.
    pub fn validate(&self, changes_dir: Option<&Path>) -> Vec<ValidationResult>;

    pub fn next_eligible(&self) -> Option<&PlanChange>;
    pub fn transition(
        &mut self,
        name: &str,
        target: PlanStatus,
        reason: Option<&str>,
    ) -> Result<(), Error>;
    pub fn create(
        &mut self,
        change: PlanChange,
    ) -> Result<(), Error>;
    pub fn amend(
        &mut self,
        name: &str,
        patch: PlanChangePatch,
    ) -> Result<(), Error>;
    pub fn topological_order(&self) -> Result<Vec<&PlanChange>, Error>;
}
```

### Conventions

- **Location.** One *active* plan per project at `.specify/plan.yaml`. Multiple concurrent plans are a future concern.
- **Lifecycle.** When an initiative completes (`specify plan status` reports no eligible changes, all non-terminal entries resolved), the plan is archived to `.specify/archive/plans/<plan-name>-<YYYYMMDD>.yaml` by a future `specify plan archive` command (see §Future Capabilities). Until that command exists, operators move the file by hand. Starting a new initiative while a previous `plan.yaml` still exists is *not* automatic — the operator is expected to archive or rename first; `specify plan` commands refuse to proceed if the current plan reports "all done" and a create/amend would add to it, pointing the operator at archive instead.
- **Bootstrapping.** Until `specify plan init` exists (see §Future Capabilities), the initial `plan.yaml` is authored by hand. `specify plan validate` is the recommended first command after authoring.
- **Name identity.** The plan entry `name` becomes the Specify change name (the directory under `.specify/changes/`). Names must be unique across the entire plan, including entries with terminal statuses (`done`, `skipped`).
- **Name format.** Same as Specify change names: kebab-case (lowercase letters, digits, hyphens).
- **List order.** YAML list order has **no effect** on the `pending → in-progress` transition whenever a single change is eligible — `depends-on` resolution is the primary ordering signal. It is used only as a deterministic tie-break when two or more changes are simultaneously eligible (in which case `plan next` returns the first in list order). Reordering entries with an unambiguous `depends-on` graph has no observable effect.
- **Adding changes mid-initiative.** Run `specify plan create` / `specify plan amend` (Layer 1), or invoke `/spec:plan` (Layer 2; phases do this automatically). No other code path writes change *entries* to the plan.
- **Initiative completion.** The initiative is complete when no eligible changes remain. `specify plan status` reports whether this means "all done" or "remaining changes are blocked/failed."
- **Plan-to-change linkage and transient states.** `specify plan validate` checks that `in-progress` entries have corresponding `.specify/changes/<name>/` directories and reports orphaned change directories (present on disk but absent from the plan) as warnings. Three transient states where an `in-progress` entry legitimately has no active change directory are expected:
  1. **Start-of-phase window.** `/spec:execute` transitions `pending → in-progress` *before* invoking `/spec:define`, so there is a brief interval before define creates the change directory where validate would see a mismatch. This is a warning, not an error.
  2. **Pre-define crash window.** `/spec:execute` crashed after transitioning `pending → in-progress` but before `/spec:define` produced any artifacts. There is no archive and no live change directory. See [§Plan Mutation and Crash Safety](#plan-mutation-and-crash-safety).
  3. **Post-phase crash window.** `/spec:execute` crashed between `/spec:drop` (or `/spec:merge`) and the follow-up `specify plan transition`. See [§Plan Mutation and Crash Safety](#plan-mutation-and-crash-safety) for self-heal semantics.
- **Concurrent access — write-level locking.** `specify plan create`, `specify plan amend`, and `specify plan transition` acquire an exclusive advisory file lock (`flock`) on `plan.yaml` for the duration of the read-modify-write. The lock is process-scoped: a human running `specify plan transition` while `/spec:execute --loop` holds the lock will block until the driver's current write completes (and vice versa). Lock contention is never silent — commands that cannot acquire the lock within a short timeout fail with `Error::PlanLocked` and exit non-zero. **Note:** `flock` semantics are unreliable on network filesystems (NFS, SMB); Specify workspaces are expected to live on a local filesystem, and this RFC does not attempt to defend against cross-host concurrency.
- **Concurrent access — driver-level locking.** `/spec:execute` (any mode, including `--dry-run`) acquires an exclusive advisory lock on `.specify/plan.lock` held for its entire lifetime. A second `/spec:execute` invocation refuses to start with `Error::DriverBusy`, naming the PID of the running driver. This prevents two drivers from simultaneously selecting "next eligible" against the same plan and racing on transitions — a hazard that per-command `flock` does not cover. The lockfile is removed cleanly on normal exit; stale locks (held by a dead PID) are detected on startup and reclaimed.

---

## Layer 2: Automated Execution

Layer 2 adds the **`/spec:execute`** driver skill that automates the human-driven loop from Layer 1. It reads `plan.yaml`, selects the next eligible change, runs the phase sequence, and updates the currently-active entry's status — recording questions and failures rather than blocking on them. `/spec:execute` does not create or amend change entries; when a phase needs to add a new entry or edit an existing one, it invokes `/spec:plan` directly.

`/spec:execute` is the first skill that programmatically invokes other skills. All existing skills (extract, define, build, merge, drop) remain unchanged; `/spec:execute` invokes the phase skills with arguments and interprets their outputs. Its contract is with the **phase skills** (`/spec:define`, `/spec:build`, `/spec:merge`) — not with the briefs inside each phase's pipeline or the plugin skills those briefs delegate to. See §"Execution Model Overview" above for the full skill layering.

### Invariants summary

The invariants below restate the rules from §"Why this matters" and §"Phase Boundary" in one table, showing which skill owns each.

| Invariant | Enforced by | Source |
|---|---|---|
| Driver contracts with phases, not briefs | `/spec:execute` only invokes `/spec:define`, `/spec:build`, `/spec:merge` | Rule 1 |
| Phases own verify-repair loops | Phase skills exhaust their repair budget before returning | Rule 1 |
| Exactly one of `success`/`failure`/`deferred` per phase | Phase output contract (phase appends a terminal `type: outcome` entry to `journal.yaml` before returning; see [§Phase Outcome Contract](#phase-outcome-contract)) | Rule 1 |
| Change *entries* written only via `Plan::create` / `Plan::amend` | Phases invoke `/spec:plan`; humans run `specify plan create` / `specify plan amend`; both funnel into the same library functions | Rule 2 |
| Change *status* updates written only via `Plan::transition` | `/spec:execute` (Layer 2) or humans (Layer 1) run `specify plan transition` | Rule 2 |
| Single `in-progress` at a time | `plan next` / `plan validate` | §Status State Machine |
| Single `/spec:execute` driver at a time | `.specify/plan.lock` advisory lock | §Conventions |

### Invocation

```
/spec:execute [--dry-run] [--loop]
```

- No arguments: reads `.specify/plan.yaml`, processes a single change then stops (supervised mode)
- `--loop`: process changes one at a time until `specify plan next` reports no eligible change. A `blocked` or `failed` change is *not* an eligible change, so `--loop` naturally skips over them and continues with any still-eligible siblings; it stops only when no `pending` change has all its `depends-on` entries `done`. At that point, final output reports the remaining counts by status so the operator can see whether the initiative is complete or merely stuck.
- `--dry-run`: show what would run next without executing

The plan path is fixed at `.specify/plan.yaml` (see §Conventions → Location). Multi-plan support is a future capability and would add an optional path argument at that time.

### Core Loop

The following is the normative expansion of the `execute` box on the framework diagram. For a single change, `/spec:execute` performs `get next change`, drives `execute change` through define → build → merge, and updates `status` on the currently-active entry. It does not create or amend change entries — phases invoke `/spec:plan` directly when that is needed.

```text
  1. Read plan.yaml
  2. Select next eligible change (all depends-on are done, status is pending)
  3. If none eligible → stop (report blocked/remaining counts)
  4. Transition plan entry: pending → in-progress
  5. Run the phase sequence: invoke /spec:define, then /spec:build, then /spec:merge.
     Each phase internally runs its brief pipeline from the active schema.yaml,
     honouring per-brief `needs` edges. /spec:execute only pre-resolves arguments
     to the phase skill (with field-presence adjustments from the plan entry);
     it does not invoke individual briefs or plugin skills. Phases may invoke
     /spec:plan mid-run to add or amend other change entries; those writes are
     synchronous and visible to every subsequent `get next change` call.
  6. On success: transition in-progress → done
  7. On failure: invoke /spec:drop, transition in-progress → failed, record status-reason
  8. On deferred question: invoke /spec:drop, transition in-progress → blocked, record status-reason
  9. If --loop: continue from step 1; otherwise stop
```

Step 4 transitions `pending → in-progress` *before* `/spec:define` creates the change directory, so between steps 4 and 5 the plan briefly shows an `in-progress` entry with no matching `.specify/changes/<name>/` directory. This is the **start-of-phase transient window** documented in §Conventions; `specify plan validate` reports it as a warning, not an error. Self-heal for the analogous crash-recovery windows is covered in §Plan Mutation and Crash Safety.

### Non-Interactive Execution

#### Problem

The existing skills use `AskQuestion` for confirmations, disambiguation, and warnings. An automated loop can't stop and wait for human input on every change.

#### Design

Most of the skills' interactive decision points are now routed through
the `specify` CLI and resolved without needing a prompt. In
Option-2-style phase skills, lifecycle bookkeeping (existence checks,
`touched_specs` scanning, overlap reports, status transitions, archive
moves, spec merge preview and conflict detection) is performed by CLI
subcommands with CLI flags — there is nothing for `/spec:execute` to
pre-supply at that layer. What remains are genuinely agent-judgement
questions.

| Skill   | Interactive Point                          | `/spec:execute` strategy                                                                 |
|---------|--------------------------------------------|------------------------------------------------------------------------------------------|
| define  | "What do you want to build?"               | Pre-supplied: change `name` + `description` from plan.                                    |
| define  | "Change already exists — continue or restart?" | CLI flag: `specify change create --if-exists continue` (or `restart`); no prompt fires. |
| define  | Source path confirmation (extract)          | Pre-supplied by `/spec:execute` to define; define forwards it to its extract-invoking brief. |
| define  | Overlapping `touched_specs` warning         | `specify change overlap` returns structured JSON; non-empty results are journaled as informational, never blocking. |
| build   | "Task is unclear" pause                     | Recorded as a question, change deferred.                                                  |
| build   | "Design issue discovered" pause             | Recorded as a question, change deferred.                                                  |
| merge   | Artifact / needs / task warnings            | `specify validate` + `specify task progress` return structured JSON; the driver can thresh these without prompting. |
| merge   | Merge preview confirmation                  | `specify spec preview` reports structured operations; pre-confirmed by the driver. |
| merge   | Baseline conflict detected                  | `specify spec conflict-check` reports structured drift; for `/spec:execute`, any non-empty result defers the change. |
| drop    | "Confirm before dropping"                   | Pre-confirmed: `/spec:execute` only drops on failure/deferral; reason plumbed via `specify change drop --reason ...`. |

Skills don't need non-interactive variants. The `specify` CLI is
non-interactive by construction; `/spec:execute` supplies deterministic
answers via CLI flags and by reading structured JSON from each call.
When it *can't* resolve a decision (a genuine question requiring human
judgement — the two `build` pauses above and an unexpected merge-time
lifecycle state), it defers the change rather than guessing. (Extract
is not a peer phase — see the §Note on extract above.)

#### Question Recording

When a step encounters a situation requiring human input, `/spec:execute` writes a structured entry to a journal file at `.specify/changes/<name>/journal.yaml`:

```yaml
entries:
  - timestamp: 2026-04-16T14:30:00Z
    step: build
    type: question
    summary: "Task 3/7 unclear — payment gateway contract references undefined type PaymentIntent"
    context: |
      Working on task: Implement checkout payment processing
      The spec references PaymentIntent but no type definition exists in
      design.md or upstream specs.
```

The plan entry transitions to `blocked` with `status-reason` populated from the **most recent** `type: question` entry in the journal — if a phase records multiple questions before returning `deferred`, only the last one is summarised into `status-reason`; the full list remains in `journal.yaml` for human review. This reuses the existing `blocked` status and its manual `blocked → pending` transition — a human reviews the journal, resolves the question (perhaps by updating the plan description via `specify plan amend`, adding to the spec, or refining the design), and unflags the change.

### Failure and Resumption

#### Problem

A change can fail mid-build (tests don't pass, extraction produces garbage, merge conflicts). What happens to the half-created Specify change?

#### Design

Mark as `failed` with the reason and move on to the next eligible change. The failure signal always arrives at the phase boundary — a brief-level problem (e.g. a failed `cargo test` inside the Omnia `build.md` verify-repair loop) does not surface to `/spec:execute` until the phase skill has exhausted its own repair budget (see §"Phase Boundary" below).

```text
on failure at any phase:
  1. Record failure reason in journal.yaml:
     - timestamp, phase, type: failure, summary, context (stderr, test output, etc.)
  2. Drop the Specify change via /spec:drop (archives partial artifacts)
  3. Transition plan entry: in-progress → failed
  4. Set status-reason on the plan entry from the summary
  5. Continue to next eligible change
```

**Retry**: A human reviews the failure, optionally updates the plan entry's description or dependencies (via `specify plan amend`), then transitions `failed → pending`. On the next `/spec:execute` run, a fresh Specify change is created for that entry.

**Prior-attempt context on retry.** When `/spec:execute` starts a change whose name already has archived attempts, it reads each prior attempt's journal at `.specify/changes/archive/<name>-<timestamp>/journal.yaml`, collects the terminal entry from each (the `type: outcome` entry plus any trailing `type: question` / `type: failure` entries), and passes that list — ordered newest-first, capped at the five most recent attempts — to `/spec:define` as an additional input. The define phase uses it as a "things to avoid" hint — e.g. "the previous attempt failed because the proposed schema was incompatible with baseline `X`". This is a best-effort context channel, not a formal protocol: full forensic detail remains in the archive for humans. The concrete shape of the argument is part of Layer 2's skill invocation model (see [§Skill Invocation Model](#skill-invocation-model)).

#### Failure vs Deferral

Failure means the step ran and produced an error. Deferral means the step couldn't proceed without human input. Both result in the Specify change being dropped and archived, but the distinction matters for triage:

| | Plan status | Reason field | Cause | Resolution |
|---|---|---|---|---|
| Failure | `failed` | `status-reason` | Step error (tests, merge conflict, bad extraction) | Fix the issue, retry (`failed → pending`) |
| Deferral | `blocked` | `status-reason` | Needs human decision (ambiguous requirement, design question) | Answer the question, unflag (`blocked → pending`) |

### Phase Boundary

`/spec:execute` only communicates with phase skills (`/spec:define`, `/spec:build`, `/spec:merge`); it does not invoke individual briefs or plugin skills. Two rules pin down that contract precisely.

#### Rule 1 — Phases own their verify-repair loops

A phase skill is responsible for all recovery that lives inside its brief pipeline. When a brief encounters a brief-level failure (compilation error, failed test, lint violation, reviewer finding, etc.), the phase skill runs its documented repair strategy (for example, the 3-iteration verify-repair loop defined in the Omnia `build.md` brief, which re-enters `crate-writer` or `test-writer` based on the failure classification). `/spec:execute` is not involved.

A phase returns one of exactly three outcomes to `/spec:execute`:

| Outcome | Meaning | `/spec:execute` reaction |
|---|---|---|
| `success` | Phase completed; all briefs produced their `generates` artifacts and any verify-repair loops converged. | Proceed to the next phase (or, after merge, transition plan entry to `done`). |
| `failure` | Phase could not complete after exhausting its internal repair budget. The phase provides a structured summary (which brief failed, final stderr/test output, what was attempted). | Record in `journal.yaml` (`type: failure`), drop the Specify change, transition plan entry to `failed` with the summary. |
| `deferred` | Phase needs human judgement (ambiguous requirement, design question, baseline merge conflict). The phase provides a structured question. | Record in `journal.yaml` (`type: question`), drop the Specify change, transition plan entry to `blocked`. |

This keeps `/spec:execute` free of brief-specific knowledge and avoids double-booked repair logic.

#### Rule 2 — Phases invoke `/spec:plan`; `/spec:execute` transitions `status`

Phases invoke `/spec:plan` when they need to add or modify change entries; it is the only skill that writes change entries to `plan.yaml`. State updates on the currently-active entry (e.g. `in-progress → done`) are performed by `/spec:execute` via `specify plan transition`. No other code path writes `plan.yaml`.

A phase may discover a new neighbouring change (extraction finds `registration-duplicate-email-crash`), notice that an existing entry needs an added dependency, or flag a neighbouring change as touched. The phase calls `/spec:plan create` or `/spec:plan amend` directly during its run, and `/spec:plan` writes `plan.yaml` synchronously — there is no payload, no propose/apply split, and no buffered-mutation list. The new or updated entry is visible to every subsequent `get next change` call, including the next iteration of the same `--loop` run.

Because `/spec:plan` writes during the phase run, any mutations a phase made before it deferred or failed are already in the Plan; mutations it had not yet made are simply not made. There is no "apply on `deferred`" edge case and no mid-apply crash window.

**`/spec:plan amend` may target the currently-active entry.** A phase is allowed to amend non-status fields on its own `in-progress` entry (e.g. to add a newly discovered `depends-on` edge or update `description` with refined scope). Only the `status` field is off-limits to `/spec:plan` — transitions remain `/spec:execute`'s sole prerogative. `PlanChangePatch` in the library reflects this: it has no `status` field.

Consequences:

- Exactly two skills write `plan.yaml`: `/spec:plan` (change entries, in Layer 2) and `/spec:execute` (status transitions on the currently-active entry via `specify plan transition`). Both route through library entrypoints (`Plan::create`/`amend` and `Plan::transition`) that are also the CLI's backing code, so Layer 1 (CLI-only) writes are subject to the same invariants. `specify plan validate` has exactly those two classes of writes to reason about.
- Phase skills need no plan-mutation logic of their own — they invoke `/spec:plan` with the same contract a human would use. This makes phase skills easier to test (no plan fixture required) and leaves legacy/human-driven use of the same skills unaffected.
- The `registration-duplicate-email-crash` example below is the canonical end-to-end worked example of this flow.

#### Phase Outcome Contract

Every phase skill (`/spec:define`, `/spec:build`, `/spec:merge`) returns exactly one of `success`, `failure`, or `deferred` to `/spec:execute`. The transport is a **terminal `type: outcome` entry** appended to `.specify/changes/<name>/journal.yaml` as the phase's last action before returning control:

```yaml
entries:
  # ... zero or more prior type: question / type: failure / type: recovery entries ...
  - timestamp: 2026-04-18T09:14:22Z
    phase: build           # define | build | merge
    type: outcome
    outcome: success       # success | failure | deferred
    summary: "5/5 tasks complete; all verify-repair loops converged"
    context: |
      (optional; present on failure/deferred — stderr, failing test name,
      ambiguous-requirement text, etc. Rendered verbatim into the plan's
      status-reason when /spec:execute records the transition.)
```

`/spec:execute` reads the terminal entry by convention (last entry in `entries`), classifies the outcome, and reacts per the table in Rule 1. If the terminal entry is missing or its `type` is not `outcome`, `/spec:execute` treats the phase as `deferred` with a diagnostic summary — this matches the unclassifiable-crash-window behaviour at the end of [§Plan Mutation and Crash Safety](#plan-mutation-and-crash-safety) and keeps the driver self-consistent.

This transport is normative for Layer 2. Per-invocation return values (structured tool responses, etc.) remain an open design question for how one skill *invokes* another — that is the subject of [§Skill Invocation Model](#skill-invocation-model) — but the *outcome* a phase communicates is always mirrored on disk in `journal.yaml` so `/spec:execute` can read it deterministically and so humans auditing a run can see it without replaying the session.

### Context Threading

#### Problem

How does `/spec:execute` pass context between define → build → merge for a single change?

#### Design

The artifacts are the context. There is no separate context object or state bag. Each phase reads what the previous phase wrote:

```text
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│  plan.yaml             ← /spec:execute reads: name, description, │
│                          sources, affects, depends-on            │
│                                                                  │
│  /spec:define  (runs its brief pipeline; when the plan entry     │
│                 has `sources`, one of define's briefs invokes    │
│                 the /spec:extract plugin skill)                  │
│    reads: source repos (from plan sources map, via extract)      │
│           or creates from description                            │
│    writes: proposal, specs, design, tasks per schema pipeline    │
│                                                                  │
│  /spec:build                                                     │
│    reads: all define artifacts + baseline specs                  │
│    writes: code, marks tasks complete                            │
│                                                                  │
│  /spec:merge                                                     │
│    reads: completed change artifacts + baseline                  │
│    writes: merged baseline specs, archives change                │
│                                                                  │
│  plan.yaml             ← /spec:execute writes: status → done     │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

`/spec:execute`'s responsibility is limited to:

1. **Supplying initial arguments** to each phase skill invocation — derived from the plan entry (name, source paths, description)
2. **Checking preconditions** between phases — reading `.metadata.yaml` to confirm the previous phase completed (status progressed to the expected value)
3. **Deciding what to run next** — field presence (`sources`, `affects`) determines what context to pass to define; the Specify `LifecycleStatus` determines where to resume if re-entering a partially-completed change

#### Resumption Within a Change

The existing `LifecycleStatus` values (defined in the `specify-change` crate — see [RFC-1](rfc-1-cli.md) §`lifecycle.rs`) already encode which step ran last. `/spec:execute` uses this for resumption:

```text
match change.lifecycle_status:
  None           → start from the beginning (define, which invokes extract via one of its briefs if sources present)
  defining       → resume/restart define
  defined        → start build
  building       → resume build
  complete       → run merge
```

If `/spec:execute` crashes mid-change and is restarted, it picks up where it left off by reading the plan (which change is `in-progress`) and the Specify change's `.metadata.yaml` (which step was last completed).

### Step Sequence by Field Presence

The loop is always define → build → merge. `/spec:execute` adjusts what it passes to define based on which fields are present on the plan entry:

```text
change with sources:    define (extract is invoked via one of define's briefs) → build → merge
change with affects:    define (delta against affected specs) → build → merge
change (greenfield):    define → build → merge
```

These are not mutually exclusive — a change could have both `sources` and `affects`. `/spec:execute` passes both signals to define:

- **`sources`** present: `/spec:execute` resolves source paths from the top-level `sources` map and passes them to define, which forwards them to whichever of its briefs invokes `/spec:extract`.
- **`affects`** present: `/spec:execute` passes the list of affected capability names to define for delta targeting against the corresponding baseline specs.

### Plan Mutation and Crash Safety

`plan.yaml` has two library-level writers, each with a narrow scope:

1. **`Plan::create` / `Plan::amend`** write change *entries*. Surfaced as `specify plan create` / `specify plan amend` on the CLI (Layer 1: humans) and as the `/spec:plan` skill that phases invoke during a phase run (Layer 2). Both funnel into the same library functions.
2. **`Plan::transition`** writes `status` updates on the currently-active entry. Surfaced as `specify plan transition` and used by humans (Layer 1) and `/spec:execute` (Layer 2). In Layer 2, transitions happen at well-defined points:
    1. `pending → in-progress`: **before** the first phase invocation for that change
    2. `in-progress → done`: **after** `/spec:merge` completes successfully
    3. `in-progress → failed`: **after** `/spec:drop` completes
    4. `in-progress → blocked`: **after** `/spec:drop` completes and question is journaled

If `/spec:execute` crashes while a change is `in-progress`, the plan may show an active entry with no matching live change directory — or with one that is only partially populated. On restart, `/spec:execute` inspects (a) whether an active `.specify/changes/<name>/` exists, (b) whether an archive entry exists under `.specify/changes/archive/<name>-<timestamp>/`, and (c) the terminal entry of whichever `journal.yaml` is available. Five windows are possible, each with a deterministic resolution:

| Crash window | On-disk state | Terminal journal entry | Self-heal action |
|---|---|---|---|
| After `pending → in-progress`, before `/spec:define` produces any artifacts | No change dir, no archive | — | Re-invoke `/spec:define` for the entry (treat as `LifecycleStatus = None`; same as a cold start from step 5). Append a `type: recovery` entry to a freshly created `journal.yaml` recording the self-heal. |
| Mid-phase crash while change dir exists | Live change dir; no archive | (none, or non-terminal) | Resume per §[Context Threading → Resumption Within a Change](#resumption-within-a-change) using `LifecycleStatus`. No plan transition is necessary. |
| After `/spec:merge`, before `transition → done` | Archived | `type: outcome, outcome: success` | Transition plan entry to `done`. Append `type: recovery` to the archived journal. |
| After `/spec:drop` following a failure, before `transition → failed` | Archived | `type: outcome, outcome: failure` | Transition plan entry to `failed`; copy the journal `summary` into `status-reason`. |
| After `/spec:drop` following a deferral, before `transition → blocked` | Archived | `type: outcome, outcome: deferred` | Transition plan entry to `blocked`; copy the journal `summary` into `status-reason`. |

Self-heal runs before `get next change` on every `/spec:execute` invocation, so the transient `in-progress`-without-active-change state (documented in §Conventions) is always cleaned up at the start of the next run. Every self-heal action appends a `type: recovery` entry to the affected `journal.yaml` so the recovery path is auditable after the fact.

If a `/spec:execute` process cannot unambiguously classify the crash window (for example, the archived change's final `type: outcome` entry is missing, malformed, or contradicts the on-disk state), it emits a diagnostic `type: recovery, outcome: unclassified` entry and stops with a non-zero exit code so a human can triage. The plan entry is left as `in-progress` — no speculative transition is made.

Because `/spec:plan` writes synchronously during a phase run (and the underlying `Plan::create` / `Plan::amend` writes atomically), no mutations are ever "in flight": on crash, any entry the phase already wrote is already in the plan, and any entry it had not yet written is simply absent. There is no mid-apply window to recover.

### Skill Invocation Model

> **Open Design (Layer 2).** The outcome-on-disk protocol is pinned down by the [§Phase Outcome Contract](#phase-outcome-contract); what remains open is how one skill *invokes* another and how per-invocation arguments are passed.

`/spec:execute` runs within the same agent session and invokes other skills by their standard mechanism (e.g., `/spec:define change-name`). By default it processes a single change and stops, keeping the human in the loop. With `--loop`, it holds the agent for the duration of the initiative (or until all eligible changes are processed).

Under the skill layering established in §"Execution Model Overview", the caller/callee pairs that need a concrete invocation contract are:

- `/spec:execute → /spec:define | /spec:build | /spec:merge` (driver invokes phase skills)
- `phase skill → /spec:plan` (phase invokes the plan-writer skill mid-phase to add/amend entries)
- `phase skill → plugin skill` (phase invokes its brief-selected plugin skills)
- `/spec:execute → /spec:drop` (driver invokes drop on failure/deferral)

Resolved for Layer 2 (already specified elsewhere in this RFC):

- **Phase → `/spec:execute` outcome transport** is a terminal `type: outcome` entry in `journal.yaml` (see [§Phase Outcome Contract](#phase-outcome-contract)). `/spec:execute` reads this deterministically on phase return; humans auditing a run see the same data.
- **`/spec:plan` writes** are synchronous to the library (`Plan::create`/`amend`) and visible to every subsequent reader immediately after return.

Open questions applying to all four pairs:

- How are arguments passed across the `/spec:<x>` boundary (slash-command text, tool-call parameters, on-disk argument files)?
- How does the caller read the callee's per-invocation return value (distinct from the on-disk outcome, which is resolved)?
- What's the tool-call shape that makes this deterministic and resumable?

These are design work for the Layer 2 build, not Layer 1 scope. The answer must preserve the outcome-on-disk protocol so self-heal remains correct even when a crash interrupts the invocation-level transport.

### Output and Observability

`/spec:execute` produces structured output at each transition. The step counter reflects the three-phase loop (define → build → merge); extract work appears as a sub-step inside define and is not counted separately.

```text
## /spec:execute — platform-v2

### Initiative: platform-v2
Progress: done 1, in-progress 1, pending 6, blocked 0, failed 1, skipped 0 (total 9)

---

### Processing: email-verification (sources: [monolith])

Step 1/3: define
  - extract sub-step (via /spec:extract)
      Source: /path/to/legacy-codebase
      Artifacts: specs/email-verification/spec.md, design.md ✓
  Artifacts: proposal.md, specs, design.md, tasks.md ✓

Step 2/3: build
  Tasks: 5/5 complete ✓

Step 3/3: merge
  Baseline updated: .specify/specs/email-verification/spec.md ✓
  Status: done

---

### Next: registration-duplicate-email-crash (affects: [user-registration])
```

The progress line enumerates *every* status in the state machine so the output shape is stable even when some statuses have zero entries — matching `specify plan status --format json`.

For deferred changes:

```text
### Processing: notification-preferences (greenfield)

Step 1/3: define
  ⚠ Question recorded — change deferred to blocked

  Question: The description says "notification channel and frequency settings"
  but doesn't specify which channels are in scope. The baseline has no
  notification infrastructure to reference.

  Journal: .specify/changes/notification-preferences/journal.yaml
  Action needed: Update the plan description (specify plan amend …) with channel
  scope, then unflag (blocked → pending) to retry.

### Skipping to next eligible change...
```

When `--loop` terminates (whether successfully, after a failure chain, or on unrecoverable deadlock), `/spec:execute` emits a terminal summary matching the stable JSON shape from `specify plan status --format json`:

```text
## /spec:execute — platform-v2 — terminated

### Final state
Progress: done 6, in-progress 0, pending 2, blocked 1, failed 0, skipped 0 (total 9)

Completion: stuck (2 pending changes remain but their dependencies are not all done; 1 change is blocked)

Blocked:
  - notification-preferences (status-reason: "Channel scope not specified in description.")

Pending (dependencies not satisfied):
  - product-catalog (waits on: extract-shared-validation)
  - extract-shared-validation (waits on: email-verification)

Next action: resolve blocked changes (specify plan amend + specify plan transition notification-preferences blocked → pending) or wait for their dependencies to complete.
```

The `Completion:` line is one of `all-done`, `stuck` (pending changes with unmet dependencies), `halted` (a failure or deferral stopped the loop), or `driver-interrupted` (SIGINT/SIGTERM). The same classification is emitted under `completion` in the JSON form.

### Worked Example: Phase-invoked `/spec:plan`

This trace makes the `create/amend` blue box on the framework diagram load-bearing. The `registration-duplicate-email-crash` entry in the plan example above is introduced to the Plan by a phase-invoked `/spec:plan` call during another change's define phase.

1. `/spec:execute` picks up `email-verification`, transitions its entry to `in-progress` via `specify plan transition`.
2. `/spec:execute` invokes `/spec:define email-verification`.
3. During define, the `/spec:extract` plugin skill (invoked by one of define's briefs, since `email-verification` has `sources`) discovers a defect in `user-registration` (duplicate email submission returns 500 instead of 409).
4. The define phase invokes `/spec:plan` directly:
    ```text
    /spec:plan create registration-duplicate-email-crash \
        --affects user-registration \
        --description "Duplicate email submission returns 500 instead of 409. Discovered during email-verification extraction."
    ```
    `/spec:plan` writes the new entry into `plan.yaml` synchronously.
5. Define continues, completes its own briefs, returns `success`.
6. `/spec:execute` invokes `/spec:build email-verification`, then `/spec:merge email-verification`.
7. On success, `/spec:execute` transitions `email-verification` to `done` via `specify plan transition`.
8. The next `/spec:execute` iteration picks up `registration-duplicate-email-crash` (or a higher-priority sibling, depending on dependencies).

There is no buffered mutation, no `plan_mutations` payload, no deferred-apply edge case: the new entry was written by the `/spec:plan` skill during the define phase, and it is visible to every subsequent `get next change` call.

### Layer 2 Concerns Summary

| Concern | Resolution |
|---|---|
| Interactive skills | `/spec:execute` pre-resolves arguments; genuine questions defer the change |
| Failure | `/spec:drop` the Specify change, mark `failed` with `status-reason`, advance |
| Resumption | Plan `in-progress` + Specify `LifecycleStatus` encode exactly where to resume |
| Context threading | Artifacts written by each phase are read by the next; plan supplies initial args |
| Crash safety | `/spec:execute` classifies the on-disk state on restart and self-heals to `done`/`failed`/`blocked` (five windows documented, including pre-define, mid-phase, and unclassifiable) |
| Retry context | Prior-attempt terminal journal entries (capped, newest-first) are passed to `/spec:define` on `failed → pending` retry |
| Observability | Structured per-phase output + terminal summary on loop exit + `journal.yaml` for questions/failures/recoveries |
| Brief-level errors | Phase skills own their verify-repair loops; only phase-level outcomes cross the boundary |
| Phase outcome transport | Terminal `type: outcome` entry in `journal.yaml` (see [§Phase Outcome Contract](#phase-outcome-contract)) |
| Plan entry writes | `/spec:plan` is invoked directly by phases (Layer 2); `/spec:execute` only writes `status` transitions. Layer 1 uses `specify plan create`/`amend`/`transition` CLI for the same library calls. |
| Driver concurrency | `.specify/plan.lock` PID-level advisory lock prevents two `/spec:execute` processes running simultaneously |

Layer 2 adds one new file (`journal.yaml` per change), one new lockfile (`.specify/plan.lock`), and no new plan statuses — it works entirely within the existing status state machine and Specify lifecycle.

---

## Future Capabilities

These are supported by the plan format but not part of the initial implementation:

### Migration Mode (beyond Layer 2)

Source-aware define (the basic `sources` → `/spec:extract` wiring) is part of Layer 2 above and not repeated here. Migration Mode in this section refers to capabilities that build *on top of* Layer 2 but are not yet scoped for it:

**Fixture-backed verification.** For changes where the `wiretapper` has captured runtime request/response fixtures from the legacy system, the `replay-writer` generates tests from the captured fixtures and the build phase verifies the new implementation against them. This creates a behavioural regression safety net.

**Slice strategy.** Good early migration candidates are leaf services with few dependents, clear API boundaries, existing test coverage, and low cross-boundary coupling. The `depends-on` field encodes these ordering decisions. Documenting a recommended slice-selection heuristic (automated or advisory) is deferred.

### Multi-Repo Initiatives

The plan format supports multi-repo initiatives on both the source and target sides. What is *in scope* for Layer 2 is cloning and extracting from git-remote sources — a change's `sources` list may name git URLs, which `/spec:extract`'s `git-cloner` plugin resolves during the define phase. What is deferred here is *cross-repo spec references* (e.g. `@peer:capability`), which belong to RFC-3's federation model.

- **Multi-source extraction.** A change's `sources` list declares which repos to extract from; a change may reference multiple sources. In scope for Layer 2.
- **Multi-target implementation.** Features spanning multiple build targets are decomposed into separate changes with `depends-on` edges. In scope today.
- **Cross-repo spec resolution.** Cross-repo spec references (distinct from cloning source repos) are resolved through the federation model defined in [RFC-3](rfc-3-multi-repo.md). Deferred.

### Other Deferred Capabilities


| Capability | Rationale for deferral |
| ---------- | ---------------------- |
| `specify plan init` | Humans write better initial plans than automated structural discovery. Add when the volume of initiatives justifies the tooling. |
| `specify plan archive` | Automates the end-of-initiative move to `.specify/archive/plans/<name>-<date>.yaml`. Trivial on its own; deferred only so Layer 1 ships without the archive policy being opinionated. Until then, the move is manual. |
| `specify plan doctor` | Extended cross-check surface beyond `validate`: `affects` ↔ `.metadata.yaml:touched_specs` agreement, prior-attempt archive presence, orphan journal files, `affects` status coherence beyond the basic warning. Deferred because the checks depend on Layer 2 behaviours. |
| Multiple concurrent plans | Requires a path argument on every `specify plan` subcommand plus a way to pick a default. Deferred until a use case appears; today, archive-then-create is the recommended pattern. |
| Change recommender | LLM-assisted refinement of auto-generated plans. Depends on `plan init`. |
| Behavioural diff | Undesigned. The existing `replay-writer` already provides fixture-backed verification for migration use cases. |
| Cross-stack define | A mode of `/spec:define`, not a plan concern. Can be added to define independently. |


## Existing Infrastructure


| Capability                     | Status | Notes                                        |
| ------------------------------ | ------ | -------------------------------------------- |
| Source code analysis for define | Exists | `/spec:extract` (invoked inside define by a brief when change has `sources`) |
| Capture runtime fixtures       | Exists | `wiretapper`                                 |
| Generate replay tests          | Exists | `replay-writer`                              |
| Define → Build → Merge chain   | Exists | `/spec:define`, `/spec:build`, `/spec:merge` — agent-side orchestrators. All deterministic work (status transitions, `.metadata.yaml` writes, schema + pipeline resolution, spec merge preview + coherence validation, baseline drift detection, archive move) is delegated to `specify change {create, transition, touched-specs, overlap, archive, drop}`, `specify schema {resolve, pipeline}`, `specify spec {preview, conflict-check}`, `specify validate`, `specify task {progress, mark}`, and `specify merge`. |
| Drop partial change            | Exists | `/spec:drop` → `specify change drop <name> --reason` (Layer 1: invoked by humans on failure/deferral; Layer 2: invoked by `/spec:execute`) |


## New Capabilities Required

### Layer 1 (MVP)


| Capability                       | Type  | Notes                                                                          |
| -------------------------------- | ----- | ------------------------------------------------------------------------------ |
| Plan format (`plan.yaml`)        | Schema| Ordered change list with dependencies and per-change status                    |
| Plan JSON Schemas                | Schema| `plan.schema.json` (authoring) + `plan-validate-output.schema.json` (tooling)  |
| `plan.rs` in `specify-change`    | Lib   | Parsing, validation, state machine, dependency graph, consistency checks, advisory file locking |
| `specify plan validate`          | CLI   | Cycle detection, referential integrity, duplicate names, consistency check; stable JSON output |
| `specify plan next`              | CLI   | Return the next pending change (respecting `depends-on`, single in-progress)   |
| `specify plan status`            | CLI   | Initiative progress in dependency order: counts, blockers, next eligible; cycle-safe fallback to list order |
| `specify plan create`            | CLI   | Add a new change entry (state machine enforced; plan validated before write)   |
| `specify plan amend`             | CLI   | Edit non-status fields on an existing entry                                    |
| `specify plan transition`        | CLI   | Validated status transitions, with reason routing to the matching reason field |


### Layer 2 (Automated Execution)


| Capability                       | Type  | Notes                                                                          |
| -------------------------------- | ----- | ------------------------------------------------------------------------------ |
| `/spec:execute`                  | Skill | Driver skill: automated define → build → merge loop. See §"Layer 2: Automated Execution" above |
| `/spec:plan`                     | Skill | Plan-mutation skill: invoked by phases during a phase run to add/amend entries; writes via the same `Plan::create`/`amend` entrypoints as the Layer 1 CLI |
| Skill invocation model           | Design| How one skill programmatically invokes another and passes arguments (per-invocation return values remain open; outcome-on-disk is resolved by the Phase Outcome Contract) |
| Phase outcome contract           | Design| Phases return exactly one of `success`/`failure`/`deferred`; mirrored as a terminal `type: outcome` entry in `journal.yaml`; brief-level errors stay inside the phase |
| `sources` execution wiring       | Skill | `/spec:execute` resolves source paths and passes them through define to extract |
| `affects` execution wiring       | Skill | `/spec:execute` passes affected capability names to define for delta targeting  |
| `journal.yaml`                   | Schema| Structured question/failure/outcome/recovery recording per change for autonomous operation |
| `.specify/plan.lock`             | Schema| PID-level advisory lockfile preventing concurrent `/spec:execute` drivers      |


## References

- [RFC-1: `specify` CLI](rfc-1-cli.md) — prerequisite; `specify plan` subcommands extend the CLI
- [RFC-3: Multi-Repo Coordination](rfc-3-multi-repo.md) — provides federation resolution for plans that span repositories
