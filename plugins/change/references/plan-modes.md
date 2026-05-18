# Plan modes reference

Per-mode deltas from the `/change:draft` core six-step loop. The SKILL.md body keeps the at-a-glance table; this reference carries the contract for each mode.

| Mode | One-line contract |
|---|---|
| Default (no mode positional) | Run the loop unchanged. |
| `extend` | Append to an existing plan; skip step 2; reuse discovery; collisions silently skipped. |
| `dry-run` | Read-only preview; suppress every write under `.specify/`. |

## Default (no mode positional)

Run the loop exactly as written. `plan.yaml` is initialised via step 2, populated via step 4(c), validated in step 5. A pre-existing `plan.yaml` is refused at step 1 (the operator is pointed at `specify plan archive`).

## `extend`

Add to an existing `plan.yaml` instead of refusing. The skill-level contract is:

- **Step 1 refuses when `plan.yaml` is absent.** `extend` is an explicit "I know there's a plan here" signal; the skill never silently creates a fresh plan under `extend`.
- **Step 2 (`specify change draft` — the merged brief + plan scaffold) is skipped entirely.**
- **Step 4(a) is skipped when `.specify/plans/<change-name>/discovery.md` already exists**, with a log line `Discovery already present; reusing existing inventory.` Discovery is explicitly a one-shot artefact; an operator who wants to refresh it archives the plan and re-runs without `extend`. When `discovery.md` does not yet exist under `extend` (e.g. a plan authored by hand, or an earlier run aborted), step 4(a) runs normally.
- **Step 4(c) skips collisions silently.** Draft slices whose proposed `name` collides with an existing plan entry are recorded in `proposal.md` with decision `skip-existing` and the existing entry's name in the "Plan entry" column; the human is not re-prompted. Slices whose names do not collide run through the usual accept / edit / reject / abort loop.
- **Sync-workspace (step 4(b)):** when the registry declares more than one project, **do not** shell `specify workspace sync`. Still regenerate `.specify/plans/<change-name>/workspace.md` from the existing `.specify/workspace/` cache (read-only walk) so propose stays deterministic without an implicit `git fetch`.
- **Pre-existing entries are never modified.** The skill never calls `specify plan transition` on existing entries. The only `specify plan amend` call is step 4(d) Assignment (`--project`), which tags newly created entries — it does not modify pre-existing ones.

No new positional is introduced beyond `extend`. A future change may add `force-discovery` if refreshing the inventory mid-plan becomes a real need.

## `dry-run`

Emit a readiness report, the would-be-produced adapter inventory, and the would-be-proposed plan to stdout; write nothing. Dry-run folds the readiness gate, the discovery preview, and the propose preview into a single pass.

Under `dry-run` the skill MUST NOT:

- create `.specify/plans/<change-name>/`;
- shell out to `specify change draft`, `specify plan add`, `specify plan amend`, or `specify plan transition`;
- shell out to **`specify workspace sync`** or write **`.specify/plans/<change-name>/workspace.md`** (sync-workspace dry-run rule);
- write any file under `.specify/` (including under `.specify/workspace/`).

The discovery brief's input-reading side (reading `from` files, invoking `/change:analyze` against `source` / `against` inputs) runs under `dry-run` so the preview inventory is real; only the write to `discovery.md` and the `.specify/plans/<name>/` directory creation are suppressed. The propose brief's slice-decomposition pass also runs (the preview plan shape is real against the previewed inventory); the accept / edit / reject loop and every `specify plan add` call are skipped.

The full output shape (banner / sources block / pipeline line / adapter inventory preview / would-be-proposed plan / assignment preview) is pinned by `fixtures/dry-run/expected-output.md`, `fixtures/discovery/expected-discovery.md`, and `fixtures/propose/expected-proposal.md`. The `[dry-run]` banner on the first line is enough — body lines do not need a per-line prefix.

There is no `orchestrate` mode on `/change:draft`. The cross-repo execution sequence that the old umbrella owned is now the three-skill lifecycle `/change:draft <name>` → operator review → `/change:execute loop` → `/change:finalize <name>`. The pre-execute steps live in [`../skills/draft/references/runbook.md`](../skills/draft/references/runbook.md); the post-execute push / PR-observation / finalize tail lives in [`../skills/finalize/references/runbook.md`](../skills/finalize/references/runbook.md).
