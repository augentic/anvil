# RFC-23: Change Lifecycle

> Status: Implemented - Depends: [RFC-9](archive/rfc-9-platform.md), [RFC-13](archive/rfc-13-extensibility.md) - See also: [RFC-20](rfc-20-survey.md)

## Abstract

Decompose today's `/change:plan` (default mode + `orchestrate` umbrella) into three peer skills with an explicit human seam between authoring and execution:

- `**/change:draft <name>**` — author `plan.yaml` and stop. Replaces `/change:plan` (default mode) and absorbs the umbrella's pre-execute steps (brief scaffold + registry validate).
- *(operator reviews `plan.yaml`, edits with `specify plan amend` if needed)*
- `**/change:execute loop**` — drive `/spec:define → /spec:build → /spec:merge` per slice until no eligible slice remains. Unchanged from today.
- `**/change:finalize <name>**` — push branches, observe PR state, and run `specify change finalize` once every PR is `MERGED`. Wraps the umbrella's post-execute tail (steps 5–7).

The new lifecycle reads `draft → execute → finalize`, deliberately mirroring `/spec`'s `define → build → merge` rhythm at the change layer. The `orchestrate` mode is removed — there is no longer a single command that drives plan-time through merge in one shot. The human pause between authoring and execution is the design.

## Motivation

Three problems compound on `/change:plan` as it stands today:

- **Double duty obscures layering.** `/change:plan <name>` authors `plan.yaml`. `/change:plan <name> orchestrate` authors `plan.yaml`, runs `/change:execute`, pushes branches, observes PR state, and runs `specify change finalize`. New operators reading "the plan skill" reasonably expect it to plan; they do not expect it to drive PRs to merge.
- **The plan-review pause is a fiction.** `orchestrate` proceeds straight from `specify plan validate` into `/change:execute loop` with no operator gate. Operators who actually want to review `plan.yaml` (run `specify plan amend`, compare against `survey.md`, hand the plan to a teammate) have to know to invoke the manual flow instead. The umbrella *implies* you can fire-and-forget; the manual flow *delivers* the review point. Naming both as modes of "plan" hides which is which.
- **Asymmetric lifecycle vocabulary.** The spec layer reads `/spec:define → /spec:build → /spec:merge` — three peer skills, one rhythm. The change layer reads `/change:plan [orchestrate] → /change:execute → specify change finalize` — two skills with a mode positional and a CLI verb tail. The asymmetry forces operators to learn two different lifecycles for what is structurally the same shape (author → execute → close).

This RFC fixes all three by collapsing the umbrella, naming the planner for what it produces (a draft), and giving the post-execute tail its own peer skill.

## Design

### Principles

1. **Explicit human seam.** Authoring (`/change:draft`) ends at "plan validated, hand back to operator." Execution (`/change:execute`) starts when the operator decides it does. There is no automatic transition between them.
2. **Lifecycle symmetry with `/spec`.** Three peer skills at the change layer (`draft`, `execute`, `finalize`) match three peer skills at the spec layer (`define`, `build`, `merge`). Operators learn one rhythm and apply it at two granularities.
3. **Composition only.** Each new skill wraps existing CLI verbs and existing peer skills. No new on-disk state, no new lifecycle classifications, no new halt vocabulary. `/change:finalize` is thin today; it stays thin.
4. **Idempotent re-entry.** Each skill reads on-disk state as the source of truth and re-enters cleanly. `/change:draft` re-runs through `extend` mode; `/change:execute` re-runs through its existing self-heal pass; `/change:finalize` re-checks plan terminal status and PR state on every invocation.
5. **No fallback umbrella.** There is no `/change:do-everything` or `/change:plan orchestrate` survivor. Operators who want one-command behaviour can compose the three skills in their own shell wrapper; the framework does not ship one.

### The new lifecycle


| Stage        | Skill                             | Owns                                                                                                                                                                      | Halts                                                                                                             |
| ------------ | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **Plan**     | `/change:draft <name> [...]`      | brief scaffold, registry validate, plan brief pipeline (discovery → [sync-workspace] → [survey, synthesise per RFC-20] → propose → [assignment]), `specify plan validate` | propose-loop abort, `specify plan validate` failure, registry validation failure                                  |
| *(seam)*     | *operator reviews `plan.yaml*`    | operator runs `specify plan amend`, `specify plan status`, etc. as needed                                                                                                 | n/a                                                                                                               |
| **Execute**  | `/change:execute loop`            | per-slice `/spec:define → /spec:build → /spec:merge`, status transitions, driver lock                                                                                     | `stuck`, `halted`, `driver-interrupted`, `registry-amendment-required`                                            |
| *(seam)*     | *operator reviews implementation* |                                                                                                                                                                           | n/a                                                                                                               |
| **Finalize** | `/change:finalize <name> [...]`   | `specify workspace push`, `gh pr list` observation, `specify change finalize`                                                                                             | `non-terminal-entries`, `failed`, `pending-checks`, `failed-checks`, `pr-not-merged`, finalize CLI guard refusals |


