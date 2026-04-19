---
name: plan
description: |
  Author the initial .specify/plan.yaml for an initiative via the
  pipeline.plan brief pipeline. Layer 3 counterpart to /spec:execute:
  /spec:plan writes the plan, /spec:execute runs it.
license: MIT
argument-hint: "<initiative-name> [--from <path>...] [--against <path>] [--source <key>=<path-or-url>...] [--focus <area>] [--extend] [--dry-run]"
---

# Plan skill

Author `.specify/plan.yaml` for a new initiative by running the
`pipeline.plan` brief pipeline declared in the active schema's
`schema.yaml`. `/spec:plan` is the Layer 3 authoring counterpart to
`/spec:execute`: one *writes* the plan, the other *runs* it.

> **Scope note.** This revision (RFC-2 L3.E) ships the skill scaffold:
> invocation surface, the five-step core loop shape, the single-writer
> invariant, working-directory layout, and a `--dry-run` readiness
> report. **Discovery brief wiring lands in L3.F; propose brief
> wiring lands in L3.G.** Today `/spec:plan` runs steps 1, 2, and 4
> of the core loop end-to-end; steps 3(a) and 3(b) are pipeline-
> shaped placeholders that invoke the briefs declared in
> `schema.yaml` but do not yet contain their full bodies.

## Overview

Specify at authoring time is a three-layer stack — mirror of the
execution stack documented in [`../execute/SKILL.md`](../execute/SKILL.md):

1. **Plan CLI** (`specify plan {init, validate, next, status, create,
   amend, transition, archive, lock}`) — the library-backed verbs that
   read and write `.specify/plan.yaml`. The single writer of the plan
   file, used by humans (Layer 1), `/spec:execute` (Layer 2), and this
   skill (Layer 3) alike.
2. **Authoring skill** (`/spec:plan`, this one) — the Layer 3 driver
   that runs the `pipeline.plan` brief pipeline and shells out to
   `specify plan create` for each accepted slice.
3. **Driver skill** (`/spec:execute`) — the Layer 2 automation that
   consumes the plan this skill authored.

The on-disk contracts the authoring skill depends on are:

| File / directory | Owner | Role |
|---|---|---|
| `.specify/plan.yaml` | library (`Plan::{init, create, amend, transition, archive}`) | Ordered change list with per-entry status. `/spec:plan` writes only via `specify plan init` (step 2) and `specify plan create` (step 3b). |
| `.specify/plans/<name>/` | schema (`pipeline.plan` briefs) | Working directory for authoring artefacts — `discovery.md` from the discovery brief, `proposal.md` from the propose brief. Swept by `specify plan archive` alongside the plan itself (RFC-2 L3.B). |
| `schema.yaml:pipeline.plan` | schema (`Phase::Plan`) | Declares the ordered list of authoring briefs for the project's schema. Resolved via `specify schema pipeline --phase plan`. |

See [RFC-2 §"Layer 3: Plan Authoring"](../../../../rfcs/rfc-2-execution.md)
for the full design, including the [Core loop](../../../../rfcs/rfc-2-execution.md),
[Plan pipeline briefs](../../../../rfcs/rfc-2-execution.md),
[Working directory](../../../../rfcs/rfc-2-execution.md), and
[Integration with `/spec:execute`](../../../../rfcs/rfc-2-execution.md)
sections.

## Invocation

```text
/spec:plan <initiative-name> \
    [--from <path>...] \
    [--against <path>] \
    [--source <key>=<path-or-url>...] \
    [--focus <area>] \
    [--extend] \
    [--dry-run]
```

Flags:

- **`<initiative-name>`** — kebab-case identifier; becomes the plan's
  top-level `name` field. Validated with the same rules as change
  names (regex `^[a-z][a-z0-9-]*$`) before any other work. An invalid
  name is a hard exit with a clear diagnostic — the skill never
  rewrites or "helps" the name.
- **`--from <path>`** — artefact file(s) or directory describing the
  target shape for greenfield authoring. Repeatable. Consumed by the
  discovery brief (L3.F).
- **`--against <path>`** — an existing codebase to delta against, used
  for refactor or modernisation initiatives. Consumed by the discovery
  brief (L3.F).
