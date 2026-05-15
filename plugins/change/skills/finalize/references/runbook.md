# Finalize Skill Runbook

Operational detail for `/change:finalize`. The SKILL.md keeps only the orientation surface (Critical Path + halt summary + Guardrails); everything procedural lives here.

## Overview

`/change:finalize` is the third peer skill of the change lifecycle (`draft → execute → finalize`). It owns the post-execute tail of a change — push, PR observation, and the canonical archive — plus a guard for plan terminality so the skill names the right halt before any push happens. The body of work is composition only:

| Step | CLI verb | Owner |
|---|---|---|
| Pre-flight | none (kebab-case + project-root walk + `plan.yaml` presence) | this skill |
| Plan terminality | none (reads `plan.yaml`) | this skill |
| Push | `specify workspace push` | CLI |
| PR observation | `gh pr list --head specify/<name> --state all --json number,state,merged,headRefName,url` | CLI / forge |
| Finalize | `specify change finalize` | CLI |
| Wrap-up summary | none (renders the CLI's outputs) | this skill |

The skill writes nothing to `.specify/` directly. Every state mutation is a shell-out, and PR merges stay operator-owned.

## Invocation

```text
/change:finalize <change-name> [dry-run]
```

Positional grammar:

- `<change-name>` — the change to finalize. Must match `^[a-z][a-z0-9-]*$` (kebab-case). Refusal on mismatch.
- `dry-run` — observation-only. Reports plan terminality, would-push branches, and PR state without invoking `specify workspace push` or `specify change finalize`.

## Six-step body

The six steps below are normative. Each step lists its **invocation**, the **halts** it can surface, and the **failure recovery** rule when the step exits non-zero.

### Step 1 — Pre-flight

Run **all** of the following before any side-effect:

1. **`<change-name>` validation.** Reject any value not matching `^[a-z][a-z0-9-]*$`.
2. **Project root resolution.** Walk upward from CWD for `.specify/project.yaml`. Absent → exit non-zero pointing the operator at `/spec:init`.
3. **`plan.yaml` presence.** Verify `<project-root>/plan.yaml` exists. Absent → exit non-zero with the canonical "no active change" diagnostic; the operator runs `/change:draft <change-name>` first.

These checks are deterministic and synchronous; nothing observable on disk changes if any one fails.

**Halts.** None classified — pre-flight failures are hard exits with their own diagnostic, not recoverable halt classifications.

### Step 2 — Plan terminality check

Read `plan.yaml`. For each entry, the status must be one of `done`, `failed`, `blocked`, or `skipped` (the four terminal classifications).

Any entry whose status is `pending` or `in-progress` triggers the `non-terminal-entries` halt. Print:

- the change name and per-entry status table,
- the names of the offending entries,
- the next action: run `/change:execute loop` until every entry is terminal, then re-run `/change:finalize <change-name>`.

This step is read-only; the skill never mutates the plan.

**Halts.**

- `non-terminal-entries` — at least one entry is `pending` or `in-progress`. Operator runs `/change:execute loop` and re-enters.

**Failure recovery.** Re-run `/change:execute loop` until `specify plan next` reports no eligible entry. Re-running `/change:finalize <change-name>` re-reads the plan; it is idempotent.

### Step 3 — Push

**Invocation.**

```bash
specify workspace push
```

For each project with commits on the prepared `specify/<change-name>` branch, the verb pushes that branch, creates or updates the corresponding PR, and stops. It does not create a change branch on the fly, does not push a default branch, and does not merge PRs. Greenfield remotes get `gh repo create` first when the underlying CLI reports that path.

The skill prints the per-project status table verbatim. The classifications `specify workspace push` exposes are passed through to operator output unchanged.

**Halts.**

- `failed` — at least one project's status is `failed` (auth, network, missing remote, branch protection refusal). The skill stops the run.
- `pending-checks` — `specify workspace push` reports the project as awaiting required CI / branch protection checks. Operator waits and re-enters.
- `failed-checks` — required checks failed on the pushed branch. Operator fixes the underlying failure (CI break, lint regression) and re-enters.

The skill never aborts a push for a single project's sake while others succeed — `specify workspace push` is best-effort across projects — but it does halt the **finalize** run as a whole if any project is `failed` / `pending-checks` / `failed-checks`.

**Failure recovery.** Resolve the upstream issue (push a fix, retry auth, wait for CI), then re-run `/change:finalize <change-name>`. `specify workspace push` is idempotent: clones it has already pushed are reported `up-to-date` on the next run.

### Step 4 — PR observation

**Invocation.**

```bash
gh pr list \
    --head specify/<change-name> \
    --state all \
    --json number,state,merged,headRefName,url
```

The skill calls `gh pr list` directly per project (matching the umbrella's behaviour; a CLI wrap is deferred). It then inspects whether every pushed PR is already `MERGED`.

- If any PR is open, pending, failed, closed, missing, or has the wrong head branch, the skill stops with the `pr-not-merged` halt and prints the per-project PR table including each PR's URL and state.
- If every PR is `MERGED`, the skill continues to step 5.

The skill does not merge PRs. It only observes remote PR state and waits for the operator to merge through the forge UI or their own `gh pr merge` invocation.

**Halts.**

- `pr-not-merged` — at least one PR is not `MERGED`. The diagnostic names each open PR with its URL.

**Failure recovery.** Merge the listed PRs through the forge UI or a hand-run `gh pr merge`. The skill re-reads remote PR state on every invocation and continues to step 5 only after every PR is `MERGED`.

### Step 5 — Finalize

**Invocation.**

```bash
specify change finalize
```

The verb runs four guards in order: plan-presence, plan terminal-state, per-project PR-state (`MERGED` on remote), and workspace-cleanliness (`git status --porcelain` empty). All pass → `Plan::archive` sweeps `plan.yaml`, `change.md`, and `.specify/plans/<change-name>/` into `.specify/archive/plans/<change-name>-<YYYYMMDD>/`. Any guard refuses → non-zero exit and the per-project status table is surfaced verbatim.

The skill runs `specify change finalize` only when steps 2, 3, and 4 each report success on the same invocation. Most guard refusals at step 5 should already have been caught upstream — the redundancy is intentional. `specify change finalize` is the canonical guard; the upstream checks exist so the skill can name the right halt before any push happens.

**Halts** (all surfaced verbatim with the CLI's diagnostic):

- **plan absent** — `plan-not-found`. Means the plan was already archived by a prior run; on re-entry the skill reports the change as already closed and exits zero. Not a recoverable halt.
- **non-terminal entries** — should have been caught at step 2. If the CLI raises it here, the operator returns to `/change:execute loop`.
- **dirty workspace** — `git status --porcelain` was non-empty. Operator commits or stashes the residue, then re-enters.
- **unmerged PR** — should have been caught at step 4. If the CLI raises it here, the operator merges the named PRs externally and re-enters.

**Failure recovery.** Idempotent by design. Re-running `/change:finalize <change-name>` after clearing the refused guard completes the archive on the next invocation. After a successful finalize, the verb returns `plan-not-found` (the explicit "already finalized" signal) and the skill reports the change as already closed.

### Step 6 — Wrap-up summary

After `specify change finalize` returns success, the skill prints:

- the merged-PR list (one row per project, each with its PR number and URL);
- the archived plan path (`<.specify>/archive/plans/<change-name>-<YYYYMMDD>.yaml`) and archive directory;
- any post-merge tidy-ups recorded in `change.md` (the brief's `next-steps` or equivalent free-form section, if present); the operator runs these by hand outside this skill.

The wrap-up summary is rendering only — no further CLI shell-outs.

## Halt classifications

The complete set of halt classifications this skill emits is:

| Classification | Source | Re-entry rule |
|---|---|---|
| `non-terminal-entries` | step 2 | run `/change:execute loop` until every plan entry is terminal, then re-run finalize |
| `failed` | step 3 (`specify workspace push`) | fix upstream (auth, network, missing remote), re-run finalize |
| `pending-checks` | step 3 (`specify workspace push`) | wait for the upstream check, re-run finalize |
| `failed-checks` | step 3 (`specify workspace push`) | fix the underlying failure, re-run finalize |
| `pr-not-merged` | step 4 (`gh pr list`) | merge each named PR through the forge UI or a hand-run `gh pr merge`, re-run finalize |
| finalize CLI guard refusal — plan absent | step 5 (`specify change finalize`) | already finalized; reporting only — exits zero on re-entry |
| finalize CLI guard refusal — non-terminal entries | step 5 (`specify change finalize`) | run `/change:execute loop`, re-run finalize |
| finalize CLI guard refusal — dirty workspace | step 5 (`specify change finalize`) | commit / stash the dirty residue, re-run finalize |
| finalize CLI guard refusal — unmerged PR | step 5 (`specify change finalize`) | merge the named PRs externally, re-run finalize |

Every halt is surfaced with the underlying CLI's diagnostic, byte-for-byte. The skill never paraphrases.

## Re-entry algorithm

Each halt re-enters the same skill: fix the cause, re-run `/change:finalize <change-name>`. The skill is idempotent because:

1. **It re-reads `plan.yaml` on every invocation.** Plan terminality is computed from on-disk state; nothing tracks "where finalize was last run" outside the plan itself.
2. **It re-runs `specify workspace push` on every invocation.** The verb reports `up-to-date` for clones it has already pushed; it does not double-push.
3. **It re-queries `gh pr list` on every invocation.** PR state on the forge is the authoritative source.
4. **It re-runs `specify change finalize` on every invocation.** The verb is idempotent: after a successful finalize it returns `plan-not-found` and the skill reports the change as already closed.

There is no resume token, no half-state file, no in-skill memory of where a prior run halted. The on-disk and remote state is the source of truth.

## `dry-run` semantics

Under `dry-run` the skill is observation-only end-to-end. The skill MUST NOT:

- run `specify workspace push` (step 3 is a no-op);
- merge PRs (steps 4 and 5 never invoke `gh pr merge`);
- run `specify change finalize` (step 5 is skipped entirely);
- write any file under `.specify/`.

The skill MAY:

- read `plan.yaml` (step 2 still reports terminality);
- enumerate the branches `specify workspace push` would push by reading `.specify/workspace/<project>/` and the registered project list;
- run `gh pr list` (step 4's read-only query) and report state;
- emit a final preview block summarising what each subsequent step would do.

Output shape:

```text
[dry-run] /change:finalize <change-name>

Pre-flight:    ok (plan.yaml present at <path>)
Plan:          <terminality> (<n> entries: done X, failed Y, blocked Z, skipped W, pending P, in-progress I)
Would push:    <project-1> on specify/<change-name>
               <project-2> on specify/<change-name>
PR state:
  <project-1>  PR #N  state=<state>  url=<url>
  <project-2>  PR #M  state=<state>  url=<url>
Would invoke specify change finalize (skipped under dry-run).

No changes written. Remove `dry-run` to run the full sequence.
```

A non-terminal plan or any non-`MERGED` PR appears in the preview but does not exit non-zero. `dry-run` is information-only.

## Verb hygiene

Every shell-out is listed here so reviewers can grep for accidental drift:

| Step | Verb |
|---|---|
| Pre-flight | none (skill-internal validation) |
| Plan terminality | none (reads `plan.yaml`) |
| Push | `specify workspace push` |
| PR observation | `gh pr list --head specify/<change-name> --state all --json number,state,merged,headRefName,url` |
| Finalize | `specify change finalize` |

This skill must not introduce any other shell-out. Any temptation to add a flag, a sub-verb, or a side-effect is a sign the work belongs in `/change:execute`, in one of the underlying CLI verbs, or in a future RFC.

## Composition discipline

This skill adds **no new logic**. Every step is a documented shell-out to either:

- a `specify` CLI verb listed above; or
- the `gh` CLI for read-only PR observation.

Concretely, this skill MUST NOT:

- introduce a new CLI verb;
- modify any file under `.specify/` directly (every write is a shell-out);
- re-implement halt classification (the underlying verbs own it);
- merge PRs (the operator owns merges);
- swallow halts (every halt is surfaced verbatim with the underlying verb's diagnostic).

If a behaviour drift surfaces between this skill and a manual run of the same three verbs, the bug is in the underlying verb — not in this skill. File the gap against the underlying surface; the skill stays composition-only.

## Non-goals

- **Auto-merging PRs.** `gh pr merge` is operator-owned. The skill observes PR state and waits.
- **Forge-agnostic PR observation.** Step 4 uses `gh`; non-GitHub forges fall back to the manual fallback path (merge by hand, re-run finalize).
- **CHANGELOG generation or release notes synthesis.** Step 6 prints what the underlying CLIs and `change.md` already record. Synthesis is a separate concern.
- **Multi-plan finalize.** The single-`plan.yaml` invariant is preserved; the skill drives one change at a time.
- **Driving completed changes.** Once `specify change finalize` returns `plan-not-found`, re-running the skill reports the change as already finalized and exits zero. There is no "rewind" verb.
- **Per-slice work.** The skill never invokes `/spec:define`, `/spec:build`, or `/spec:merge`. Per-slice mutation is `/change:execute`'s concern.

## Cross-links

- [`SKILL.md`](../SKILL.md) — orientation surface for this skill.
- [`../execute/SKILL.md`](../../execute/SKILL.md) — peer driver skill that produces the terminal `plan.yaml` this skill consumes.
- [`specify workspace`](../../../../../docs/reference/cli/workspace.md) — the `push` verb invoked at step 3.
- [`specify change`](../../../../../docs/reference/cli/change.md) — the `finalize` verb invoked at step 5.