Re-entry across all three skills: fix the cause, re-run the same skill. Nothing tracks "where the operator was" outside of `plan.yaml` and the change brief artefacts on disk.

### `/change:draft` — what it owns

The draft skill folds today's `/change:plan` default mode plus the umbrella's pre-execute pre-flight (steps 1–2 of the seven-step sequence). Its Critical Path:

1. **Pre-flight** — validate `<change-name>` as kebab-case; require at least one of `from`, `against`, `source`, or a populated `change.md:inputs`; refuse if `plan.yaml` already exists (unless `extend`).
2. **Brief scaffold** — `specify change draft <change-name> [--source <key>=<path-or-url> ...]` when `change.md` is absent. Writes `change.md` and `plan.yaml` together (atomic refusal if either already exists). Skipped under `extend`.
3. **Registry validate** — `specify registry validate`. Halts on validation failures (description-missing, kebab violations, etc.) before any brief work.
4. **Plan brief pipeline** from `capability.yaml`:
  - **(a) Discovery** — `/change:analyze` per input → `discovery.md`. May trigger greenfield-registry-bootstrap.
  - **(b) Sync workspace** (multi-repo only) — `specify workspace sync` → `workspace.md`.
  - **(b.5) Survey** [RFC-20] — DAG decomposition → `survey.md`.
  - **(b.6) Synthesise** [RFC-20] — reconciliation → `discovery.md §Reconciliation`.
  - **(c) Propose** — accept/edit/reject loop → `specify plan add` per slice.
  - **(d) Assignment** (multi-repo only) — `specify plan amend --project` per entry, with registry-proposal sub-step when an entry names a project that does not exist yet.
5. **Validate** — `specify plan validate`. Non-zero exit on any `Error`-level finding.
6. **Hand-off summary** — point the operator at `specify plan status`, `specify plan amend` for edits, and `/change:execute loop` for the next stage. Print the slice count, target projects, and any `Warning`-level validate findings the operator should be aware of before executing.

Modes: `extend` (append-only; skip step 2 and reuse discovery), `dry-run` (read-only preview; suppress every write under `.specify/`). The `orchestrate` mode does not exist on the new skill — there is nothing to orchestrate.

### `/change:execute` — what changes

Nothing. `/change:execute` keeps its existing Critical Path (resolve → lock → self-heal → next → prepare → phase → wrap-up), its three modes (supervised, `dry-run`, `loop`), and its halt classifications (`stuck`, `halted`, `driver-interrupted`, `registry-amendment-required`). The only change is documentation: it is no longer "the executor that the orchestrate mode invokes" — it is "the second peer skill in the change lifecycle."

### `/change:finalize` — what it owns

The finalize skill wraps the umbrella's post-execute tail (steps 5–7 of the current seven-step sequence) plus a guard for plan terminality. Its Critical Path:

1. **Pre-flight** — validate `<change-name>` as kebab-case; resolve project root; verify `plan.yaml` exists.
2. **Plan terminality check** — read `plan.yaml`; require every entry to be in a terminal status (`done`, `failed`, `blocked`, or `skipped`). On non-terminal entries: print which slices are still `pending` or `in-progress`, point the operator at `/change:execute loop`, and exit with `non-terminal-entries` halt.
3. **Push** — `specify workspace push`. Halt on per-project `failed`.
4. **PR observation** — `gh pr list` (read-only) for the change's branches across projects. Halt with `pr-not-merged` on any PR not in `MERGED` state, naming each open PR with its URL. The skill never merges PRs itself.
5. **Finalize** — `specify change finalize`. Halt on guard refusals (plan absent, non-terminal entries, dirty workspace, unmerged PR). Most of these should already have been caught upstream; the redundancy is intentional — `specify change finalize` is the canonical guard.
6. **Wrap-up summary** — print the merged-PR list, the archived plan path, and any post-merge tidy-ups the change captured in `change.md`.