- **`--source <key>=<path-or-url>`** — a named source for migration.
  Repeatable. The `key` is a kebab-case identifier recorded in the
  plan's top-level `sources` map and referenced by individual plan
  entries via their `sources` list; the `value` is either a local
  filesystem path or a git URL. The skill forwards the tuple verbatim;
  cloning (if any) is the discovery brief's concern via `/spec:extract`
  and `git-cloner`.
- **`--focus <area>`** — optional scoping hint for the propose brief
  (L3.G). Free-form string; the propose brief decides how to interpret
  it.
- **`--extend`** — add to an existing `.specify/plan.yaml` instead of
  refusing. Skips step 2 (scaffold) and reopens the propose loop
  against the existing plan; existing entries are never modified —
  `--extend` is additive-only.
- **`--dry-run`** — emit the readiness report and the proposed plan to
  stdout; write nothing. See §"Dry-run output".

At least one of `--from`, `--against`, or `--source` must be supplied.
A bare `/spec:plan <name>` is a hard exit — the skill cannot decide
the initiative's shape without at least one input.

## Core loop (five steps)

Follow these steps in order on every invocation. Each step is
normative; every shell-out is to the Layer 1 `specify` CLI; this
skill writes nothing to `.specify/plan.yaml` directly.

```text
1. Parse inputs; resolve source paths; assert plan.yaml absent
   (or --extend).

   - Validate <initiative-name> as kebab-case. Reject with a hard
     exit on failure.
   - Require at least one of --from, --against, --source. Reject
     with a hard exit on failure.
   - If .specify/plan.yaml exists and --extend was NOT supplied,
     refuse with a diagnostic pointing at `specify plan archive`.
     (There is no --force. The refusal is deliberate: overwriting
     an existing plan would drop audit history.)
   - If --extend was supplied but .specify/plan.yaml does NOT
     exist, also refuse — there is nothing to extend. Point the
     operator at running without --extend.

2. Scaffold the plan.

     specify plan init <initiative-name> \
         [--source <key>=<path-or-url> ...]

   Writes an empty .specify/plan.yaml with just the initiative
   `name` and the supplied `--source` entries in the top-level
   `sources` map. `changes: []` until step 3(b) populates it.

   Skipped entirely when --extend is set: the caller is explicitly
   adding to an existing plan, and `specify plan init` refuses
   when .specify/plan.yaml already exists (see RFC-2 §"CLI
   support").

3. Run the plan brief pipeline from schema.yaml.

   Resolve the ordered list of briefs via:

     specify schema pipeline --phase plan \
         --change .specify/plans/<name> --format json

   The response lists every brief in the schema's `pipeline.plan`
   declaration in topological order, with each brief's absolute
   `path`, `needs` edges, `generates` target, and current
   `present` flag relative to this initiative's working directory.

   Then run each brief in order:

     a. discovery  — read --from artefacts and/or analyse
                     --against / --source codebases; write the
                     consolidated capability inventory to
                     .specify/plans/<name>/discovery.md.
                     (L3.F fleshes this step out; the current
                     revision exposes the step shape but does not
                     yet wire /spec:extract or git-cloner.)

     b. propose    — read discovery.md; decompose into change
                     slices with `depends-on` edges using the
                     schema's slice heuristics; materialise a
                     draft; iterate with the human (accept /
                     edit / reject per slice); for each accepted
                     slice, shell out to:

                       specify plan create <name> \
                           [--sources <key> ...] \
                           [--depends-on <other-name> ...] \
                           [--affects <other-name> ...] \
                           [--description "..."]

                     The full proposal is captured in
                     .specify/plans/<name>/proposal.md regardless
                     of per-slice decisions.
                     (L3.G fleshes this step out; the current
                     revision exposes the step shape but does not
                     yet implement the accept/edit/reject loop or
                     the per-slice `specify plan create` calls.)

4. Final validation gate.

     specify plan validate

   Runs the Layer 1 validator against the authored plan. Report
   every `ValidationResult` verbatim. Non-zero exit on any result
   with `level == Error`. A clean validate is the contract the
   skill owes its caller — a plan that ships to `/spec:execute`
   without passing `specify plan validate` is a regression.

5. Exit with a hand-off summary.

   Point the human at:
     - `specify plan status` — review the authored plan.
     - `/spec:execute --loop` — start executing it (Layer 2).

   Non-zero exit on any earlier step's hard failure; zero exit on
   a clean validate.
```

