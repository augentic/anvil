---
name: change-plan
description: "Authors `plan.yaml` for a change on the change surface via the planning brief pipeline; with the `orchestrate` positional, drives the cross-repo umbrella end to end. Use when scoping a new change or coordinating multi-repo execution from a single command on the `/change` surface."
argument-hint: "<change-name>"
---

## Critical Path (Quick Reference)

1. **Parse and validate inputs** — validate `<change-name>` as kebab-case. Require at least one of `from`, `against`, `source`, or a populated `change.md:inputs` (also accepts the legacy `initiative.md:inputs` until the operator migrates). Refuse if `plan.yaml` already exists (unless `extend`).
2. **Scaffold the plan** — `specify change plan create <change-name> [--source <key>=<path-or-url> ...]`. Skipped under `extend`.
3. **Run the plan brief pipeline** from `capability.yaml`:
   - **(a) Discovery** — invoke the discovery brief via `/spec:analyze`; writes `discovery.md`. May surface a `## Proposed registry topology` block that triggers the **greenfield registry bootstrap** before step 3(b) when no `registry.yaml` exists yet. See [discovery.md](discovery.md).
   - **(b) Sync peers** (multi-repo only) — discovery-time `specify workspace sync` (may sync all peers) + author `workspace.md`. Execution-time sync is separate and prepares only the selected entry's project unless the operator asks for more. See [sync-peers.md](sync-peers.md).
   - **(c) Propose** — run the propose brief; iterate accept/edit/reject/abort per slice; `specify change plan add` for each accepted slice. See [propose.md](propose.md).
   - **(d) Assignment** (multi-repo only) — infer `project` per entry; `specify change plan amend --project <project>`. When an unresolved row names a project that does not exist in `registry.yaml`, run the **registry-proposal sub-step** — `specify registry add` + `specify workspace sync` — before continuing. See [assignment.md](assignment.md).
4. **Validate** — `specify change plan validate`. Non-zero exit on any `Error`-level finding. Never skip this step.
5. **Exit with hand-off summary** — point the operator at `specify change plan status` and `/change:execute loop`.

**Orchestration mode (`orchestrate`).** When `orchestrate` is set, after step 5 above the skill continues into the cross-repo umbrella sequence (brief → registry → plan → execute → push/PR handoff → finalize after operator merge). The plan-authoring half of orchestration delegates to the same default mode documented above. See [orchestration.md](orchestration.md) for the full sequence, [shapes.md](shapes.md) for shape inference / validation, and [re-entry.md](re-entry.md) for the idempotent re-entry algorithm.

# Plan skill

Author `plan.yaml` for a change by running the planning brief pipeline declared in the active capability's `capability.yaml`. `/change:plan` is the Layer 3 authoring counterpart to `/change:execute`: one *writes* the plan, the other *runs* it.

> **Status.** Layer 3 is fully landed: discovery (step 3(a)) routes through `/spec:analyze`; when the registry declares **more than one project**, a **sync-peers** step (3(b)) runs `specify workspace sync` and authors `workspace.md` before propose (step 3(c)); after propose, an **assignment** step (3(d)) infers and writes `project` per entry for multi-repo routing.

## Overview

Specify at authoring time is a three-layer stack — mirror of the execution stack documented in [`../execute/SKILL.md`](../execute/SKILL.md):

1. **Plan CLI** (`specify change plan {create, add, amend, next, status, doctor, lock, transition, validate, archive}`) — the library-backed verbs that read and write `plan.yaml`. The shared single-writer rules live in [plan-single-writer.md](../../references/plan-single-writer.md).
2. **Authoring skill** (`/change:plan`, this one) — the Layer 3 driver that runs the planning brief pipeline and shells out to `specify change plan add` for each accepted slice.
3. **Driver skill** (`/change:execute`) — the Layer 2 automation that consumes the plan this skill authored.

The on-disk contracts the authoring skill depends on are:

| File / directory | Owner | Role |
|---|---|---|
| `plan.yaml` | library (`Plan::{create, amend, transition, archive}`) | Ordered change list with per-entry status; write boundaries are in [plan-single-writer.md](../../references/plan-single-writer.md). |
| `.specify/plans/<name>/` | skill (planning briefs) | Working directory for authoring artefacts — `discovery.md`, optional `workspace.md` (multi-repo), `proposal.md`, `analyze/<key>/metadata.json` (legacy-code). Swept by `specify change plan archive` alongside the plan itself. |
| `briefs/<capability>/{discovery,propose}.md` | skill (this directory) | Per-capability planning briefs the skill renders for steps 3(a) and 3(c). Bundled here under `briefs/omnia/` and `briefs/vectis/`; further capabilities ship their planning brief variant alongside. The capability manifest schema rejects a `pipeline.plan` block — planning is orchestration, not capability-owned slice work, so the briefs ride with this skill rather than the capability manifest. |