Halts re-enter the same skill: fix the cause (run more execute, push again, merge the PR externally) and re-run `/change:finalize <name>`.

The skill is deliberately thin today. It exists as a peer of `draft` and `execute` because (a) the lifecycle reads cleaner with three named stages, (b) it gives future post-execute tidy-ups (release notes, downstream notifications, doc regeneration) a natural home without re-opening the umbrella question, and (c) it keeps `specify change finalize` (the CLI verb) doing exactly one thing — terminal-state validation and archive — rather than absorbing push and PR observation.

### Mapping today's seven-step umbrella


| Today (`/change:plan orchestrate` umbrella) | After RFC-23                                               |
| ------------------------------------------- | ---------------------------------------------------------- |
| 1 Brief (`specify change draft`)            | `/change:draft` step 2                                     |
| 2 Registry (`specify registry validate`)    | `/change:draft` step 3                                     |
| 3 Plan (`/change:plan <name>` default mode) | `/change:draft` step 4 (with steps 1, 5, 6 as scaffolding) |
| *(implicit transition)*                     | **operator review pause** — explicit, no command           |
| 4 Execute (`/change:execute loop`)          | `/change:execute loop` (unchanged)                         |
| 5 Push (`specify workspace push`)           | `/change:finalize` step 3                                  |
| 6 PR handoff (`gh pr list`)                 | `/change:finalize` step 4                                  |
| 7 Finalize (`specify change finalize`)      | `/change:finalize` step 5                                  |


Every step survives. The structural change is *where the seams are*: today there is one seam (step 7 to "done"); after this RFC there are three seams (`draft` ends at validate, `execute` ends at "no eligible slice", `finalize` ends at archived).

### Naming

**Skill identifiers**: `change-draft`, `change-execute`, `change-finalize` (matching the existing `change-analyze` pattern in SKILL.md frontmatter). Slash commands: `/change:draft`, `/change:execute`, `/change:finalize`.

The choice of `draft` is grounded in four properties:

1. **Honest about provisionality.** `plan.yaml` between authoring and execution *is* a draft — it is the thing the operator reviews, edits with `specify plan amend`, and commits to by running `/change:execute`. Naming the skill for the artefact's state at hand-off makes the seam legible. Once execute consumes the draft, it is no longer provisional in any meaningful sense; the artefact and the skill that produced it are decoupled at that point.
2. **Lifecycle pairing with `/spec`.** `/change:draft → /change:execute → /change:finalize` parallels `/spec:define → /spec:build → /spec:merge` at the change layer. Operators learn one three-stage rhythm and apply it at two scales (per-slice via `/spec:*`, per-change via `/change:*`).
3. **No collision with existing names.** `draft` is not used as a verb anywhere on the slash, CLI, or artefact surface today. It does not collide with `discovery`, `propose`, or any brief-pipeline term inside the new draft skill.
4. **Verb register.** "Draft" reads as deliberate authoring (a written artefact for review), matching the operator-facing reality that `plan.yaml` is reviewed and edited before execution. It avoids "kickoff" connotations that would be wrong for a skill that explicitly stops at the human seam.

**Why not `/change:plan**` (keep the existing name): the old skill carries the `orchestrate` mode's baggage. Operators today know `/change:plan` as both "author the plan" and "do everything." Reusing the name forces a long, awkward release where docs and operator memory have to actively unlearn the second meaning. A new name is a clean break.

**Why not `/change:initiate**` (an earlier proposal): "initiate" implies starting a process — kicking off work. The new skill is explicit about *not* kicking anything off; it stops at the operator review seam. `initiate` would actively mislead.

**Why not `/change:scope`, `/change:propose`, or `/change:brief**`: `scope` lacks lifecycle resonance; `propose` collides with the per-slice propose brief inside the draft pipeline; `brief` collides with the `change.md` artefact name and with brief-pipeline terminology.

**Why not fold finalize into `/change:execute**` (extend execute through push/PR/finalize instead of adding a third skill): execute's per-slice scope is well-understood; bolting workspace-and-change-level concerns onto it would rebuild the same double-duty problem this RFC exists to solve. A peer skill is the cleanest layout, and the three-stage rhythm matches `/spec` for free.

**Why not leave finalize as a CLI verb only** (`specify change finalize` with no skill wrapper): the operator-facing rhythm is `draft, execute, finalize` — three skills. A CLI-only third stage breaks the rhythm and forces operators to know which post-execute steps belong to which surface (`specify workspace push` separately, `gh pr list` by hand, then `specify change finalize`). Wrapping the three steps in a peer skill makes the lifecycle uniform.

