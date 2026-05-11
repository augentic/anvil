# Change landing issues

Use this page when `specify workspace push`, `specify change finalize`, or `specify compatibility check` refuses while you are trying to publish or close out a change.

## Prerequisites

- A change that has finished executing locally and is now being pushed, finalized, or compatibility-checked.
- The CLI output (the project name, the status code, and the failing branch or PR identifier).

## `no-branch` from `workspace push`

**Symptom:** `specify workspace push <project>` reports `no-branch`.

**Cause:** The slot is not currently on exact `specify/<change-name>`, or the expected change branch resolves to the remote default branch. `workspace push` is transport-only: it does not create or check out the change branch, and it never pushes `main`, `master`, or any default branch.

**Resolution:**
1. Check the branch and match state: `specify workspace status <project>`.
2. If execution has not run for this project, run `/change:execute` so branch preparation creates or reuses `specify/<change-name>` before mutation.
3. If you are recovering by hand, check out the exact `specify/<change-name>` branch in the slot and ensure it contains the intended commits.
4. Re-run `specify workspace push <project>`.

## Dirty slot from `workspace push` or `change finalize`

**Symptom:** `specify workspace push` reports status `failed` with a dirty-checkout message, or `specify change finalize` reports status `dirty`.

**Cause:** The workspace slot has uncommitted work. Push refuses dirty slots because it only transports committed state. Finalize refuses dirty slots, even without `--clean`, so no local work is lost during archive or cleanup.

**Resolution:**
1. Inspect the slot: `specify workspace status <project>`.
2. Commit and push intended work on `specify/<change-name>`, or stash/remove unrelated local edits.
3. Re-run `specify workspace push <project>` if the PR still needs publication.
4. After the PR is merged, re-run `specify change finalize`.

## `unmerged` from `change finalize`

**Symptom:** `specify change finalize` refuses with status `unmerged` for one or more projects.

**Cause:** A PR exists on `specify/<change-name>` but is still open. Finalize is read-only with respect to forges; it verifies that the operator already landed the PR and never invokes a merge API.

**Resolution:**
1. Open the PR shown in the finalize output.
2. Merge it through the forge UI, `gh pr merge`, or the repository's normal merge queue.
3. Re-run `specify change finalize` after the forge reports the PR as merged.

## `branch-pattern-mismatch`

**Symptom:** branch preparation or `specify change finalize` refuses on a project with status `branch-pattern-mismatch`.

**Cause:** The change branch or PR head is not exactly `specify/<change-name>`. The guard exists so Specify never prepares, publishes, or finalizes an unintended branch.

**Resolution:** Inspect the branch or PR by hand (`gh pr view <pr> -R <org/repo>`). If the PR is correct, recreate or rename it so the head branch is exactly `specify/<change-name>`. If it was created outside the Specify flow, close it, publish the exact change branch with `specify workspace push`, merge it through the forge, and re-run `specify change finalize`. The guard is never overridden.

## `plan-not-found` from `change finalize`

**Symptom:** `specify change finalize` exits non-zero with `plan-not-found`.

**Cause:** `plan.yaml` does not exist. This is the explicit "already finalized" signal -- a previous successful `finalize` run swept the plan into `.specify/archive/plans/<YYYYMMDD>-<name>/`.

**Resolution:** None needed -- the change is already closed. Inspect the archive to confirm: `ls .specify/archive/plans/`. If the plan was lost some other way (e.g. accidental `rm`), recover from version control.

## Breaking findings from `specify compatibility check`

**Symptom:** `specify compatibility check` exits validation-failed and reports `breaking`, `ambiguous`, or `unverifiable` findings.

**Cause:** Compatibility classification found producer-to-consumer contract risk, or it could not compare the current producer contract with a consumer workspace view.

**Resolution:** Read [Resolve cross-project compatibility findings](../resolve-cross-project-contract-warnings.md) for the triage checklist. Typical paths: spawn a follow-up consumer slice to track the producer's update, refresh the workspace clone if the finding is unverifiable, or accept the drift if the consumer is intentionally lagging.