## Single-writer invariant (RFC-2 §"Phase Boundary → Rule 2")

Every plan entry this skill writes goes through **`specify plan
create`**. The skill never edits `.specify/plan.yaml` directly, never
rewrites existing entries, and never bundles multiple entries into a
batch write. This preserves the single-writer invariant established
in RFC-2 §"Phase Boundary → Rule 2": exactly two classes of writes
touch `plan.yaml` (entry writes via `Plan::{create, amend}` and
status writes via `Plan::transition`), and both route through the
library.

The invariant extends to `--extend`: additional entries are added via
`specify plan create`; pre-existing entries are left untouched. The
skill has no path that calls `specify plan amend` or `specify plan
transition` — those verbs belong to the running initiative
(humans in Layer 1, `/spec:execute` in Layer 2), not to the authoring
step.

See [RFC-2 §"Phase Boundary → Rule 2"](../../../../rfcs/rfc-2-execution.md)
for the full contract.

## Working directory (`.specify/plans/<name>/`)

Authoring artefacts live under `.specify/plans/<initiative-name>/`,
mirroring the `.specify/changes/<name>/` pattern used by the phase
skills:

```text
.specify/
├── plan.yaml                       # the authored plan
└── plans/
    └── <initiative-name>/
        ├── discovery.md            # from the discovery brief (step 3a)
        └── proposal.md             # from the propose brief (step 3b)
```

The working directory is created lazily — by the discovery brief
itself when it writes `discovery.md`, not by the skill scaffold.
Step 2 (`specify plan init`) does not create it.

On archive, `specify plan archive` (RFC-2 L3.B) sweeps this directory
alongside `plan.yaml` into `.specify/archive/plans/<name>-<YYYYMMDD>/`,
preserving the authoring trail with the plan it produced.

## Dry-run output

Under `--dry-run`, the skill emits a pre-authoring **readiness
report** and exits without writing anything. The report confirms
that the inputs parsed, the pipeline resolved, and the skill is
ready to run — it does not include a proposed plan (that comes out
of the discovery/propose briefs, which do not run under `--dry-run`
in this revision).

The report shape, pinned by
[`fixtures/dry-run/expected-output.md`](fixtures/dry-run/expected-output.md):

```text
[dry-run] /spec:plan — <initiative-name>

Initiative: <initiative-name>
Sources:
  - <key>: <path-or-url>
  - ...
Pipeline: pipeline.plan (<brief-id>, <brief-id>...)

No files written. Remove --dry-run to run the pipeline.
```

Section rules:

- The `Sources:` block is omitted entirely when `--source` was not
  supplied. `--from` and `--against` inputs do not surface here — the
  report pins the top-level `sources` map shape that step 2 would
  write to `plan.yaml`, not the full set of discovery inputs.
- The `Pipeline:` line names `pipeline.plan` and lists the brief IDs
  in the order `specify schema pipeline --phase plan` returns.
- Every line prefixed with `[dry-run]` on the banner is enough — the
  body lines do not need a per-line prefix.

Discovery and propose have their own dry-run rendering (proposed
entries, preview diff against the existing plan for `--extend`, etc.)
and those render in L3.F / L3.G alongside the brief wiring that
produces them. The L3.E dry-run output is the readiness gate that
precedes all of that.

## Constraints

- **`.specify/plan.yaml` already exists without `--extend`.** Refuse
  with a diagnostic pointing at `specify plan archive`. There is no
  `--force`; a human wanting to start over runs archive first. This
  matches the `specify plan init` CLI contract (RFC-2 §"CLI support").
- **`--extend` with no existing plan.** Refuse with a diagnostic
  pointing at re-running without `--extend`. The skill never
  silently creates a fresh plan under `--extend` — the flag is an
  explicit "I know there's a plan here" signal.
- **`--dry-run` writes nothing.** No `specify plan init`, no
  `specify plan create`, no `discovery.md`, no `proposal.md`. The
  dry-run contract is read-only end to end; an editor watching the
  filesystem for writes during a dry-run should see nothing under
  `.specify/`.
- **Kebab-case `<initiative-name>`.** Validated before any other
  work (including before reading `.specify/plan.yaml`, before
  resolving `--source` paths, and before any CLI shell-out). A bad
  name exits non-zero with a clear diagnostic and no side effects.
