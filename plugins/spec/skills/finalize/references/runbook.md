# Finalize skill runbook

Operational detail for `/spec:finalize`. The SKILL.md keeps only the orientation surface, critical path, closing message, and links; everything procedural lives here.

## Invocation

```text
/spec:finalize <name>
```

Positional grammar:

- `<name>` — the plan / change to finalize. Must match `^[a-z][a-z0-9-]*$` (kebab-case). Refusal on mismatch.

## Five-step body

The five steps below are normative. Each step lists its **invocation**, the **halts** it can surface, and the **failure recovery** rule when the step exits non-zero.

### Step 1 — Pre-flight

Run **all** of the following before any side-effect:

1. **`<name>` validation.** Reject any value not matching `^[a-z][a-z0-9-]*$`.
2. **Project root resolution.** Walk upward from CWD for `.specify/project.yaml`. Absent → exit non-zero pointing the operator at `/spec:init`.
3. **`plan.yaml` presence.** Verify `<project-root>/plan.yaml` exists. Absent → exit non-zero with the canonical "no active change" diagnostic; the operator runs `/spec:plan <name>` first.

These checks are deterministic and synchronous; nothing observable on disk changes if any one fails.

**Halts.** None classified — pre-flight failures are hard exits with their own diagnostic, not recoverable halt classifications.

### Step 2 — Drained check via `specify plan next`

**Invocation.**

```bash
specify plan next --format json
```

The plan is drained when the envelope reports `reason: drained` (with `active: null` and `next: null`). All other reasons (`in-progress`, `stuck`, or a queued `next: <name>`) mean at least one entry is still non-`done`. The skill never reads `plan.yaml` directly; drainage is computed by the CLI from the per-entry `status` set.

This step is read-only.

**Halts.**

- `non-terminal-entries` — `specify plan next` returns anything other than `reason: drained`. The diagnostic names the offending entry (`active` or `next` from the envelope). Operator runs `/spec:execute` to drive the loop forward, then re-enters.

**Failure recovery.** Re-run `/spec:execute` until `specify plan next` reports `drained`. Re-running `/spec:finalize <name>` re-queries the plan; it is idempotent.

### Step 3 — Push

**Invocation.**

```bash
specify workspace push
```

For each project on the plan, the verb pushes the prepared `specify/<name>` branch, creates or updates the corresponding PR, and stops. It does not create a change branch on the fly, does not push a default branch, and does not merge PRs. Greenfield remotes get `gh repo create` first when the underlying CLI reports that path. Single-repo plans are the degenerate case (one project on the table); workspace plans drive every project the plan touches in one invocation.

The skill prints the per-project status table verbatim. The classifications `specify workspace push` exposes are passed through to operator output unchanged.

**Halts.**

- `failed` — at least one project's status is `failed` (auth, network, missing remote, branch protection refusal). The skill stops the run.
- `pending-checks` — `specify workspace push` reports the project as awaiting required CI / branch protection checks. Operator waits and re-enters.
- `failed-checks` — required checks failed on the pushed branch. Operator fixes the underlying failure (CI break, lint regression) and re-enters.

The skill never aborts a push for a single project's sake while others succeed — `specify workspace push` is best-effort across projects — but it does halt the **finalize** run as a whole if any project is `failed` / `pending-checks` / `failed-checks`.

**Failure recovery.** Resolve the upstream issue (push a fix, retry auth, wait for CI), then re-run `/spec:finalize <name>`. `specify workspace push` is idempotent: clones it has already pushed are reported `up-to-date` on the next run.

### Step 4 — PR observation loop

**Invocation (per pushed project).**

```bash
gh pr view "$PR_URL" --json state,url,number
```

The skill calls `gh pr view` directly per pushed PR (the URL is taken from the `specify workspace push` output). It then polls each non-`MERGED` PR until every PR reports `MERGED`.

**Polling parameters.**

- **Interval** — 30s between polls per PR (default; configurable per invocation by the operator).
- **Ceiling** — 1h per run (default). The skill resets the clock on each fresh invocation, so the operator can simply re-run `/spec:finalize` after merging.
- **Progress** — emit one line per polling cycle naming the still-`OPEN` PR set, so the transcript remains scannable during long waits.

**State branching.**

- Any PR `OPEN` → wait the interval, re-poll. Continue until every PR is `MERGED` or the ceiling fires.
- Ceiling exhaustion → halt with `pr-poll-exhausted`; name every still-`OPEN` PR with its URL.
- Any PR `CLOSED` (not merged) → halt with `pr-closed`; name every closed PR with its URL.
- Every PR `MERGED` → continue to step 5.

The skill never invokes `gh pr merge`. It only observes remote PR state and waits for the operator to merge through the forge UI or their own `gh pr merge` invocation.

**Halts.**

- `pr-poll-exhausted` — the polling ceiling fired with at least one PR still `OPEN`. Operator merges through the forge UI or a hand-run `gh pr merge`, then re-runs.
- `pr-closed` — at least one PR is `CLOSED` without being merged. Operator reopens, amends the plan, or otherwise resolves the closure, then re-runs.

**Failure recovery.** Merge or reopen the listed PRs externally. The skill re-queries every PR on every invocation and continues to step 5 only after every PR is `MERGED`.

### Step 5 — Finalize

**Invocation.**

```bash
specify plan archive
```

The verb runs archive preflight only: active plan presence, no outstanding non-`done` entries unless `--force` is set, and destination collision checks. All pass → the verb sweeps `plan.yaml` to `.specify/archive/plans/<name>-<YYYYMMDD>.yaml`, and co-moves `change.md` and any `.specify/plans/<name>/` working directory under `.specify/archive/plans/<name>-<YYYYMMDD>/` alongside it.

