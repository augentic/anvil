# Draft Skill Runbook

Operational detail for `/change:draft`. The SKILL.md keeps only the orientation surface (Critical Path + Reference table + Guardrails); everything procedural lives here.

## Overview

Specify at authoring time is a small stack — mirror of the execution stack documented in [`../../execute/SKILL.md`](../../execute/SKILL.md):

1. **Plan CLI** (`specify plan {add, amend, next, status, doctor, lock, transition, validate, archive}`) — the library-backed `specify` verbs that read and write `plan.yaml`. The shared single-writer rules live in [`../../../references/plan-single-writer.md`](../../../references/plan-single-writer.md).
2. **Authoring skill** (`/change:draft`, this one) — the Layer 2 authoring counterpart that runs the planning brief pipeline and shells out to `specify plan add` for each accepted slice.
3. **Driver skill** (`/change:execute`) — the Layer 2 automation that consumes the plan this skill authored.
4. **Finalize skill** (`/change:finalize`) — the Layer 2 close-out that pushes branches, observes PR state, and runs `specify change finalize` once every PR is `MERGED`.

The on-disk contracts the authoring skill depends on are:

| File / directory | Owner | Role |
|---|---|---|
| `plan.yaml` | library (`Plan::{create, amend, transition, archive}`) | Ordered change list with per-entry status; write boundaries are in [`../../../references/plan-single-writer.md`](../../../references/plan-single-writer.md). |
| `change.md` | library (`Change::draft`) | Operator brief; scaffolded together with `plan.yaml` by `specify change draft`. Atomic refusal when either file already exists. |
| `registry.yaml` | library (`Registry::{add, remove, validate}`) | Registered project list; validated in step 3 before any brief work. |
| `.specify/plans/<name>/` | skill (planning briefs) | Working directory for authoring artefacts — `discovery.md`, optional `workspace.md` (multi-repo), `proposal.md`, `analyze/<key>/metadata.json` (legacy-code). Swept by `specify plan archive` alongside the plan itself. |
| `briefs/<capability>/{discovery,propose}.md` | skill (this directory) | Per-capability planning briefs the skill renders for steps 4(a) and 4(d). Bundled here under `briefs/omnia/` and `briefs/vectis/`; further capabilities ship their planning brief variant alongside. The capability manifest schema rejects a `pipeline.plan` block — planning is orchestration, not capability-owned slice work, so the briefs ride with this skill rather than the capability manifest. |

## Invocation

```text
/change:draft <change-name> \
    [from <path>...] [against <path>] [source <key>=<path-or-url>...] \
    [focus <area>] [extend] [dry-run]
```

Positional grammar, kind suffixes, and the input-sufficiency rule live in [`../../../references/plan-invocation.md`](../../../references/plan-invocation.md).

## Input kinds (normative)

Every input eventually analysed by the draft flow — whether slash-supplied (`from` / `against` / `source`) or brief-supplied (`change.md:inputs[].kind`) — is classified by a **closed kind enum**:

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

## Core loop (six steps)

Follow these steps in order on every invocation. Each step is normative; every shell-out is to the `specify` CLI; this skill writes nothing to `plan.yaml` directly.

```text
1. Pre-flight — parse inputs; resolve source paths; assert
   plan.yaml absent (or extend).

   - Validate <change-name> as kebab-case. Reject with a hard
     exit on failure.
   - Require at least one of from, against, source, or a
     populated change-brief `inputs` list. Discover the brief's
     inputs via `specify change show --format json`
     (exit 0 and `"brief": null` ⇒ brief absent ⇒ no brief inputs;
     `"frontmatter.inputs": []` ⇒ present but empty ⇒ no brief
     inputs). A bare /change:draft <name> with neither slash
     inputs nor populated change-brief inputs is still a hard
     exit.
   - If plan.yaml exists and extend was NOT supplied,
     refuse with a diagnostic pointing at `specify plan archive`.
     (There is no force positional. Overwriting an existing plan
     would drop audit history.)
   - If extend was supplied but plan.yaml does NOT
     exist, also refuse — there is nothing to extend.

2. Brief scaffold — scaffold the brief and plan together.

     specify change draft <change-name> \
         [--source <key>=<path-or-url> ...]

   Writes both `change.md` (operator brief frontmatter
   template) and `plan.yaml` (carrying the change `name`
   and the supplied `source` entries in the top-level
   `sources` map). `changes: []` until step 4(d) populates
   the plan. Atomic refusal: refuses with `already-exists`
   when either file is present, leaving both untouched.

   Skipped entirely when extend is set.

   Manual fallback (when running by hand):

     specify change draft <name>
     $EDITOR change.md

   A non-zero exit from `specify change draft` (e.g. `<name>`
   failed kebab-case validation, or a previous run partially
   scaffolded the brief) surfaces verbatim and exits the skill
   non-zero. Remove the half-written `change.md` (or fix the
   name) and re-run.

3. Registry validate — `specify registry validate`.

   Three branches based on the resolved registry:

   a. Empty registry + greenfield discovery. The discovery brief
      may surface a `## Proposed registry topology` block in
      step 4(a); the greenfield-bootstrap path inside step 4(a)
      then runs `specify registry add` once per accepted entry.
      `/change:draft` does NOT call `specify registry add` from
      this step.
   b. Multi-project registry. `specify registry validate`
      enforces the `description-missing-multi-repo` invariant.
      Refusal exits non-zero pointing the operator at
      `specify registry add <name> --description "..."` for each
      entry missing a description.
   c. Single-project registry. Pass-through — the validate
      output is reported and the loop continues.

   Operator-actionable validation failures (missing description,
   kebab-case violation, invalid URL, capability identifier typo)
   halt the loop with the validator's diagnostic verbatim. The
   operator amends `registry.yaml` via `specify registry add` /
   `specify registry remove`, runs `specify workspace sync` to
   refresh clones, and re-runs the skill.

