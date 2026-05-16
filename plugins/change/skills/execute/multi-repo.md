# Multi-repo routing and compatibility reporting

For multi-repo changes the driver keeps the coordinator repo as the owner of `plan.yaml`, `.specify/plan.lock`, and terminal status transitions. For each plan entry with `project`, it resolves and prepares that project's materialised workspace slot, `chdir`s into the slot only for phase execution, then restores CWD before writing the terminal plan transition. After a successful merge, the driver commits any non-baseline residue before it can mark the entry `done`. Cross-project consumer-impact reporting is a separate `specify compatibility` CLI surface. Shared plan/outcome/journal ownership rules live in [execute-state-handoff.md](../../references/execute-state-handoff.md).

These clones are the read-write **tier-2** workspace; they outlive the change and are pushed to remotes by `specify workspace push`. The read-only **tier-1** legacy-source clones used by `/change:analyze` at plan time are a separate concern entirely. See [Workspace Tiers](../../../../docs/explanation/workspace-tiers.md) for the full contrast.

## Workspace routing and branch preparation (per-slice algorithm step 5a)

Read `project` from the `specify plan next` response (step 4 of the per-slice algorithm). If `project` is non-null:

- Resolve the target project through `registry.yaml` using the same selector preflight as `specify workspace *`. Unknown names halt before filesystem, Git, forge, phase, or plan-status side effects.
- Save CWD (the initiating repo root).
- Resolve every key in the entry's `sources` list to an absolute filesystem path anchored to the initiating repo root. Git URLs pass through unchanged. These resolved paths are reused for `/spec:define` and for branch-preparation dirty-work classification.
- Check workspace state via `specify workspace status <project> --format json`. If the selected slot is `missing`, run `specify workspace sync <project>` and re-check only that project. Do **not** run broad `specify workspace sync` from `/change:execute`; selected execution materialises only the current plan entry's project unless the operator chose broader sync elsewhere.
- For mismatched materialisation (`other`, wrong origin, wrong symlink target, missing `.specify/project.yaml`, etc.), halt with the status diagnostic. Release the lock and exit non-zero; do not transition the plan entry.
- Prepare the worktree before any phase writes:
  ```bash
  specify workspace prepare-branch <project> \
      --change <change-name> \
      [source <absolute-source-path> ...] \
      [output <capability-owned-output-path> ...] \
      --format json
  ```
  The target branch is exactly `specify/<change-name>`. The helper fetches the remote-backed slot, resolves `origin/HEAD`, creates or reuses the local change branch, fast-forwards from `origin/specify/<change-name>` when appropriate, and classifies dirty work against the active slice boundary.
- On `prepared: true`, remember the returned `slot_path` and branch. After the plan entry transitions to `in-progress`, `chdir` into that prepared project root.
- Emit diagnostic: `Routing: <name> → <project> (<resolved-path>)`

If `project` is null, skip this step entirely (single-repo path).

### Branch-preparation failures

`workspace prepare-branch` failures are pre-phase failures, not phase outcomes. They never call `/spec:drop`, never write `.metadata.yaml:outcome`, and never transition the entry to `failed` or `blocked` automatically.

Stable diagnostic keys from the helper include:

| Key | Driver behaviour |
|---|---|
| `workspace-slot-missing` | Run the selected `specify workspace sync <project>` once, then retry status / branch preparation. If it is still missing, halt. |
| `origin-head-unresolved` | Halt before phase writes. Do not guess a default branch. |
| `dirty-unrelated-tracked` | Halt before checkout. Surface the blocked paths. |
| `dirty-branch-mismatch` | Halt before checkout. Resume-safe tracked work is allowed only when already on `specify/<change-name>`. |
| `origin-mismatch`, `workspace-slot-not-git`, `branch-pattern-mismatch`, `git-operation-failed` | Halt with the helper's diagnostic payload. |

When the branch-preparation failure occurs before the slice directory exists, there is no slice journal to write yet; the terminal output is the audit trail. When it occurs on a self-heal resume and `.specify/slices/<name>/journal.yaml` exists in the project slot, append a `failure` entry with summary `branch-preparation-failed: <diagnostic-key>` and the helper's JSON diagnostic in `--context`, then halt. In both cases the coordinator lock is released and the plan entry remains `pending` (fresh run) or `in-progress` (resume).

## Post-merge residue commit (per-slice algorithm step 9a)

For a routed project entry, `/spec:merge` success is not enough to mark the entry `done`. RFC-14 splits commit ownership:

1. `specify slice merge run` owns the merge-baseline commit and commits only `.specify/specs/` plus `.specify/archive/` with message `specify: merge <slice-name>`.
2. `/change:execute` owns any remaining project-output residue produced by define/build/merge, such as `crates/`, `contracts/`, `apps/`, generated tests, or other capability-owned files.

Immediately after reading `outcome: success` from `/spec:merge`, while still `chdir`ed into the project slot:

1. Check `.specify/specs/` and `.specify/archive/` for dirty tracked or untracked paths. If either tree is dirty, halt with diagnostic key `baseline-residue-after-merge`. Do not create a residue commit and do not transition the plan entry to `done`; the baseline commit boundary failed and requires operator triage.
2. Check the rest of the worktree, excluding `.specify/specs/` and `.specify/archive/`. If it is clean, emit `Residue: clean; no commit.` and continue.
3. If non-baseline residue exists, stage and commit only that residue:
   ```bash
   git add --all -- . ':!.specify/specs/**' ':!.specify/archive/**'
   git commit -m "specify: residue <slice-name>"
   ```
   On success, emit `Residue committed: specify: residue <slice-name>`.
4. If staging or committing fails, halt with diagnostic key `residue-commit-failed`. Leave the plan entry `in-progress`, release the lock, and tell the operator to inspect `git status` in the project slot. A later `/change:execute` run must pass self-heal's residue guard before it can transition the entry to `done`.

## CWD restore (per-slice algorithm step 9b)

If the CWD routing step (5c) changed the working directory, restore CWD to the saved initiating repo root. This ensures `specify plan transition` (which reads `plan.yaml` in the initiating repo) runs from the correct directory. In `--loop` mode, the CWD routing and CWD restore steps bracket every iteration so that `specify plan next` always runs from the initiating repo root.

## Cross-project compatibility reporting (RM-04)

`/change:execute` does not run consumer-impact classification as a side-effect of step 10. Operators can run the CLI-owned report explicitly:

```bash
specify compatibility check --change <change-name> --report-only   # read-only RM-04 report, always exits 0
specify compatibility check                                         # strict gate, exits 2 on non-additive findings
```

The report walks `registry.yaml`, matches `contracts.produces` to `contracts.consumes`, compares root `contracts/<path>` to `.specify/workspace/<consumer>/contracts/<path>`, and classifies each comparable delta as `additive`, `breaking`, `ambiguous`, or `unverifiable`. The bare `compatibility check` exits validation-failed when any finding is not additive; `--report-only` suppresses that exit code for audit/CI workflows. RM-11 will decide how those classifications become plan lifecycle gates.
