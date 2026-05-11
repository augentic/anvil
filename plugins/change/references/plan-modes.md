# Plan modes reference

Per-mode deltas from the `/change:plan` core five-step loop. The SKILL.md body keeps the at-a-glance table; this reference carries the contract for each mode.

| Mode | One-line contract |
|---|---|
| Default (no mode positional) | Run the five-step loop unchanged. |
| `extend` | Append to an existing plan; skip step 2; reuse discovery; collisions silently skipped. |
| `dry-run` | Read-only preview; suppress every write under `.specify/`. |
| `orchestrate` | Default authoring loop, then the cross-repo umbrella sequence. |

## Default (no mode positional)

Run the five-step loop exactly as written. `plan.yaml` is initialised via step 2, populated via step 3(c), validated in step 4. A pre-existing `plan.yaml` is refused at step 1 (the operator is pointed at `specify change plan archive`).

## `extend`

Add to an existing `plan.yaml` instead of refusing. The skill-level contract is:

- **Step 1 refuses when `plan.yaml` is absent.** `extend` is an explicit "I know there's a plan here" signal; the skill never silently creates a fresh plan under `extend`.
- **Step 2 (`specify change plan create`) is skipped entirely.**
- **Step 3(a) is skipped when `.specify/plans/<change-name>/discovery.md` already exists**, with a log line `Discovery already present; reusing existing inventory.` Discovery is explicitly a one-shot artefact; an operator who wants to refresh it archives the plan and re-runs without `extend`. When `discovery.md` does not yet exist under `extend` (e.g. a plan authored by hand, or an earlier run aborted), step 3(a) runs normally.
- **Step 3(c) skips collisions silently.** Draft slices whose proposed `name` collides with an existing plan entry are recorded in `proposal.md` with decision `skip-existing` and the existing entry's name in the "Plan entry" column; the human is not re-prompted. Slices whose names do not collide run through the usual accept / edit / reject / abort loop.
- **Sync-peers (step 3(b)):** when the registry declares more than one project, **do not** shell `specify workspace sync`. Still regenerate `.specify/plans/<change-name>/workspace.md` from the existing `.specify/workspace/` cache (read-only walk) so propose stays deterministic without an implicit `git fetch`.
- **Pre-existing entries are never modified.** The skill never calls `specify change plan transition` on existing entries. The only `specify change plan amend` call is step 3(d) Assignment (`--project`), which tags newly created entries — it does not modify pre-existing ones.

No new positional is introduced beyond `extend`. A future change may add `force-discovery` if refreshing the inventory mid-plan becomes a real need.

## `dry-run`

Emit a readiness report, the would-be-produced capability inventory, and the would-be-proposed plan to stdout; write nothing. Dry-run folds the readiness gate, the discovery preview, and the propose preview into a single pass.

Under `dry-run` the skill MUST NOT:

- create `.specify/plans/<change-name>/`;
- shell out to `specify change plan create`, `specify change plan add`, `specify change plan amend`, or `specify change plan transition`;
- shell out to **`specify workspace sync`** or write **`.specify/plans/<change-name>/workspace.md`** (sync-peers dry-run rule);
- write any file under `.specify/` (including under `.specify/workspace/`).

The discovery brief's input-reading side (reading `from` files, invoking `/spec:analyze` against `source` / `against` inputs) runs under `dry-run` so the preview inventory is real; only the write to `discovery.md` and the `.specify/plans/<name>/` directory creation are suppressed. The propose brief's slice-decomposition pass also runs (the preview plan shape is real against the previewed inventory); the accept / edit / reject loop and every `specify change plan add` call are skipped.

The full output shape (banner / sources block / pipeline line / capability inventory preview / would-be-proposed plan / assignment preview) is pinned by `fixtures/dry-run/expected-output.md`, `fixtures/discovery/expected-discovery.md`, and `fixtures/propose/expected-proposal.md`. The `[dry-run]` banner on the first line is enough — body lines do not need a per-line prefix.

## `orchestrate`

Run the cross-repo umbrella sequence after the authoring loop completes. The orchestration mode is composition only — every step shells out to a verb that already exists in the v1 surface. It does not own the human PR merge; it only opens/updates PRs via `specify workspace push`, observes whether they have been merged, and invokes `specify change finalize` once the remote PR state is ready. See [orchestration.md](../skills/plan/orchestration.md) for the full sequence, halts table, manual fallbacks, and verb hygiene; [shapes.md](../skills/plan/shapes.md) for shape inference and validation; [re-entry.md](../skills/plan/re-entry.md) for the idempotent resume algorithm.

Under `orchestrate dry-run`, the umbrella is observation-only end-to-end: the authoring loop runs in dry-run (per the §`dry-run` section above), and the execution, push, PR-observation, and finalize portions emit "would invoke" preview lines without invoking any phase skill, push, forge merge, or finalize. See [orchestration.md](../skills/plan/orchestration.md) §"`dry-run` semantics".