- **No driver lock.** `/spec:plan` holds no locks that `/spec:execute`
  observes. The `.specify/plan.lock` PID stamp is the execution
  side's concern — authoring and execution are strictly ordered
  (authoring produces the plan, execution consumes it), so there is
  no shared-state race to guard against with a lock. A human running
  `specify plan transition` or `specify plan amend` by hand while
  `/spec:plan` is authoring is safe because every write goes through
  the atomic library functions.

## What this skill does NOT do

| Surface | Status |
|---|---|
| Execute the plan | Never. Execution is `/spec:execute`'s concern (Layer 2). `/spec:plan` exits with a hand-off summary that points the operator at `/spec:execute --loop`. |
| Modify existing plan entries | Never. `--extend` is append-only; pre-existing entries are left untouched. Editing a pending entry mid-authoring is done via `specify plan amend` by the human, not by this skill. |
| Skip `specify plan validate` | Never. Step 4 is unconditional — every run ends with a validation gate, and a non-clean validate exits non-zero. This is the contract the skill owes its caller. |
| Invoke `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, or `/spec:execute` | Never. `/spec:plan` only invokes the briefs declared in `schema.yaml`'s `pipeline.plan`, plus the `specify plan` CLI for scaffolding, entry creation, and validation. |
| Hold a driver lock | Never. `.specify/plan.lock` is reserved for `/spec:execute`; authoring runs outside that lock. |
| Write `.specify/plan.yaml` directly | Never. Every write goes through `specify plan init` (step 2, skipped under `--extend`) or `specify plan create` (step 3b, one call per accepted slice). |
| Clone git URLs | Never. `--source` values that are git URLs are passed through to the discovery brief verbatim; cloning (if any) happens inside `/spec:extract` via `git-cloner` (RFC-2 L3.F). |
| Fill in discovery brief bodies | Not in this Change. The discovery step shape is pinned here; full wiring (artefact reading, `/spec:extract` invocation, `discovery.md` emission) lands in L3.F. |
| Fill in propose brief bodies | Not in this Change. The propose step shape is pinned here; full wiring (slice decomposition, accept/edit/reject loop, per-slice `specify plan create` calls) lands in L3.G. |

The state the skill mutates in this Change:

1. `.specify/plan.yaml` via `specify plan init` (step 2; skipped
   under `--extend`) and `specify plan create` (step 3b; once per
   accepted slice, wired fully in L3.G).
2. `.specify/plans/<initiative-name>/discovery.md` written by the
   discovery brief (step 3a; wired fully in L3.F).
3. `.specify/plans/<initiative-name>/proposal.md` written by the
   propose brief (step 3b; wired fully in L3.G).

No other on-disk state is written by `/spec:plan` itself.

## Guardrails

- Never hand-edit `.specify/plan.yaml`. Route every write through
  `specify plan init` (step 2) or `specify plan create` (step 3b).
  The single-writer invariant in RFC-2 §"Plan Mutation and Crash
  Safety" depends on it.
- Never skip `specify plan validate` (step 4). A plan that ships to
  `/spec:execute` without a clean validate is a regression; the
  validator is the contract the skill owes the downstream driver.
- Validate `<initiative-name>` before any filesystem read or CLI
  shell-out. A bad name should never leave a half-written plan
  behind.
- For `--dry-run` specifically: the skill MUST NOT shell out to
  `specify plan init`, `specify plan create`, `specify plan amend`,
  or `specify plan transition`; MUST NOT invoke the discovery or
  propose briefs (they would write to `.specify/plans/<name>/`);
  MUST NOT create `.specify/plans/<name>/` at all. The first-line
  banner prefixes the rendered output with `[dry-run] ` (the body
  lines do not need a per-line prefix — the banner is enough).
- For `--extend` specifically: step 2 is skipped in full; step 3(b)
  only appends entries via `specify plan create` — it never calls
  `specify plan amend` or `specify plan transition` on existing
  entries. A propose-time decision to modify an existing entry is
  surfaced to the human, who runs `specify plan amend` by hand
  outside the authoring loop.
- Treat an unexpected `specify schema pipeline --phase plan`
  response shape (missing keys, unknown brief IDs, empty pipeline)
  as a hard failure: print the raw JSON and exit non-zero. Do not
  speculate about brief ordering.