### Skill shape


| Aspect              | `/change:draft`                                                           | `/change:execute`                                      | `/change:finalize`                                                            |
| ------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------ | ----------------------------------------------------------------------------- |
| Skill location      | `plugins/change/skills/draft/SKILL.md`                                    | `plugins/change/skills/execute/SKILL.md` (unchanged)   | `plugins/change/skills/finalize/SKILL.md`                                     |
| Invocation          | `/change:draft <name> [from/against/source/extend/dry-run]`               | `/change:execute [supervised                           | dry-run                                                                       |
| Owns logic?         | No (composition over CLI verbs + briefs)                                  | No (composition over `/spec:*` + CLI verbs, unchanged) | No (composition over CLI verbs)                                               |
| Owns on-disk state? | No (writes via `specify change draft` / `specify plan {add, amend}` only) | No (unchanged)                                         | No (writes via `specify workspace push` / `specify change finalize` only)     |
| Re-entry            | idempotent via `extend` mode and on-disk state                            | idempotent via self-heal (unchanged)                   | idempotent — re-reads plan and PR state on every invocation                   |
| `--dry-run`         | observation-only; runs validate; previews brief outputs                   | observation-only (unchanged)                           | observation-only; reports plan terminality, would-push branches, and PR state |


### Survey and synthesise (RFC-20 interaction)

If RFC-20 has landed by the time this RFC is implemented, `survey` and `synthesise` are sub-steps of `/change:draft`'s brief pipeline (steps 4(b.5) and 4(b.6) per RFC-20 §"Pipeline ordering"). The decomposition primitive moves from `/change:plan`'s pipeline to `/change:draft`'s pipeline; the contract is identical.

If RFC-20 has *not* landed, this RFC is unaffected — `/change:draft`'s pipeline is the four-step shape (discovery → [sync-workspace] → propose → [assignment]). RFC-23 does not depend on RFC-20.

### CLI alignment

The `specify` CLI verbs called by the three skills are:

- `/change:draft` shells out to `specify change draft`, `specify plan {add, amend, validate, status}`, `specify registry {validate, add}`, and (multi-repo) `specify workspace sync`.
- `/change:execute` is unchanged — same verbs as today.
- `/change:finalize` shells out to `specify workspace push`, `gh pr list`, and `specify change finalize`.

`specify change create` is renamed to `specify change draft` to match the skill's name and the artefact's state at hand-off. The verb's shape (positional `<change-name>`, repeated `--source` flag, atomic refusal when `change.md` or `plan.yaml` already exists) is otherwise unchanged. No other CLI verb is added, renamed, or removed. The composition lives in the skill layer.

**Text-only touch-ups land in the same release** so post-rename operators are not steered at the retired skill name `/change:plan` by Layer 1 surfaces:

1. `**specify init` hand-off** — `src/commands/init.rs` prints a "Next:" line after a successful init that names `/change:plan`. Rewrite to `/change:draft`.
2. **Domain doc-comments naming the owning skill** — `crates/domain/src/capability/capability.rs` references `/change:plan` on the `Plan` type docs, the `briefs.plan` field, and the "authoring-time step driven by `/change:plan`" line. Rewrite all four to `/change:draft`. The `Plan` type name itself stays — only the doc text changes.
3. **CLI doc-comments describing the change verbs as "the umbrella"** — `src/commands/change/cli.rs` module header and `ChangeAction` enum doc-comment. Drop "umbrella" wording; the umbrella mode is gone.
4. **CLI architecture doc** — `docs/standards/architecture.md` plugin enumeration: `/change:plan` → `/change:draft`, and append `/change:finalize` to the list.

**CLI diagnostics stay layer-clean.** Refusals like `plan-not-found`, `workspace-push-no-plan`, `non-terminal-entries-present`, and `change-finalize-blocked` continue to reference CLI verbs and on-disk paths, never slash skills. An operator pasting the diagnostic gets a runnable command in every context (agent, shell, CI). The exception is `specify init`'s hand-off line above, which is a deliberate hand-off ceremony pointing the operator at the next operator-facing surface.

The `tests/plan_orchestrate.rs` integration suite stays — it covers verbs (`change draft`, `plan validate`, end-to-end push/finalize) that survive intact. Renaming the file is optional and may follow in a separate cleanup.

## Implementation Plan

1. **Scaffold `/change:draft`.** Create `plugins/change/skills/draft/` with:
  - `SKILL.md` (orientation surface — name, description, argument-hint, Critical Path, Reference table, Guardrails). Mirror the shape of today's `/change:plan` SKILL.md.
  - `references/runbook.md` containing the verbatim Critical Path body lifted from today's `plugins/change/skills/plan/references/runbook.md`, plus the brief-scaffold and registry-validate steps lifted from today's `orchestration.md` (steps 1–2).
  - Move per-flow briefs (`discovery.md`, `sync-workspace.md`, `propose.md`, `assignment.md`) from `plugins/change/skills/plan/` into `plugins/change/skills/draft/`. Update internal cross-references.
  - Move per-capability briefs (`briefs/omnia/`, `briefs/vectis/`) from `plan/` to `draft/`.
  - Move fixtures (`fixtures/discovery/`, `fixtures/propose/`, `fixtures/multi-project/`, `fixtures/registry-proposal/`, `fixtures/dry-run/`, `fixtures/plan-multi-repo/`, `fixtures/shape-*`) from `plan/` to `draft/`. Update fixture transcripts so the invocation line reads `/change:draft <name>` instead of `/change:plan <name>`.
2. **Scaffold `/change:finalize`.** Create `plugins/change/skills/finalize/` with:
  - `SKILL.md` (Critical Path, halts, re-entry, dry-run semantics).
  - `references/runbook.md` containing the verbatim Critical Path body for steps 1–6 above.
  - Fixtures: terminal-status-not-met, push-failed, pr-not-merged, finalize-guard-refusal, happy-path. Lift the existing umbrella fixtures' tail sections (steps 5–7) where applicable.
3. **Delete `/change:plan` and its umbrella internals.** Remove `plugins/change/skills/plan/` entirely — `SKILL.md`, `orchestration.md`, `references/re-entry.md`, `references/shapes.md`, and any other plan-skill files. The change plugin has no external consumers depending on the old skill name, so the verb is dropped outright with no deprecation forward.
4. **Update tutorials.** Rewrite `docs/tutorials/cross-repo-change.md`, `docs/tutorials/landing-a-change.md`, and `docs/tutorials/cross-repo-execute.md` to use the three-skill lifecycle (`/change:draft <name>` → review → `/change:execute loop` → `/change:finalize <name>`). Remove `--orchestrate` flag references in `docs/how-to/` and `docs/explanation/` (notably `layered-stack.md` and `drop-down-a-layer.md`). Add a short doc on the operator review point — what to look at, when to use `specify plan amend`, when to abort the change.
5. **Update CLI / framework reference docs.** Update `docs/reference/change-skills/index.md`, `docs/reference/change-component.md`, `AGENTS.md`, `.cursor/rules/project.mdc`, and `.cursor-plugin/marketplace.json` to describe the three-skill lifecycle in place of `/change:plan` + the umbrella. The marketplace description currently reads "drives the cross-repo umbrella under `--orchestrate`" — rewrite to "authors `plan.yaml` (`/change:draft`) and finalizes a completed change (`/change:finalize`); execution remains `/change:execute`."
6. **Rename `specify change create` to `specify change draft` and align `specify-cli` text references.** In a coordinated PR against `augentic/specify-cli`:
  - `src/commands/change/cli.rs` — rename the `Create` `ChangeAction` enum variant and its `clap` subcommand (`specify change create` → `specify change draft`). The positional `<change-name>` and `--source <key>=<path-or-url>` flag shape are unchanged. Update the surrounding doc-comments and drop "umbrella" wording from the module header and the `ChangeAction` enum doc-comment.
  - `src/commands/change.rs` (and any helper modules) — rename internal handlers (`create_change` / `ChangeCreate*` types) to the `draft` spelling. Update diagnostic strings that name the verb (`change-create-*` → `change-draft-*` where the diagnostic embeds the verb name).
  - `tests/change_create.rs` — rename to `tests/change_draft.rs` and update invocations + fixtures accordingly.
  - `src/commands/init.rs` — rewrite the init hand-off "Next:" string: `/change:plan` → `/change:draft`.
  - `crates/domain/src/capability/capability.rs` — rewrite the four doc-comment references to `/change:plan` (`Plan` type docs, `briefs.plan` field docs, "Layer 2 authoring phase (`/change:plan`)" lines, and "authoring-time step driven by `/change:plan`").
  - `docs/standards/architecture.md` — update the plugin enumeration on the "Every deterministic operation lives in this CLI..." line: `/change:plan` → `/change:draft`; append `/change:finalize` to the list. Update any `specify change create` mentions to `specify change draft`.
  - Verify with `rg "change:plan|change:initiate|orchestrate|umbrella|change create" -i` that no stale references remain in code, tests, or docs.
  - `make checks` must stay clean.
7. **Update cross-RFC references.** RFC-20 (`rfcs/rfc-20-survey.md`) currently references `/change:initiate` and `/change:plan` interchangeably; rewrite those references to `/change:draft`. The `## Pipeline ordering` table's step numbers stay the same; only the owning skill name changes. Other RFCs that mention `/change:plan` or `/change:plan orchestrate` get the same treatment (audit during implementation).
8. **CHANGELOG.** Add an entry recording the skill rename (`/change:plan` → `/change:draft`), the CLI verb rename (`specify change create` → `specify change draft`), the new `/change:finalize` skill, and the outright removal of the `orchestrate` umbrella mode.
9. **Acceptance.** Add a regression to the cross-repo Deno acceptance suite asserting that the three-skill flow produces the same merged-PR outcome the umbrella did, given the same inputs and an automated review step (no-op `specify plan status` between draft and execute).