## Invocation

```text
/change:plan <change-name> \
    [from <path>...] \
    [against <path>] \
    [source <key>=<path-or-url>...] \
    [focus <area>] \
    [extend] \
    [dry-run] \
    [orchestrate] \
    [shape migrate-legacy|new-feature|update-existing]
```

Positional arguments:

- **`<change-name>`** — kebab-case identifier; becomes the plan's top-level `name` field. Validated with the same rules as slice names (regex `^[a-z][a-z0-9-]*$`) before any other work. An invalid name is a hard exit with a clear diagnostic — the skill never rewrites or "helps" the name.
- **`from <path>`** — artefact file(s) or directory describing the target shape for greenfield authoring. Repeatable. Consumed by the discovery brief (L3.F). Kind defaults to `documentation`; override via `:<kind>` suffix (see §Kind defaults for positional inputs).
- **`against <path>`** — an existing codebase to delta against, used for refactor or modernisation changes. Consumed by the discovery brief (L3.F). Kind defaults to `legacy-code`; override via `:<kind>` suffix.
- **`source <key>=<path-or-url>`** — a named source for migration. Repeatable. The `key` is a kebab-case identifier recorded in the plan's top-level `sources` map and referenced by individual plan entries via their `sources` list; the `value` is either a local filesystem path or a git URL. The skill forwards the tuple verbatim; cloning (if any) is the discovery brief's concern via `/spec:analyze` (which inlines a guarded `git clone` snippet — see the *Cloning a source tree* subsection in [`../../../spec/skills/analyze/SKILL.md`](../../../spec/skills/analyze/SKILL.md)). Kind defaults to `legacy-code`; override via `:<kind>` suffix.
- **`focus <area>`** — optional scoping hint for the propose brief (L3.G). Free-form string; the propose brief decides how to interpret it.
- **`extend`** — add to an existing `plan.yaml` instead of refusing. See §Modes → `extend` for the full contract.
- **`dry-run`** — emit the readiness report and the proposed plan to stdout; write nothing. See §Modes → `dry-run`.
- **`orchestrate`** — enable orchestration mode: run the cross-repo umbrella after the authoring loop. The umbrella pushes per-project PRs, stops for operator merge when PRs are still open, and later finalizes after `specify change finalize` verifies every PR is merged. See [orchestration.md](orchestration.md). Required when `shape` is supplied.
- **`shape migrate-legacy|new-feature|update-existing`** — explicit shape override under `orchestrate`. Inferred from the supplied inputs when omitted. Rejected with a hard diagnostic when `orchestrate` is absent. See [shapes.md](shapes.md).
- **`auto-merge`** — retired. Treat this positional as an error that explains `/change:plan <name> orchestrate` never calls `specify workspace merge` or `gh pr merge`. Operators merge the PRs opened by `specify workspace push` through the forge UI or an explicit hand-run `gh pr merge`, then re-run the umbrella to finalize.

At least one of `from`, `against`, `source`, or a populated change-brief `inputs` list must be supplied. A bare `/change:plan <name>` with no slash inputs **and** no change brief (or a brief with empty `inputs`) is a hard exit — the skill cannot decide the change's shape without at least one input.

When the change-brief `inputs` list is the only source of inputs, the skill reads them via `specify change show --format json` before entering the core loop and treats each entry as if it had been supplied as slash-positionals: `kind: legacy-code` entries route through the same path as `source <k>=<path>:legacy-code`, and `kind: documentation` entries route through the `from` path. Both documentation and legacy-code dispatch are live via `/spec:analyze`. Plan-time `/spec:extract` call sites have been fully retired; `/spec:extract` now runs only at `/spec:define` time with scope inferred from the slice's description.

## Input kinds (normative)

Every input eventually analysed by the plan flow — whether slash-supplied (`from` / `against` / `source`) or brief-supplied (`change.md:inputs[].kind`) — is classified by a **closed kind enum**:

| kind            | Purpose                                                                                                 |
| --------------- | ------------------------------------------------------------------------------------------------------- |
| `legacy-code`   | Source code to be inferred into capability summaries at plan time and extracted per-slice at define time. |
| `documentation` | Prose, PDFs, runbooks, API specs — parsed for capability summaries, constraints, and open questions.    |

