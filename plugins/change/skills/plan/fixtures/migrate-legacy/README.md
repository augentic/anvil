# `--shape migrate-legacy` — `migrate-foo` end-to-end

This fixture pins the **happy path** of `/change:plan <name> orchestrate` driving the `migrate-legacy` shape against an empty platform hub. Sources arrive via `source <key>=<git-url>`; the registry is empty, so discovery proposes the initial topology. The umbrella opens PRs, stops for operator merge, then finalizes on re-entry after `specify change finalize` verifies the remote PR state.

## Scenario

The operator wants to migrate the legacy `mono-repo-foo` TypeScript service onto Augentic's Omnia + Vectis stack. They start in a fresh hub repo (`shop-platform/`) created earlier with `specify init --hub`.

```text
/change:plan <name> orchestrate migrate-foo \
    shape migrate-legacy \
    source monolith=git@github.com:org/legacy-foo.git
```

The umbrella runs through PR handoff, then resumes after operator merge:

1. **Brief.** No `change.md` → `specify change create migrate-foo` scaffolds it; the operator confirms the inferred default body (one `legacy-code` input pointing at the monolith).
2. **Registry.** Empty registry + `shape migrate-legacy` → hand off to the greenfield registry path inside `/change:plan`.
3. **Plan.** `/change:plan` runs discovery against the cloned monolith, proposes a two-project topology (`foo-backend`, `foo-mobile`), the operator approves both, and the registry-proposal sub-step shells `specify registry add` twice + `specify workspace sync` once before propose runs. Propose decomposes into three slices — one cross-project contract change plus one implementation slice per project — and assignment routes the implementation slices to their respective projects.
4. **Execute.** `/change:execute loop` drives all three changes to `done` (contract change runs against the hub; the two implementation changes prepare only their selected project slots on `specify/migrate-foo` before mutation). Terminal classification: `all-done`.
5. **Push.** `specify workspace push` creates `specify/migrate-foo` on each project's remote and opens two PRs.
6. **PR handoff.** The umbrella lists the open PRs and stops. The operator merges both PRs through the forge UI or an explicit hand-run `gh pr merge`.
7. **Finalize.** Re-running the umbrella observes both PRs `MERGED`; `specify change finalize` verifies remote PR state and archives `plan.yaml`, `change.md`, and `.specify/plans/migrate-foo/` into `.specify/archive/plans/<YYYYMMDD>-migrate-foo/`.

## Layout

| File | Pins |
|---|---|
| [`inputs/registry.yaml.before`](inputs/registry.yaml.before) | Empty registry that 2B's greenfield path materialises in step 3. |
| [`inputs/project.yaml`](inputs/project.yaml) | Hub `project.yaml` with `hub: true` (the `capability:` field is omitted on hubs). |
| [`expected/registry.yaml.after`](expected/registry.yaml.after) | Two-project registry after the greenfield path runs. |
| [`expected/plan.yaml.after`](expected/plan.yaml.after) | Terminal plan: three entries `done`. |
| [`expected/change.md.after`](expected/change.md.after) | Brief as scaffolded by step 1 (`specify change create`) plus the operator-confirmed default body. |
| [`expected/archive-summary.md`](expected/archive-summary.md) | Post-finalize archive shape (the path layout `specify change finalize` wrote). |
| [`transcript.md`](transcript.md) | Full skill dialogue: pre-flight, all seven steps, every shell-out and its expected output. |

## Key invariants

- **Greenfield registry path runs inside `/change:plan`.** The umbrella does not call `specify registry add` itself — every registry mutation passes through 2B's discovery + operator-approval flow inside the plan skill.
- **Contract change runs against the hub.** The cross-project HTTP contract (`migrate-foo-contract`) carries no `project` field; `/change:execute` runs it against the hub root, not inside a workspace clone.
- **PR merge is operator-owned.** The umbrella stops after `specify workspace push` until the operator merges the PRs outside the skill.
- **Idempotent re-entry.** Re-running `/change:plan <name> orchestrate migrate-foo shape migrate-legacy source monolith=...` after a successful run reports `plan-not-found` from `specify change finalize` and exits zero.
- **No retired CLI verbs.** Every shell-out in [`transcript.md`](transcript.md) uses the current surface (`specify change create`, `specify change plan {create, add, amend, validate}`, `specify slice outcome show`, `specify slice journal append`).

## Counter-examples (not pinned)

- A `--dry-run` rendering — see [§`--dry-run` semantics](../../SKILL.md#--dry-run-semantics) in the skill for the output shape.
- A halt mid-way (e.g. `registry-amendment-required` from step 4, push failure at step 5, or unmerged PRs at step 6) — recovery is documented in the skill but not pinned in this directory.