## Migration

This RFC is **a behavioural change**, not a pure rename. The byte-stable parts of today's flow (the brief pipeline, the per-slice loop, the push/PR/finalize tail) survive intact; the structural change is the human seam between authoring and execution and the splitting of work across three peer skills.

The change plugin has no external consumers today, so `/change:plan`, the `orchestrate` umbrella, and the `specify change create` CLI verb are removed outright in the same release — no deprecation window, no forwarding shim.

For operators:

- **Replace `/change:plan <name> [...]`** with `/change:draft <name> [...]`. Positional inputs (`from`, `against`, `source`, `extend`, `dry-run`) are unchanged.
- **Replace `specify change create <change-name> [...]`** with `specify change draft <change-name> [...]`. Flags and positional shape are unchanged.
- **Replace `/change:plan <name> orchestrate [...]`** with the three-skill flow:
  1. `/change:draft <name> [...]` — produces `plan.yaml`; stops at hand-off summary.
  2. *(review `plan.yaml`; edit with `specify plan amend` if needed; abort by deleting `change.md`, `plan.yaml`, and `.specify/plans/<change>/`)*
  3. `/change:execute loop` — runs the per-slice loop until no eligible slice remains.
  4. `/change:finalize <name>` — pushes branches, observes PR state, runs `specify change finalize` once every PR is `MERGED`. Re-run after merging any open PRs externally.
