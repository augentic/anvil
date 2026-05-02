# `--shape migrate-legacy` — `migrate-foo` end-to-end

This fixture pins the **happy path** of `/spec:plan --orchestrate` (formerly `/spec:initiative`) driving the `migrate-legacy` shape against an empty platform hub. Sources arrive via `--source <key>=<git-url>`; the registry is empty, so step 3 enters the 2B greenfield path; `--auto-merge` lets the umbrella run all the way through to `specify initiative finalize`.

## Scenario

The operator wants to migrate the legacy `mono-repo-foo` TypeScript service onto Augentic's Omnia + Vectis stack. They start in a fresh hub repo (`shop-platform/`) created earlier with `specify init --hub`.

```text
/spec:plan --orchestrate migrate-foo \
    --shape migrate-legacy \
    --source monolith=git@github.com:org/legacy-foo.git \
    --auto-merge
```

The umbrella runs all seven steps without halting:

1. **Brief.** No `initiative.md` → `specify initiative create migrate-foo` scaffolds it; the operator confirms the inferred default body (one `legacy-code` input pointing at the monolith).
2. **Registry.** Empty registry + `--shape migrate-legacy` → hand off to the 2B greenfield path inside `/spec:plan`.
3. **Plan.** `/spec:plan` runs discovery against the cloned monolith, proposes a two-project topology (`foo-backend`, `foo-mobile`), the operator approves both, and the registry-proposal sub-step shells `specify registry add` twice + `specify workspace sync` once before propose runs. Propose decomposes into three slices — one cross-project contract change plus one implementation slice per project — and assignment routes the implementation slices to their respective projects.
4. **Execute.** `/spec:execute --loop` drives all three changes to `done` (contract change runs against the hub; the two implementation changes run inside their workspace clones). Terminal classification: `all-done`.
5. **Push.** `specify workspace push` creates `specify/migrate-foo` on each project's remote and opens two PRs.
6. **Land.** `--auto-merge` → `specify workspace merge` waits for CI, sees both PRs green, and `gh pr merge --squash`es them.
7. **Finalize.** `specify initiative finalize` confirms both PRs `MERGED` on remote and archives `plan.yaml`, `initiative.md`, and `.specify/plans/migrate-foo/` into `.specify/archive/plans/<YYYYMMDD>-migrate-foo/`.

## Layout

| File | Pins |
|---|---|
| [`inputs/registry.yaml.before`](inputs/registry.yaml.before) | Empty registry that 2B's greenfield path materialises in step 3. |
| [`inputs/project.yaml`](inputs/project.yaml) | Hub `project.yaml` with `schema: hub` and `hub: true`. |
| [`expected/registry.yaml.after`](expected/registry.yaml.after) | Two-project registry after the greenfield path runs. |
| [`expected/plan.yaml.after`](expected/plan.yaml.after) | Terminal plan: three entries `done`. |
| [`expected/initiative.md.after`](expected/initiative.md.after) | Brief as scaffolded by step 1 (`specify initiative create`) plus the operator-confirmed default body. |
| [`expected/archive-summary.md`](expected/archive-summary.md) | Post-finalize archive shape (the path layout `specify initiative finalize` wrote). |
| [`transcript.md`](transcript.md) | Full skill dialogue: pre-flight, all seven steps, every shell-out and its expected output. |

## Key invariants

- **Greenfield registry path runs inside `/spec:plan`.** The umbrella does not call `specify registry add` itself — every registry mutation passes through 2B's discovery + operator-approval flow inside the plan skill.
- **Contract change runs against the hub.** The cross-project HTTP contract (`migrate-foo-contract`) carries no `project` field; `/spec:execute` runs it against the hub root, not inside a workspace clone.
- **`--auto-merge` is best-effort across projects.** Each project lands independently; one project's `failed-checks` would not abort the others' merges. This fixture pins the fully-green path; per-project failure is documented in [`/spec:execute` → §Cross-project contract check](../../../execute/SKILL.md#cross-project-contract-check-rfc-9-3b) and in `specify workspace merge`'s status table.
- **Idempotent re-entry.** Re-running `/spec:plan --orchestrate migrate-foo --shape migrate-legacy --source monolith=... --auto-merge` after a successful run reports `plan-not-found` from `specify initiative finalize` and exits zero.
- **No retired CLI verbs.** Every shell-out in [`transcript.md`](transcript.md) uses the post-1F+1G v1 surface (`specify initiative create`, `specify plan {create, add, amend, validate}`, `specify change outcome show`, `specify change journal append`). The retired-verb checker enforces this — see the migration map in [docs/explanation/migrating-cli-v1.md](../../../../../../docs/explanation/migrating-cli-v1.md) for the full rename trail.

## Counter-examples (not pinned)

- A `--dry-run` rendering — see [§`--dry-run` semantics](../../SKILL.md#--dry-run-semantics) in the skill for the output shape.
- A halt mid-way (e.g. `registry-amendment-required` from step 4, `failed-checks` from step 6) — recovery is documented in the skill but not pinned in this directory.
- A non-`--auto-merge` run — see [`fixtures/update-existing/`](../update-existing/) for the supervised land path.