4. Plan brief pipeline — run from `capability.yaml`.

   The briefs are bundled with this skill under
   `briefs/<capability>/`. Resolve the active capability via:
     specify capability resolve --format json

   Then load `briefs/<capability>/discovery.md` and
   `briefs/<capability>/propose.md` from this skill directory and
   run each in order:
     a. discovery       — see ../discovery.md (greenfield registry bootstrap also).
     b. sync-workspace  — see ../sync-workspace.md (multi-repo only).
     c. survey          — see ../../survey/SKILL.md (legacy-code sources only;
                           drives the per-language enumeration brief and
                           validates the result before sizing; skip when the
                           change has no legacy-code inputs).
     d. propose         — see ../propose.md.
     e. assignment      — see ../assignment.md (multi-repo only; includes
                           the registry-proposal sub-step for unresolved
                           project names).

5. Final validation gate.

     specify plan validate

   Runs the CLI validator against the authored plan. Report
   every `ValidationResult` verbatim. Non-zero exit on any result
   with `level == Error`. A clean validate is the contract the
   skill owes its caller.

6. Hand-off summary.

   Print:
     - Slice count (entries written via `specify plan add`).
     - Target projects (each entry's `project`, when assigned).
     - Any `Warning`-level findings from step 5 the operator
       should see before executing.

   Then point the operator at:
     - `specify plan status` — review the authored plan.
     - `specify plan amend` — edit per-entry fields if needed.
     - `/change:execute loop` — start executing the plan.

   Non-zero exit on any earlier step's hard failure; zero exit on
   a clean validate.
```

## Single-writer invariant

Every plan write this skill performs follows [`../../../references/plan-single-writer.md`](../../../references/plan-single-writer.md): create the shell (alongside `change.md`) through `specify change draft`, add accepted entries through `specify plan add`, amend only assignment fields through `specify plan amend`, and never call `specify plan transition` from the authoring step.

## Working directory (`.specify/plans/<name>/`)

Authoring artefacts live under `.specify/plans/<change-name>/`, mirroring the change name recorded in `plan.yaml`:

```text
.specify/
├── plan.yaml                       # the authored plan
└── plans/
    └── <change-name>/
        ├── discovery.md            # from the discovery brief (step 4a)
        ├── workspace.md            # from sync-workspace (step 4b; multi-repo only)
        ├── survey.md               # from /change:survey (step 4c; legacy-code sources only)
        ├── survey/                  # /change:survey artefacts: `staged/<source-key>.json` (LLM-produced candidates),
        │                            # `sources.yaml` (batch input), and canonical `<source-key>/{metadata.json,surfaces.json}` sidecars
        ├── proposal.md             # from the propose brief (step 4d) + assignment table (step 4e)
        └── analyze/                # `/change:analyze` sidecars (documentation): `<source-key>/metadata.json`
```

The working directory is created lazily — by the discovery brief itself when it writes `discovery.md`, not by the skill scaffold. Step 2 (`specify change draft`) does not create it.

`.specify/plans/<change-name>/analyze/<key>/` is the **tier-1** legacy-source clone — read-only, ephemeral, and bound to this change. The **tier-2** registered project clones materialised by step 4(b) live separately under `.specify/workspace/<project>/`, are read-write during execution, and outlive any single change. See [Workspace Tiers](../../../../../docs/explanation/workspace-tiers.md) for the full contrast.

On archive, `specify plan archive` sweeps this directory alongside `plan.yaml` into `.specify/archive/plans/<name>-<YYYYMMDD>/`, preserving the authoring trail with the plan it produced.

## Modes

| Mode | Behaviour |
|---|---|
| Default (no positional) | Run the six-step loop unchanged. |
| `extend` | Append to an existing plan; skip step 2; reuse discovery; collisions silently skipped. |
| `dry-run` | Read-only preview; suppress every write under `.specify/`. |

Per-mode deltas, dry-run write prohibitions, and `extend` collision rules live in [`../../../references/plan-modes.md`](../../../references/plan-modes.md).

## Verb hygiene

Every shell-out this skill performs is listed below so reviewers can grep for accidental drift:

| Step | Verb |
|---|---|
| Pre-flight | `specify --version`, `specify change show --format json` |
| 2 Brief scaffold | `specify change draft <name> [--source <key>=<path-or-url> ...]` |
| 3 Registry validate | `specify registry validate`, `specify registry show --format json` |
| 4 Plan brief pipeline | `specify capability resolve --format json`, `/change:analyze`, `/change:survey`, `specify plan add`, `specify plan amend`, `specify registry add`, `specify workspace sync` |
| 5 Plan validate | `specify plan validate` |
| 6 Hand-off | `specify plan status` (preview only — the skill prints the summary; the operator runs `status` themselves) |

## Non-goals

- **Execute the plan.** Never. Execution is `/change:execute`'s concern. `/change:draft` exits with a hand-off summary that points the operator at `specify plan status`, `specify plan amend`, and `/change:execute loop`; there is no automatic transition from authoring to execution.
- **Push branches, observe PRs, or finalize the change.** Never. Those are `/change:finalize`'s concern.
- **Modify existing plan entries.** Never. `extend` is append-only; pre-existing entries are left untouched. Editing a pending entry mid-authoring is done via `specify plan amend` by the human, not by this skill.
- **Skip `specify plan validate`.** Never. Step 5 is unconditional — every run ends with a validation gate, and a non-clean validate exits non-zero.
- **Invoke `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/change:execute`, or `/change:finalize`.** Never. `/change:draft` only invokes the planning briefs bundled with this skill under `briefs/<capability>/`, plus the `specify` CLI for scaffolding, registry validation, entry creation, and final validation.
- **Hold a driver lock.** Never. `.specify/plan.lock` is reserved for `/change:execute`; authoring runs outside that lock.
- **Write `plan.yaml` directly.** Never. Every write follows [`../../../references/plan-single-writer.md`](../../../references/plan-single-writer.md).
- **Clone git URLs from this skill.** Never for **discovery** inputs: `documentation` source URLs are passed to `/change:analyze`; `legacy-code` source URLs are passed to `/change:survey`. Multi-repo **workspace** materialisation is exclusively `specify workspace sync`, invoked only in the sync-workspace step when `len(registry.projects) > 1`.
- **Merge PRs.** Never. PR observation and the merge wait belong to `/change:finalize`; the operator merges through the forge UI or a hand-run `gh pr merge`.
- **Author propose brief bodies.** Never. The propose brief body is owned by the capability; the skill only drives the accept / edit / reject loop against whatever the brief emits.
- **Auto-repair a failing `specify plan validate`.** Never. Step 5's validation gate is read-only; any `Error`-level finding surfaces to the human with a recommended `specify plan amend` / `specify plan transition skipped` fix, never an in-skill edit.

The state the skill mutates:

1. `plan.yaml` through the CLI verbs allowed by [`../../../references/plan-single-writer.md`](../../../references/plan-single-writer.md).
2. `change.md` through `specify change draft` (step 2 only; never edited directly).
3. `registry.yaml` indirectly via the assignment-step registry-proposal sub-step (`specify registry add` + `specify workspace sync`); never written by the skill itself.
4. `.specify/plans/<change-name>/discovery.md` written by the discovery brief (step 4a).
5. `.specify/plans/<change-name>/survey.md`, the staged candidates under `.specify/plans/<change-name>/survey/staged/<source-key>.json`, the `--sources` batch file at `.specify/plans/<change-name>/survey/sources.yaml`, and the canonical sidecars under `.specify/plans/<change-name>/survey/<source-key>/{surfaces,metadata}.json` written by `/change:survey` (step 4c, legacy-code sources only).
6. `.specify/plans/<change-name>/proposal.md` written by the propose brief (step 4d).
7. `.specify/plans/<change-name>/workspace.md` written by step 4(b) when the registry declares more than one project.

No other on-disk state is written by `/change:draft` itself.

## References

- [RFC-13: Extensibility](../../../../../rfcs/archive/rfc-13-extensibility.md) — pipelines may not declare a `pipeline.plan` block; planning briefs ship with this skill.
