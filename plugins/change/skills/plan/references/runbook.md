# Plan Skill Runbook

Operational detail for `/change:plan`. The SKILL.md keeps only the orientation surface (Critical Path + Reference table + Guardrails); everything procedural lives here.

## Overview

Specify at authoring time is a small stack — mirror of the execution stack documented in [`../../execute/SKILL.md`](../../execute/SKILL.md):

1. **Plan CLI** (`specify change plan {create, add, amend, next, status, doctor, lock, transition, validate, archive}`) — the library-backed `specify` verbs that read and write `plan.yaml`. The shared single-writer rules live in [`../../../references/plan-single-writer.md`](../../../references/plan-single-writer.md).
2. **Authoring skill** (`/change:plan`, this one) — the Layer 2 authoring counterpart that runs the planning brief pipeline and shells out to `specify change plan add` for each accepted slice.
3. **Driver skill** (`/change:execute`) — the Layer 2 automation that consumes the plan this skill authored.

The on-disk contracts the authoring skill depends on are:

| File / directory | Owner | Role |
|---|---|---|
| `plan.yaml` | library (`Plan::{create, amend, transition, archive}`) | Ordered change list with per-entry status; write boundaries are in [`../../../references/plan-single-writer.md`](../../../references/plan-single-writer.md). |
| `.specify/plans/<name>/` | skill (planning briefs) | Working directory for authoring artefacts — `discovery.md`, optional `workspace.md` (multi-repo), `proposal.md`, `analyze/<key>/metadata.json` (legacy-code). Swept by `specify change plan archive` alongside the plan itself. |
| `briefs/<capability>/{discovery,propose}.md` | skill (this directory) | Per-capability planning briefs the skill renders for steps 3(a) and 3(c). Bundled here under `briefs/omnia/` and `briefs/vectis/`; further capabilities ship their planning brief variant alongside. The capability manifest schema rejects a `pipeline.plan` block — planning is orchestration, not capability-owned slice work, so the briefs ride with this skill rather than the capability manifest. |

## Invocation

```text
/change:plan <change-name> \
    [from <path>...] [against <path>] [source <key>=<path-or-url>...] \
    [focus <area>] [extend] [dry-run] \
    [orchestrate] [shape migrate-legacy|new-feature|update-existing]
```

Positional grammar, kind suffixes, and the input-sufficiency rule live in [`../../../references/plan-invocation.md`](../../../references/plan-invocation.md).

## Input kinds (normative)

Every input eventually analysed by the plan flow — whether slash-supplied (`from` / `against` / `source`) or brief-supplied (`change.md:inputs[].kind`) — is classified by a **closed kind enum**:

| kind            | Purpose                                                                                                 |
| --------------- | ------------------------------------------------------------------------------------------------------- |
| `legacy-code`   | Source code to be inferred into capability summaries at plan time and extracted per-slice at define time. |
| `documentation` | Prose, PDFs, runbooks, API specs — parsed for capability summaries, constraints, and open questions.    |

The enum is closed: any other value is a hard error at the analyse phase (see `/change:analyze`). This enum is frozen — NEVER extend it from this skill. This keeps the plan-time discovery contract auditable — every line in `discovery.md` is traceable to the kind-branch that produced it.

## Kind defaults for positional inputs

When an input is supplied via a positional input, its kind is determined as follows. The suffix syntax applies to `source`, `from`, and `against` identically, though in practice only `source` tends to carry explicit suffixes.

| Positional input            | Default kind    | Explicit override                |
| ------------------- | --------------- | -------------------------------- |
| `source <k>=<p>`  | `legacy-code`   | `source <k>=<p>:<kind>`        |
| `from <p>`        | `documentation` | `from <p>:<kind>`              |
| `against <p>`     | `legacy-code`   | `against <p>:<kind>`           |

Inputs supplied via `change.md:inputs` carry their `kind:` explicitly in the frontmatter; no default is applied.