- **CI scripts that ran the umbrella** need an explicit review step (or no-op review) between draft and execute, plus the new finalize invocation. The framework does not ship a one-command equivalent; teams that want one can wrap the three skills in their own shell script, accepting that the wrapper opts out of the review pause this RFC introduces.

For skill / fixture authors:

- Briefs, fixtures, and per-capability content move from `plugins/change/skills/plan/` to `plugins/change/skills/draft/`. Update cross-references in any external docs that hard-link into the plan skill's fixture paths.
- The umbrella's `orchestration.md`, `re-entry.md`, and `shapes.md` are deleted. Anything operator-facing that referenced them now references `/change:draft`'s runbook and `/change:finalize`'s runbook.
- Internal docs that say "the orchestration mode" should say "the draft → execute → finalize lifecycle" instead.

For tutorial / how-to authors:

- All command snippets that read `/change:plan <name>` become `/change:draft <name>`.
- All command snippets that read `specify change create ...` become `specify change draft ...`.
- All command snippets that read `/change:plan <name> orchestrate ...` expand into the three-step sequence, ideally with an explicit pause in the prose for operator review.
- Layering diagrams that show `/change:plan` straddling Layer 2 authoring and Layer 2 orchestration get three boxes: `/change:draft` (authoring), `/change:execute` (per-slice driver), `/change:finalize` (post-execute close). The orchestration mode disappears from the diagram entirely.

## Alternatives Considered

**Keep the umbrella as `/change:plan orchestrate` (status quo).** Rejected. The double-duty problem and the implicit-pause problem do not get smaller as the umbrella accretes capabilities; the asymmetric vocabulary problem is permanent. The seven-step body is preserved across the three new skills — there is no behavioural debt to migrate, only structural rehoming.

