# Recover from `registry-amendment-required`

When `/change:execute --loop` halts on a change with the `registry-amendment-required` outcome, a phase skill (typically `/spec:extract` or a build brief) discovered that the change targets a capability that does not fit any existing registry project and proposed a new one. The framework refuses to auto-modify `registry.yaml`; the operator owns the decision.

This how-to walks the canonical recovery sequence end-to-end. RFC-9 Section 2B introduces the outcome variant and pins this exact sequence so phase skills, executors, and umbrella skills compose against it.

## Prerequisites

- A change where `/change:execute --loop` halted with `Completion: stuck` or surfaced a `registry-amendment-required` line in the per-slice summary.
- The dropped change's name (from `specify change plan status` -- the entry is in `blocked` state with `status-reason: registry-amendment-required`).

## 1. Read the proposal payload

The driver records the proposal payload (`{ proposed-name, proposed-url, proposed-schema, proposed-description, rationale }`) on the dropped change's `journal.yaml` before transitioning the entry to `blocked`. Read it via:

```bash
specify slice journal show <change>
```

Look for the most recent entry with `kind: phase-outcome` and `outcome: registry-amendment-required`. The `details` field carries the structured payload. The `rationale` is the phase skill's argument for why a new project is needed -- read it carefully; sometimes the right answer is to reject the proposal and route the change differently.

## 2. Decide whether to accept

Three legitimate decisions:

| Decision | When | Action |
|----------|------|--------|
| **Accept** | The proposal is sound -- the capability genuinely belongs in a new project. | Continue to step 3. |
| **Reject and re-route** | An existing registry project is a better fit (the phase skill's heuristic missed it). | Skip to step 4. |
| **Reject and abort** | The capability does not belong in this change at all. | `specify change plan transition <change> skipped --reason "..."` and continue with the rest of the plan. |

For "Accept," continue with the canonical sequence below. For "Reject and re-route," skip to [Re-route to an existing project](#re-route-to-an-existing-project).

## 3. Run the canonical recovery sequence (Accept path)

Four CLI verbs in this exact order. Skipping or reordering any step breaks the validator's invariants -- the plan verbs reject unknown projects, so `registry add` must precede `plan amend`.

### 3a. Add the proposed project to the registry

```bash
specify registry add <proposed-name> \
    --url <proposed-url> \
    --schema <proposed-schema> \
    --description "<proposed-description>"
```

Substitute the literal values from the journal payload. The verb runs `validate_shape` after the write, including the `description-missing-multi-repo` invariant.

### 3b. Bootstrap the workspace clone

```bash
specify workspace sync
```

`sync` is idempotent across the whole registry -- it materialises the new slot under `.specify/workspace/<proposed-name>/` and refreshes any existing slots. Greenfield URLs (remote does not yet exist) are bootstrapped via `git init` + `specify init`.

### 3c. Re-route the blocked change to the new project

```bash
specify change plan amend <change> --project <proposed-name>
```

The verb validates `<proposed-name>` against the now-updated registry (rejects unknown projects). After the write, the entry's `project:` field points at the new slot.

### 3d. Re-queue the change

```bash
specify change plan transition <change> pending
```

The entry transitions back to `pending`. The next `/change:execute` cycle picks it up and routes it into the new workspace clone.

## 4. Re-run `/change:execute`

```bash
/change:execute --loop
```

The driver picks up the re-queued change first (it has no unsatisfied dependencies that were not already satisfied before the halt), runs `/spec:define` -> `/spec:build` -> `/spec:merge` against the new workspace slot, and continues with the rest of the plan.

If you started this change via `/change:plan --orchestrate <name>`, re-running the umbrella achieves the same end state -- the umbrella's re-entry algorithm detects the now-pending change and resumes at step 4 (Execute). See [`/change:plan --orchestrate` re-entry](../reference/change-skills/change.md#re-entry--idempotency).

## Re-route to an existing project

If the right answer is an existing project rather than a new one, skip steps 3a and 3b -- you do not need to mutate the registry:

```bash
specify change plan amend <change> --project <existing-project>
specify change plan transition <change> pending
/change:execute --loop
```

The phase skill that proposed the amendment will not see the rejection; the journal entry stays in place as audit. If the same proposal recurs on the next run, that is a signal the capability is genuinely ambiguous and the phase skill's heuristic deserves an update -- file an issue.

## Verify the recovery

| Check | Command | Expect |
|-------|---------|--------|
| Registry has the new project | `specify registry show` | `<proposed-name>` listed under `projects[]`. |
| Workspace slot is materialised | `specify workspace status` | `<proposed-name>` shows `git-clone` or `symlink`, `dirty: no`. |
| Plan entry is re-routed | `specify change plan status` | `<change>` is `pending`, `project: <proposed-name>`. |
| Change drives to `done` on next run | `/change:execute --loop` | The change merges; the plan progresses. |

## Why the framework requires operator confirmation

Two reasons drive the "operator owns every registry mutation" stance:

1. **Topology is a design decision.** Registries describe how a platform is sliced into repos. Auto-creating a new repo (via `gh repo create` triggered by `registry add` plus a greenfield `workspace push`) is not a decision the framework has enough context to make on the operator's behalf.
2. **The validator is opinionated.** The `description-missing-multi-repo` invariant fires the moment a registry crosses from one project to two. Auto-adding without an operator-authored description would either generate a placeholder description (degrading the assignment step's quality) or fail the validator (creating a worse halt than the one we just recovered from).

The framework reports drift; the operator decides what to do about it. Same posture as the cross-project contract check -- see [Resolve cross-project contract warnings](resolve-cross-project-contract-warnings.md).

## See also

- [Registry amendment required](../appendices/troubleshooting.md#registry-amendment-required) -- troubleshooting entry.
- [`specify registry`](../reference/cli/registry.md) -- CLI reference for `add` and `remove`.
- [Manage registry projects](manage-registry-projects.md) -- the broader add/remove how-to.
- [`/change:execute`](../reference/change-skills/execute.md) -- the executor that surfaces the halt.
- [`/change:plan --orchestrate`](../reference/change-skills/change.md) -- the umbrella mode (formerly `/spec:initiative`), which composes against this same recovery sequence.