An explicit `:<kind>` suffix whose value is not in the closed enum is a hard exit before the core loop begins (same diagnostic as `/change:analyze`'s unknown-kind error). The suffix grammar is `<value>[:<kind>]`, where `<kind>` is one of `legacy-code` or `documentation` (kebab-case, case-sensitive).

## Core loop (five steps)

Follow these steps in order on every invocation. Each step is normative; every shell-out is to the `specify` CLI; this skill writes nothing to `plan.yaml` directly.

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

2. Scaffold the brief and plan together.

     specify change create <change-name> \
         [--source <key>=<path-or-url> ...]

   Writes both `change.md` (operator brief frontmatter
   template) and `plan.yaml` (carrying the change `name`
   and the supplied `source` entries in the top-level
   `sources` map). `changes: []` until step 3(c) populates
   the plan. Atomic refusal: refuses with `already-exists`
   when either file is present, leaving both untouched.

   Skipped entirely when extend is set.

3. Run the planning brief pipeline.

   The briefs are bundled with this skill under
   `briefs/<capability>/`. Resolve the active capability via:
     specify capability resolve --format json

   Then load `briefs/<capability>/discovery.md` and
   `briefs/<capability>/propose.md` from this skill directory and
   run each in order:
     a. discovery   — see ../discovery.md (greenfield registry bootstrap also).
     b. sync-workspace  — see ../sync-workspace.md (multi-repo only).
     c. propose     — see ../propose.md.
     d. assignment  — see ../assignment.md (multi-repo only; includes
                      the registry-proposal sub-step for unresolved
                      project names).

4. Final validation gate.

     specify change plan validate

   Runs the CLI validator against the authored plan. Report
   every `ValidationResult` verbatim. Non-zero exit on any result
   with `level == Error`. A clean validate is the contract the
   skill owes its caller.

5. Exit with a hand-off summary.

   Point the human at:
     - `specify change plan status` — review the authored plan.
     - `/change:execute loop` — start executing it.

   Non-zero exit on any earlier step's hard failure; zero exit on
   a clean validate.
```

## Single-writer invariant

Every plan write this skill performs follows [`../../../references/plan-single-writer.md`](../../../references/plan-single-writer.md): create the shell (alongside `change.md`) through `specify change create`, add accepted entries through `specify change plan add`, amend only assignment fields through `specify change plan amend`, and never call `specify change plan transition` from the authoring step.

## Working directory (`.specify/plans/<name>/`)

Authoring artefacts live under `.specify/plans/<change-name>/`, mirroring the change name recorded in `plan.yaml`:

```text
.specify/
├── plan.yaml                       # the authored plan
└── plans/
    └── <change-name>/
        ├── discovery.md            # from the discovery brief (step 3a)
        ├── workspace.md            # from sync-workspace (step 3b; multi-repo only)
        ├── proposal.md             # from the propose brief (step 3c) + assignment table (step 3d)
        └── analyze/                # `/change:analyze` sidecars (legacy-code): `<source-key>/metadata.json`
```

The working directory is created lazily — by the discovery brief itself when it writes `discovery.md`, not by the skill scaffold. Step 2 (`specify change create`) does not create it.

`.specify/plans/<change-name>/analyze/<key>/` is the **tier-1** legacy-source clone — read-only, ephemeral, and bound to this change. The **tier-2** registered project clones materialised by step 3(b) live separately under `.specify/workspace/<project>/`, are read-write during execution, and outlive any single change. See [Workspace Tiers](../../../../../docs/explanation/workspace-tiers.md) for the full contrast.

On archive, `specify change plan archive` sweeps this directory alongside `plan.yaml` into `.specify/archive/plans/<name>-<YYYYMMDD>/`, preserving the authoring trail with the plan it produced.

## Modes

| Mode | Behaviour |
|---|---|
| Default (no positional) | Run the five-step loop unchanged. |
| `extend` | Append to an existing plan; skip step 2; reuse discovery; collisions silently skipped. |
| `dry-run` | Read-only preview; suppress every write under `.specify/`. |
| `orchestrate` | Default authoring loop, then the cross-repo umbrella sequence ([`../orchestration.md`](../orchestration.md), [`../shapes.md`](../shapes.md), [`../re-entry.md`](../re-entry.md)). |

Per-mode deltas, dry-run write prohibitions, and `extend` collision rules live in [`../../../references/plan-modes.md`](../../../references/plan-modes.md).

## Non-goals

- **Execute the plan in default mode.** Never. Execution is `/change:execute`'s concern. `/change:plan` exits with a hand-off summary that points the operator at `/change:execute loop`; `orchestrate` composes that separate skill after authoring.
- **Modify existing plan entries.** Never. `extend` is append-only; pre-existing entries are left untouched. Editing a pending entry mid-authoring is done via `specify change plan amend` by the human, not by this skill.
- **Skip `specify change plan validate`.** Never. Step 4 is unconditional — every run ends with a validation gate, and a non-clean validate exits non-zero.
- **Invoke `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, or `/change:execute`.** Never. `/change:plan` only invokes the planning briefs bundled with this skill under `briefs/<capability>/`, plus the `specify change plan` CLI for scaffolding, entry creation, and validation.
- **Hold a driver lock.** Never. `.specify/plan.lock` is reserved for `/change:execute`; authoring runs outside that lock.
- **Write `plan.yaml` directly.** Never. Every write follows [`../../../references/plan-single-writer.md`](../../../references/plan-single-writer.md).
- **Clone git URLs from this skill.** Never for **discovery** inputs: `source` git URLs are passed through to `/change:analyze` verbatim. Multi-repo **workspace** materialisation is exclusively `specify workspace sync`, invoked only in the sync-workspace step when `len(registry.projects) > 1`.
- **Merge PRs.** Never. `specify workspace push` opens or updates PRs; the operator merges them through the forge UI or a hand-run `gh pr merge`; `specify change finalize` only verifies that remote state.
- **Author propose brief bodies.** Never. The propose brief body is owned by the capability; the skill only drives the accept / edit / reject loop against whatever the brief emits.
- **Auto-repair a failing `specify change plan validate`.** Never. Step 4's validation gate is read-only; any `Error`-level finding surfaces to the human with a recommended `specify change plan amend` / `specify change plan transition skipped` fix, never an in-skill edit.

The state the skill mutates:

1. `plan.yaml` through the CLI verbs allowed by [`../../../references/plan-single-writer.md`](../../../references/plan-single-writer.md).
2. `.specify/plans/<change-name>/discovery.md` written by the discovery brief (step 3a).
3. `.specify/plans/<change-name>/proposal.md` written by the propose brief (step 3c).
4. `.specify/plans/<change-name>/workspace.md` written by step 3(b) when the registry declares more than one project.

No other on-disk state is written by `/change:plan` itself.

## References

- [RFC-13: Extensibility](../../../../../rfcs/archive/rfc-13-extensibility.md) — pipelines may not declare a `pipeline.plan` block; planning briefs ship with this skill.
