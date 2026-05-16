# Phase outcome contract

The four phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`) return control to the `/change:execute` driver loop through a single three-channel contract:

1. **`PhaseOutcome`** stamped into `.specify/slices/<name>/.metadata.yaml` — the only state `/change:execute` reads on phase return.
2. **`journal.yaml`** entries appended during the run — append-only audit log; never a signalling channel.
3. **`plan.yaml`** mutations issued through `specify plan add` / `specify plan amend` — bounded by the allow/forbid table below.

This document is the parameterised contract, including the concrete success criteria, failure modes, and deferral triggers for each phase. Individual skills link here and keep only phase-local operational steps in their always-loaded bodies.

---

## Per-phase deltas

### Define

- **`success`** — every define brief produced its `generates` artefact and there are no `[unknown]` blockers; the slice is ready for `/spec:build`.
- **`failure`** — a brief halted before all artefacts were written, such as extraction fixture capture crashing or a writer brief exhausting its repair budget.
- **`deferred`** — upstream input is missing or ambiguous, such as a source/baseline conflict, unclear scope, or a requirement that needs human judgement.

### Build

- **`success`** — every build brief converged, validation is green, and `specify slice task progress` reports `pending == 0`; the slice is ready for `/spec:merge`.
- **`failure`** — implementation or verification halted after the repair budget, such as a non-converging test/build loop or a specialist writer returning a non-recoverable error.
- **`deferred`** — the build is blocked on a question, such as an ambiguous task, design issue surfaced during implementation, or unsafe artefact update.

### Merge

- **`success`** — baseline merge applied, lifecycle transitioned to `merged`, and the slice archive moved. This path is uniquely CLI-stamped by `specify slice merge run`; skills MUST NOT call `outcome set` after a successful merge.
- **`failure`** — `specify slice merge run` exited non-zero and left the filesystem unchanged; record skill-side via `outcome set ... merge failure ...`.
- **`deferred`** — `specify slice merge run` was never invoked, such as when the user declined the preview, conflict-check needs human arbitration, or lifecycle is not `Complete`; record skill-side via `outcome set ... merge deferred ...`.

### Drop

- **`success`** — `specify slice drop` exited zero, archiving the slice with status `dropped` and the supplied reason recorded in `.metadata.yaml`. The lifecycle stamp is the success signal; no separate `outcome set` call is made.
- **`failure`** — `specify slice drop` returned a lifecycle violation or malformed-directory error; record skill-side via `outcome set ... drop failure ...`.
- **`deferred`** — rare; an interactive cancel or precondition needs human resolution before the drop is safe. Non-interactive runs from `/change:execute` do not reach this path.

---

## Outcome values

Every phase MUST record exactly one of these three outcomes before returning control. The outcome-recording call is the **last action** the skill takes.

- **`success`** — the phase converged: every brief produced its `generates` artefacts, every verify-repair loop closed, and the phase's terminal CLI invocation exited zero. The driver translates this into a plan transition to `done`.
- **`failure`** — the phase halted after the repair budget was exhausted (a brief or specialist skill returned a non-recoverable error). Use `--summary` to name **which** brief and the load-bearing stderr / failing-test line; use `--context` for verbatim detail (compiler output, failing assertion, coherence-check tail, etc.). The driver translates this into `failed`.
- **`deferred`** — human judgement is needed (an ambiguous requirement, missing scope, baseline drift requiring arbitration, an artefact update that is not safe to do unattended). Use `--summary` to name **the question**; use `--context` for verbatim detail (the ambiguous-requirement text, the conflict-check report, etc.). The driver translates this into `blocked`.

---

## Recording the outcome

The standard form (used by `define`, `build`, and the failure / deferred paths of `merge`):

```bash
specify slice outcome set <name> <phase> <outcome> --summary "..." [--context "..."]
```

`/spec:drop` records the outcome implicitly: the CLI stamps the `dropped` lifecycle state and the operator-supplied reason into `.metadata.yaml` when `specify slice drop --reason "..."` exits zero. See `drop/SKILL.md` §Non-interactive mode for the forwarding rules.

### Merge success path is CLI-stamped

`specify slice merge run` is the unique exception: on success it stamps `PhaseOutcome { phase: merge, outcome: success }` into `.metadata.yaml` atomically with the `Merged` lifecycle transition, **before** the archive move. Skills MUST NOT call `outcome set` on this path — the slice directory no longer exists under `.specify/slices/` after archiving, so the call would fail with `not found`. The archived `.metadata.yaml` carries the outcome; `/change:execute` reads it via `specify slice outcome show <name>`, which falls back to the archive when the active directory is absent.

In a materialised RFC-14 workspace slot, the merge CLI owns only the baseline commit boundary: `.specify/specs/` and `.specify/archive/` are committed as `specify: merge <slice-name>`. Non-baseline project residue is intentionally left for `/change:execute`, which must verify the baseline paths are clean and then either commit residue as `specify: residue <slice-name>` or halt before marking the plan entry `done`.

### Driver fallback on missing or malformed outcome

If `.metadata.yaml:outcome` is missing or malformed when `/change:execute` reads it on phase return, the driver treats the phase as `deferred` and stops for triage. **Do not** skip the recording call as a "soft success" — silence is treated as deferral, not as completion.

---

## Verbatim-`summary` rule

When `/change:execute` reclaims a `failure` or `deferred` outcome by invoking `/spec:drop` (self-heal on startup, or per-iteration cleanup), it copies `outcome.summary` **byte-for-byte** into the `reason` argument:

```bash
/spec:drop <change> reason "<outcome.summary verbatim>"
```

Skills MUST therefore write `--summary` strings that are useful as a `--reason`: present-tense, self-contained, and short enough to fit a CLI argument without truncation. Route any verbatim stderr / log tail through `--context` instead — that field is not forwarded to `--reason`.

---

## Journal entries during the run

Whenever a phase encounters a situation a human should see — a genuine question, a repair attempt that failed, or a notable recovery — it appends to `.specify/slices/<name>/journal.yaml` **during** the run, not just at the end:

```bash
specify slice journal append <name> <phase> <kind> --summary "..." [--context "..."]
```

### Kinds

- **`question`** — anything that might produce a `deferred` outcome at the end of the phase: an ambiguous requirement, a missing scope hint, baseline drift surfaced by `change merge conflict-check`, a design issue uncovered mid-build. Write one entry per question so triagers see the full trail.
- **`failure`** — a brief, specialist writer skill, or CLI invocation returned an error after retry. Write one entry per failure; the final `outcome set --summary` rolls up only the load-bearing one, but the journal preserves every attempt inside the verify-repair loop.
- **`recovery`** — a self-heal / recovery step happened. Typically written by `/change:execute` itself; phase skills rarely need to append this kind.

`journal.yaml` is a pure append-only audit log. `/change:execute` never reads it as a signalling channel — the `outcome` field in `.metadata.yaml` is the only state the driver consumes on phase return.

---

## Mutating the plan mid-run

Phases MAY shell out to `specify plan add` / `specify plan amend` mid-run when they discover something structural about the slice. Both commands write `plan.yaml` synchronously — the new or updated entry is visible to every subsequent `/change:execute` iteration.

### Allowed

- **`specify plan add <new-name> --description "...modifies <current-name>..."`** — when a phase surfaces a neighbouring defect or prerequisite refactor that warrants its own change. Examples: a define extract sub-step uncovers the canonical `registration-duplicate-email-crash` case; a build implementation reveals a sibling refactor; a merge conflict-check surfaces a neighbouring slice that must land first.
- **`specify plan amend <current-name> --depends-on <newly-needed>`** — when a phase discovers a dependency on another plan entry. `amend` MAY target the currently-active `in-progress` entry: non-`status` fields on it are fair game.

### Forbidden

- **Writing `status` through `amend`.** The `PlanChangePatch` type has no `status` field — this is a type-system guarantee. Status transitions are `/change:execute`'s sole prerogative via `specify plan transition`.
- **Hand-editing `plan.yaml` or `.specify/slices/<name>/.metadata.yaml`.** Always route through the CLI so the single-writer invariant holds.

`/spec:drop` does not mutate `plan.yaml` directly: it terminates the active slice, and `/change:execute` then issues `specify plan transition <name> failed` or `blocked` based on the upstream outcome.

### Allow/forbid table

| Operation                                                | `define` | `build` | `merge` | `drop` | Notes                                                          |
| -------------------------------------------------------- | :------: | :-----: | :-----: | :----: | -------------------------------------------------------------- |
| `specify plan add <new-name> ...`                        |    ✓     |    ✓    |    ✓    |   ✗    | Surfacing a neighbouring slice is a phase-time discovery.     |
| `specify plan amend <current-name> --depends-on ...`     |    ✓     |    ✓    |    ✓    |   ✗    | Non-`status` fields only; may target the active entry.         |
| `specify plan amend ... status=...`                      |    ✗     |    ✗    |    ✗    |   ✗    | Type-system guarantee: `PlanChangePatch` has no `status`.      |
| `specify plan transition <name> <state>`                 |    ✗     |    ✗    |    ✗    |   ✗    | Driver-only — `/change:execute` owns plan-status transitions.    |
| Hand-edit `plan.yaml` or `.metadata.yaml`                |    ✗     |    ✗    |    ✗    |   ✗    | Always route through the CLI; preserves single-writer invariant. |