**Rehome the umbrella as `/change:initiate` without removing it (the previous proposal in this RFC).** Rejected. That preserves the implicit-pause problem and leaves three change verbs (`plan`, `initiate`, `execute`) competing for operator attention. The current proposal removes the umbrella outright and matches `/spec`'s rhythm.

**Pick `/change:plan` (keep the name), `/change:initiate`, `/change:scope`, `/change:propose`, or `/change:brief` instead of `/change:draft`.** Considered. `/change:plan` carries the orchestrate baggage and cannot be cleanly recycled. `/change:initiate` implies kickoff and contradicts the explicit-stop semantics. `/change:scope` lacks lifecycle resonance. `/change:propose` collides with the per-slice propose brief inside the draft pipeline. `/change:brief` collides with the `change.md` artefact name. `/change:draft` was chosen for its honest provisionality semantics and its lifecycle pairing with `/spec`.

**Have `/change:execute` extend through push, PR observation, and finalize (no new `/change:finalize` skill).** Considered. Folding steps 5–7 into execute would give us the two-skill lifecycle `/change:draft → /change:execute(-through-finalize)`. Rejected because: (a) execute's existing scope is per-slice; bolting workspace-and-change-level concerns onto it would replicate the same double-duty problem this RFC exists to solve; (b) keeping a thin `/change:finalize` skill provides a hook for future post-execute tidy-ups (release notes, downstream notifications, doc regeneration) without re-opening the umbrella question; (c) the parity with `/spec`'s three-skill rhythm is worth a small skill.

**Leave finalize as a CLI verb only (`specify change finalize` plus the operator running `specify workspace push` and `gh pr list` by hand).** Considered. The CLI verb already exists and does the terminal-state validation; the rest could stay manual. Rejected because the operator-facing rhythm is `draft, execute, finalize` — three named stages. A CLI-only third stage breaks the rhythm and forces operators to know which post-execute steps belong to which surface. Wrapping the three steps in a peer skill makes the lifecycle uniform and gives the post-execute work a single re-entry point.

**Make `/change:draft` a pass-through wrapper around a single CLI verb that runs the whole brief pipeline.** Rejected. `specify change draft` is renamed from `specify change create` and keeps its narrow shape — it writes `change.md` and `plan.yaml` and nothing else. Folding discovery, survey, propose, and assignment into one CLI verb would duplicate the brief pipeline outside the skill surface and break the composition discipline that the rest of the change layer follows (skill = orchestrator over Layer 1 CLI verbs and per-capability briefs).

**Bundle the rehoming into RFC-20 (Survey-to-Plan Pipeline).** Rejected. RFC-20 is scoped to "Survey-to-Plan Pipeline" and the lifecycle restructuring is independent of survey — a reader of RFC-20 should not also have to consume an unrelated three-skill rename. The two RFCs land independently; if both land in the same release, the CHANGELOG combines the operator-facing notes.

**Keep a deprecation forward for `/change:plan` and `specify change create` for one release.** Rejected. The change plugin has no external consumers depending on the current names, so a forwarding shim is pure carrying cost — extra fixtures, extra docs explaining the transition, extra code on the shim's removal path. Renaming outright in a single release is cheaper and gives a single clean cut.

**Keep an opt-in single-command flow (`/change:do <name>` or `/change:plan --auto`) that runs draft + execute + finalize in sequence.** Considered. This would preserve the umbrella's convenience for teams that want it. Rejected for v1: the explicit-pause property is a design goal of this RFC, not an inconvenience to work around. Teams that want one-command flow can write a thin shell wrapper themselves; the framework not shipping one is a deliberate signal that the human seam is the recommended posture. Revisit if telemetry shows large operator demand (and if so, the wrapper is a separate RFC against the slash surface, not a re-introduction of the umbrella).

## Non-Goals

- **New behaviour inside any of the three skills.** Each is composition over existing CLI verbs and existing peer skills. New halt classifications, new recovery sequences, new pre-flight checks, new on-disk state, and forge-merge automation are all out of scope for the rename. Any such gap belongs in `/spec:`*, the underlying CLI verbs, or a follow-up RFC.
- **Changing the shape of `specify change draft` (renamed from `specify change create`) or renaming `specify change show` / `specify change finalize`.** The CLI surface otherwise stays stable; only the slash skills and the one `create` → `draft` verb rename change.
- **A one-command convenience wrapper.** Operators who want `draft + execute + finalize` in one shot write their own shell wrapper. The framework does not ship one because automatic transition between authoring and execution is the property this RFC removes.
- **Multi-plan orchestration.** RFC-3a's single `plan.yaml` invariant is preserved; the three-skill lifecycle drives one change at a time. Multi-plan output and parallel changes remain a separate concern (deferred to RFC-21 / RFC-22 territory).
- **Forge-agnostic finalize.** `/change:finalize` step 4 still uses `gh`; non-GitHub forges still fall back to the manual fallback path (merge by hand, re-run finalize). Forge abstraction is a separate concern.
- **Re-thinking the seven-step sequence.** The steps and their owners are unchanged; they are redistributed across three skills, not redesigned. If the steps need to change, that is a separate RFC.
- **Subsuming `/change:execute`'s per-slice algorithm into `/change:draft` or `/change:finalize`.** Each skill keeps its own scope.