The enum is closed: any other value is a hard error at the analyse phase (see `/spec:analyze`). This enum is frozen — NEVER extend it from this skill. This keeps the plan-time discovery contract auditable — every line in `discovery.md` is traceable to the kind-branch that produced it.

## Kind defaults for positional inputs

When an input is supplied via a positional input, its kind is determined as follows. The suffix syntax applies to `source`, `from`, and `against` identically, though in practice only `source` tends to carry explicit suffixes.

| Positional input            | Default kind    | Explicit override                |
| ------------------- | --------------- | -------------------------------- |
| `source <k>=<p>`  | `legacy-code`   | `source <k>=<p>:<kind>`        |
| `from <p>`        | `documentation` | `from <p>:<kind>`              |
| `against <p>`     | `legacy-code`   | `against <p>:<kind>`           |

Inputs supplied via `change.md:inputs` carry their `kind:` explicitly in the frontmatter; no default is applied.

An explicit `:<kind>` suffix whose value is not in the closed enum is a hard exit before the core loop begins (same diagnostic as `/spec:analyze`'s unknown-kind error). The suffix grammar is `<value>[:<kind>]`, where `<kind>` is one of `legacy-code` or `documentation` (kebab-case, case-sensitive).

## Core loop (five steps)

Follow these steps in order on every invocation. Each step is normative; every shell-out is to the Layer 1 `specify` CLI; this skill writes nothing to `plan.yaml` directly.

```text
1. Parse inputs; resolve source paths; assert plan.yaml absent
   (or extend).

   - Validate <change-name> as kebab-case. Reject with a hard
     exit on failure.
   - Require at least one of from, against, source, or a
     populated change-brief `inputs` list. Discover the brief's
     inputs via `specify change show --format json`
     (exit 0 and `"brief": null` ⇒ brief absent ⇒ no brief inputs;
     `"frontmatter.inputs": []` ⇒ present but empty ⇒ no brief
    inputs). A bare /change:plan <name> with neither slash inputs nor
     populated change-brief inputs is still a hard exit.
   - If plan.yaml exists and extend was NOT supplied,
     refuse with a diagnostic pointing at `specify change plan archive`.
     (There is no force positional. Overwriting an existing plan would drop
     audit history.)
   - If extend was supplied but plan.yaml does NOT
     exist, also refuse — there is nothing to extend.

2. Scaffold the plan.

     specify change plan create <change-name> \
         [--source <key>=<path-or-url> ...]

   Writes an empty plan.yaml with just the change
   `name` and the supplied `source` entries in the top-level
   `sources` map. `changes: []` until step 3(c) populates it.

   Skipped entirely when extend is set.

3. Run the planning brief pipeline.

   The briefs are bundled with this skill under
   `briefs/<capability>/`. Resolve the active capability via:
     specify capability resolve --format json

   Then load `briefs/<capability>/discovery.md` and
   `briefs/<capability>/propose.md` from this skill directory and
   run each in order:
     a. discovery   — see discovery.md (greenfield registry bootstrap also).
     b. sync-peers  — see sync-peers.md (multi-repo only).
     c. propose     — see propose.md.
     d. assignment  — see assignment.md (multi-repo only; includes
                      the registry-proposal sub-step for unresolved
                      project names).

   RFC-13 §3.11 moved these briefs out of `capability.yaml:pipeline.plan`
   (now actively rejected by the manifest schema) — planning is
   orchestration, not capability-owned slice work, so the briefs ride
   with this skill rather than the capability manifest.

4. Final validation gate.

     specify change plan validate

   Runs the Layer 1 validator against the authored plan. Report
   every `ValidationResult` verbatim. Non-zero exit on any result
   with `level == Error`. A clean validate is the contract the
   skill owes its caller.

5. Exit with a hand-off summary.

   Point the human at:
     - `specify change plan status` — review the authored plan.
     - `/change:execute loop` — start executing it (Layer 2).

   Non-zero exit on any earlier step's hard failure; zero exit on
   a clean validate.
```

## Single-writer invariant

Every plan write this skill performs follows [plan-single-writer.md](../../references/plan-single-writer.md): create the shell through `specify change plan create`, add accepted entries through `specify change plan add`, amend only assignment fields through `specify change plan amend`, and never call `specify change plan transition` from the authoring step.

## Working directory (`.specify/plans/<name>/`)

Authoring artefacts live under `.specify/plans/<slice-name>/`, mirroring the `.specify/slices/<name>/` pattern used by the phase skills:

```text
.specify/
├── plan.yaml                       # the authored plan
└── plans/
    └── <slice-name>/
        ├── discovery.md            # from the discovery brief (step 3a)
        ├── workspace.md            # from sync-peers (step 3b; multi-repo only)
        ├── proposal.md             # from the propose brief (step 3c) + assignment table (step 3d)
        └── analyze/                # `/spec:analyze` sidecars (legacy-code): `<source-key>/metadata.json`
```

The working directory is created lazily — by the discovery brief itself when it writes `discovery.md`, not by the skill scaffold. Step 2 (`specify change plan create`) does not create it.

`.specify/plans/<slice-name>/analyze/<key>/` is the **tier-1** legacy-source clone — read-only, ephemeral, and bound to this change. The **tier-2** registered project clones materialised by step 3(b) live separately under `.specify/workspace/<project>/`, are read-write during execution, and outlive any single change. See [Workspace Tiers](../../../../docs/explanation/workspace-tiers.md) for the full contrast.

On archive, `specify change plan archive` sweeps this directory alongside `plan.yaml` into `.specify/archive/plans/<name>-<YYYYMMDD>/`, preserving the authoring trail with the plan it produced.

## Modes

Each mode below describes only the *delta* from the core five-step loop. The default mode runs the loop unchanged; `extend` and `dry-run` each relax or suppress specific writes.

### Default (no mode positional)

Run the five-step loop exactly as written. `plan.yaml` is initialised via step 2, populated via step 3(c), validated in step 4. A pre-existing `plan.yaml` is refused at step 1 (the operator is pointed at `specify change plan archive`).

### `extend`

Add to an existing `plan.yaml` instead of refusing. The skill-level contract is:

- **Step 1 refuses when `plan.yaml` is absent.** `extend` is an explicit "I know there's a plan here" signal; the skill never silently creates a fresh plan under `extend`.
- **Step 2 (`specify change plan create`) is skipped entirely.**
- **Step 3(a) is skipped when `.specify/plans/<slice-name>/discovery.md` already exists**, with a log line `Discovery already present; reusing existing inventory.` Discovery is explicitly a one-shot artefact; an operator who wants to refresh it archives the plan and re-runs without `extend`. When `discovery.md` does not yet exist under `extend` (e.g. a plan authored by hand, or an earlier run aborted), step 3(a) runs normally.
- **Step 3(c) skips collisions silently.** Draft slices whose proposed `name` collides with an existing plan entry are recorded in `proposal.md` with decision `skip-existing` and the existing entry's name in the "Plan entry" column; the human is not re-prompted. Slices whose names do not collide run through the usual accept / edit / reject / abort loop.
- **Sync-peers (step 3(b)):** when the registry declares more than one project, **do not** shell `specify workspace sync`. Still regenerate `.specify/plans/<slice-name>/workspace.md` from the existing `.specify/workspace/` cache (read-only walk) so propose stays deterministic without an implicit `git fetch`.
- **Pre-existing entries are never modified.** The skill never calls `specify change plan transition` on existing entries. The only `specify change plan amend` call is step 3(d) Assignment (`--project`), which tags newly created entries — it does not modify pre-existing ones.

No new positional is introduced beyond `extend`. A future change may add `force-discovery` if refreshing the inventory mid-plan becomes a real need.

### `dry-run`

Emit a readiness report, the would-be-produced capability inventory, and the would-be-proposed plan to stdout; write nothing. Dry-run folds the L3.E readiness gate, the L3.F discovery preview, and the L3.G propose preview into a single pass.

Under `dry-run` the skill MUST NOT:

- create `.specify/plans/<slice-name>/`;
- shell out to `specify change plan create`, `specify change plan add`, `specify change plan amend`, or `specify change plan transition`;
- shell out to **`specify workspace sync`** or write **`.specify/plans/<slice-name>/workspace.md`** (sync-peers dry-run rule);
- write any file under `.specify/` (including under `.specify/workspace/`).

The discovery brief's input-reading side (reading `from` files, invoking `/spec:analyze` against `source` / `against` inputs) runs under `dry-run` so the preview inventory is real; only the write to `discovery.md` and the `.specify/plans/<name>/` directory creation are suppressed. The propose brief's slice-decomposition pass also runs (the preview plan shape is real against the previewed inventory); the accept / edit / reject loop and every `specify change plan add` call are skipped.

The full output shape (banner / sources block / pipeline line / capability inventory preview / would-be-proposed plan / assignment preview) is pinned by `fixtures/dry-run/expected-output.md`, `fixtures/discovery/expected-discovery.md`, and `fixtures/propose/expected-proposal.md`. The `[dry-run]` banner on the first line is enough — body lines do not need a per-line prefix.

### `orchestrate`

Run the cross-repo umbrella sequence after the authoring loop completes. The orchestration mode is composition only — every step shells out to a verb that already exists in the v1 surface. It does not own the human PR merge; it only opens/updates PRs via `specify workspace push`, observes whether they have been merged, and invokes `specify change finalize` once the remote PR state is ready. See [orchestration.md](orchestration.md) for the full sequence, halts table, manual fallbacks, and verb hygiene; [shapes.md](shapes.md) for shape inference and validation; [re-entry.md](re-entry.md) for the idempotent resume algorithm.

Under `orchestrate dry-run`, the umbrella is observation-only end-to-end: the authoring loop runs in dry-run (per the §`dry-run` section above), and the execution, push, PR-observation, and finalize portions emit "would invoke" preview lines without invoking any phase skill, push, forge merge, or finalize. See [orchestration.md](orchestration.md) §"`dry-run` semantics".

## Non-goals

- **Execute the plan in default mode.** Never. Execution is `/change:execute`'s concern (Layer 2). `/change:plan` exits with a hand-off summary that points the operator at `/change:execute loop`; `orchestrate` composes that separate skill after authoring.
- **Modify existing plan entries.** Never. `extend` is append-only; pre-existing entries are left untouched. Editing a pending entry mid-authoring is done via `specify change plan amend` by the human, not by this skill.
- **Skip `specify change plan validate`.** Never. Step 4 is unconditional — every run ends with a validation gate, and a non-clean validate exits non-zero.
- **Invoke `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, or `/change:execute`.** Never. `/change:plan` only invokes the planning briefs bundled with this skill under `briefs/<capability>/`, plus the `specify change plan` CLI for scaffolding, entry creation, and validation.
- **Hold a driver lock.** Never. `.specify/plan.lock` is reserved for `/change:execute`; authoring runs outside that lock.
- **Write `plan.yaml` directly.** Never. Every write follows [plan-single-writer.md](../../references/plan-single-writer.md).
- **Clone git URLs from this skill.** Never for **discovery** inputs: `source` git URLs are passed through to `/spec:analyze` verbatim. Multi-repo **workspace** materialisation is exclusively `specify workspace sync` (Layer 1 CLI), invoked only in the sync-peers step when `len(registry.projects) > 1`.
- **Merge PRs.** Never. `specify workspace push` opens or updates PRs; the operator merges them through the forge UI or a hand-run `gh pr merge`; `specify change finalize` only verifies that remote state.
- **Author propose brief bodies.** Never. The propose brief body is owned by the capability; the skill only drives the accept / edit / reject loop against whatever the brief emits.
- **Auto-repair a failing `specify change plan validate`.** Never. Step 4's validation gate is read-only; any `Error`-level finding surfaces to the human with a recommended `specify change plan amend` / `specify change plan transition skipped` fix, never an in-skill edit.

The state the skill mutates:

1. `plan.yaml` through the CLI verbs allowed by [plan-single-writer.md](../../references/plan-single-writer.md).
2. `.specify/plans/<slice-name>/discovery.md` written by the discovery brief (step 3a).
3. `.specify/plans/<slice-name>/proposal.md` written by the propose brief (step 3c).
4. `.specify/plans/<slice-name>/workspace.md` written by step 3(b) when the registry declares more than one project.

No other on-disk state is written by `/change:plan` itself.

## Guardrails

- Never hand-edit `plan.yaml`; follow [plan-single-writer.md](../../references/plan-single-writer.md).
- Never skip `specify change plan validate` (step 4). A plan that ships to `/change:execute` without a clean validate is a regression.
- Validate `<slice-name>` before any filesystem read or CLI shell-out. A bad name should never leave a half-written plan behind.
- For `dry-run` specifically: the skill MUST NOT shell out to `specify change plan create`, `specify change plan add`, `specify change plan amend`, or `specify change plan transition`; MUST NOT create `.specify/plans/<name>/`; MUST NOT write `discovery.md` or any other file under `.specify/`. The discovery brief's input-reading side still runs so the stdout inventory preview is real.
- For `extend` specifically: step 2 is skipped in full; step 3(c) only appends entries via `specify change plan add` — it never calls `specify change plan transition` on existing entries. The only `specify change plan amend` call is step 3(d) Assignment (`--project`), which tags newly created entries, not pre-existing ones. Draft slices whose names collide with existing plan entries are skipped with decision `skip-existing` in `proposal.md`.
- Treat a missing `briefs/<capability>/discovery.md` or `briefs/<capability>/propose.md` for the active capability as a hard failure: print the resolved capability name and the expected brief paths, then exit non-zero. Do not speculate about brief ordering or fall back to a different capability's briefs.