The skill runs `specify plan archive` only when steps 2, 3, and 4 each report success on the same invocation. PR state belongs to step 4's `gh pr view` loop; archive preflight does not contact the forge or inspect workspace cleanliness.

**Halts** (all surfaced verbatim with the CLI's diagnostic):

- **plan absent** — no active `plan.yaml`. Treat as already closed only after confirming the expected archive path or prior transcript; otherwise report that there is no active change.
- **non-terminal entries** — should have been caught at step 2. If the CLI raises it here, the operator returns to `/spec:execute`.
- **archive target exists** — the dated archive destination already exists. Operator inspects or moves the existing archive, then re-enters.

**Failure recovery.** Re-running `/spec:finalize <name>` after clearing the refused archive preflight completes the archive on the next invocation. After a successful finalize, there is no active `plan.yaml`; the skill reports the change as already closed only when it can confirm the archive path or prior successful transcript.

### Step 6 — Wrap-up summary

After `specify plan archive` returns success, the skill prints:

- the merged-PR list (one row per project, each with its PR number and URL);
- the archived plan path (`<.specify>/archive/plans/<name>-<YYYYMMDD>.yaml`) and archive directory;
- any post-merge tidy-ups recorded in `change.md` (the brief's `next-steps` or equivalent free-form section, if present); the operator runs these by hand outside this skill;
- the canonical closing line, byte-for-byte:

  ```text
  Change <name> finalized. Plan archived at <.specify>/archive/plans/<name>-<YYYYMMDD>.yaml.
  ```

The wrap-up summary is rendering only — no further CLI shell-outs.

## Halt classifications

The complete set of halt classifications this skill emits is:

| Classification | Source | Re-entry rule |
|---|---|---|
| `non-terminal-entries` | step 2 (`specify plan next`) | run `/spec:execute` until the plan reports `drained`, then re-run finalize |
| `failed` | step 3 (`specify workspace push`) | fix upstream (auth, network, missing remote), re-run finalize |
| `pending-checks` | step 3 (`specify workspace push`) | wait for the upstream check, re-run finalize |
| `failed-checks` | step 3 (`specify workspace push`) | fix the underlying failure, re-run finalize |
| `pr-closed` | step 4 (`gh pr view`) | reopen the PR or amend the plan, re-run finalize |
| `pr-poll-exhausted` | step 4 (`gh pr view`) | merge each named PR through the forge UI or a hand-run `gh pr merge`, re-run finalize |
| finalize CLI guard refusal — plan absent | step 5 (`specify plan archive`) | confirm archive path or prior transcript before treating as already finalized |
| finalize CLI guard refusal — non-terminal entries | step 5 (`specify plan archive`) | run `/spec:execute`, re-run finalize |
| finalize CLI guard refusal — archive target exists | step 5 (`specify plan archive`) | inspect or move the existing archive, re-run finalize |

Every halt is surfaced with the underlying CLI's diagnostic, byte-for-byte. The skill never paraphrases.

## Re-entry algorithm

Each halt re-enters the same skill: fix the cause, re-run `/spec:finalize <name>`. The skill is idempotent because:

1. **It re-runs `specify plan next` on every invocation.** Drainage is computed from on-disk state; nothing tracks "where finalize was last run" outside the plan itself.
2. **It re-runs `specify workspace push` on every invocation.** The verb reports `up-to-date` for clones it has already pushed; it does not double-push.
3. **It re-queries `gh pr view` on every invocation.** PR state on the forge is the authoritative source.
4. **It re-runs archive confirmation on every invocation.** If no active plan remains, the skill confirms the archive path or prior successful transcript before reporting the change as already closed.

There is no resume token, no half-state file, no in-skill memory of where a prior run halted. The on-disk and remote state is the source of truth.

## Verb hygiene

Every shell-out is listed here so reviewers can grep for accidental drift:

| Step | Verb |
|---|---|
| Pre-flight | none (skill-internal validation) |
| Drained check | `specify plan next --format json` |
| Push | `specify workspace push` |
| PR observation | `gh pr view "$PR_URL" --json state,url,number` |
| Finalize | `specify plan archive` |

This skill must not introduce any other shell-out. Any temptation to add a flag, a sub-verb, or a side-effect is a sign the work belongs in `/spec:execute`, in one of the underlying CLI verbs, or in a future RFC.

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

If a behaviour drift surfaces between this skill and a manual run of the same four verbs, the bug is in the underlying verb — not in this skill. File the gap against the underlying surface; the skill stays composition-only.

## Non-goals

- **Auto-merging PRs.** `gh pr merge` is operator-owned. The skill observes PR state and waits.
- **Forge-agnostic PR observation.** Step 4 uses `gh`; non-GitHub forges fall back to the manual fallback path (merge by hand, re-run finalize).
- **CHANGELOG generation or release notes synthesis.** Step 6 prints what the underlying CLIs and `change.md` already record. Synthesis is a separate concern.
- **Multi-plan finalization.** The single-`plan.yaml` invariant is preserved; the skill drives one change at a time.
- **Driving completed changes.** Once the active plan has been archived, re-running the skill reports the change as already finalized only after confirming the archive path or prior successful transcript. There is no "rewind" verb.
- **Per-slice work.** The skill never invokes `/spec:refine`, `/spec:build`, or `/spec:merge`. Per-slice mutation is `/spec:execute`'s concern.

## Cross-links

- [`../SKILL.md`](../SKILL.md) — orientation surface for this skill.
- [`../../execute/SKILL.md`](../../execute/SKILL.md) — peer driver skill that drains the plan this skill closes.