## Open Questions

1. **Should `/change:finalize` observe PR state directly, or call `specify workspace status`?** Today's umbrella step 6 uses `gh pr list` directly. Wrapping that in a CLI verb (`specify workspace status --pr` or similar) would centralise PR-observation logic and keep the skill thinner. Current preference: defer — the skill calls `gh` directly for v1, matching the umbrella's behaviour. A CLI wrap is a separate refactor.
2. **Should `/change:finalize` emit a CHANGELOG-style summary or release notes?** The skill is thin today; a "what changed" summary derived from `plan.yaml` entries and merged PRs would be a natural post-execute tidy-up. Current preference: not in this RFC; the hook exists in step 6 ("Wrap-up summary") for a follow-up RFC to populate.
3. **Should the human seam between draft and execute be enforced** (e.g., draft writes `.specify/plans/<change>/.review-required` that execute checks for and refuses to start without an explicit `--reviewed` flag)? Current preference: no — trust the operator. The seam is a recommendation backed by the lack of a one-command wrapper, not a gate. Revisit if telemetry shows operators consistently re-invoking `specify plan amend` mid-execute (which would suggest the review point is being skipped and is doing real work).
4. **RFC filename.** This file is `rfc-23-initiate.md`, named for an earlier draft proposing `/change:initiate`. The new design proposes `/change:draft` and `/change:finalize` and removes `/change:plan`; the filename is misleading. Should it be renamed to `rfc-23-draft.md` or `rfc-23-decompose-plan.md`? Current preference: leave the filename for stability of in-repo and external references; the title accurately describes the new content. Revisit if cross-repo links to RFC-23 routinely surface confusion.

## References

- [RFC-9: Platform](archive/rfc-9-platform.md) — orchestration umbrella and shape inference; the original landing of the seven-step sequence this RFC redistributes.
- [RFC-13: Extensibility](archive/rfc-13-extensibility.md) — Layer 2 skill composition rules.
- [RFC-20: Survey-to-Plan Pipeline](rfc-20-survey.md) — peer RFC; if both land together, `/change:draft`'s step 4 internally grows survey + synthesise sub-steps without affecting the draft surface.
- `[/change:draft` SKILL.md](../plugins/change/skills/draft/SKILL.md) and `[/change:finalize` SKILL.md](../plugins/change/skills/finalize/SKILL.md) — the two new peer skills introduced by this RFC. Together with `/change:execute` they replace the old `/change:plan` skill that this RFC retires.
- `[/change:draft` runbook](../plugins/change/skills/draft/references/runbook.md) and `[/change:finalize` runbook](../plugins/change/skills/finalize/references/runbook.md) — where the body of the old seven-step orchestration sequence now lives, redistributed across the draft (steps 1–4) and finalize (steps 5–7) runbooks.
- `[/change:execute` SKILL.md](../plugins/change/skills/execute/SKILL.md) — the executor; unchanged by this RFC, but newly the explicit second stage of the change lifecycle.
- `[/spec:define](../plugins/spec/skills/define/SKILL.md)`, `[/spec:build](../plugins/spec/skills/build/SKILL.md)`, `[/spec:merge](../plugins/spec/skills/merge/SKILL.md)` — the three-skill rhythm at the spec layer that this RFC mirrors at the change layer.
- `[specify change` CLI reference](../docs/reference/cli/change.md) — `specify change draft` (called by `/change:draft`) and `specify change finalize` (called by `/change:finalize`).
- `[docs/explanation/layered-stack.md](../docs/explanation/layered-stack.md)` — where the new three-skill change lifecycle sits in Specify's layered architecture (Layer 2).
- `[docs/tutorials/cross-repo-change.md](../docs/tutorials/cross-repo-change.md)` — the canonical worked example to update with the new three-skill lifecycle.

