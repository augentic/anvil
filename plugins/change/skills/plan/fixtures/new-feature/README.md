# `--shape new-feature` — `dark-mode` end-to-end

This fixture pins the **happy path** of `/change:plan <name> orchestrate` driving the `new-feature` shape against a populated multi-project hub. Sources arrive via `from <docs>` only; the registry is already populated, so step 3 routes work to existing projects without any registry mutation. Step 6 lists open PRs and stops, and the operator re-runs the umbrella after merging by hand to land step 7.

## Scenario

The platform hub `shop-platform/` was bootstrapped earlier (`specify init --hub`) and registers two code projects:

| Project | Schema | Domain |
|---|---|---|
| `omnia-backend` | `omnia@v1` | User accounts, settings persistence, theme-preference API. |
| `vectis-mobile` | `vectis@v1` | iOS and Android shells. Settings screens, theme-aware UI. |

The operator wants to land a `dark-mode` feature spanning both. They drop a one-page spec at `./docs/dark-mode-spec.md` and invoke:

```text
/change:plan <name> orchestrate dark-mode \
    shape new-feature \
    from ./docs/dark-mode-spec.md
```

The umbrella runs steps 1–5 in one pass, halts at step 6 for PR handoff, and the operator re-runs after merging by hand to land step 7.

### First run (steps 1–6, halts at step 6)

1. **Brief.** `specify change create dark-mode` scaffolds `change.md`; the operator confirms a default body that lists `./docs/dark-mode-spec.md` as a `documentation` input.
2. **Registry.** `specify registry validate` passes — both existing projects have descriptions.
3. **Plan.** `/change:plan dark-mode from ./docs/dark-mode-spec.md` runs discovery against the docs, syncs peers (multi-project registry), proposes three slices (one cross-project contract change for the theme-preference API plus one implementation slice per project), assigns each implementation slice to its existing project, and validates. **No registry mutation** — both projects are already registered.
4. **Execute.** `/change:execute loop` drives all three changes to `done`. Terminal classification: `all-done`.
5. **Push.** `specify workspace push` creates `specify/dark-mode` on each project's remote and opens two PRs.
6. **PR handoff.** The umbrella lists the open PRs and stops:

   ```text
   Step 6 — PR handoff

     foo-backend     specify/dark-mode    PR #57    https://github.com/org/omnia-backend/pull/57
     foo-mobile      specify/dark-mode    PR #29    https://github.com/org/vectis-mobile/pull/29

   Merge these PRs through the forge UI or an explicit hand-run
   `gh pr merge`, then re-run /change:plan <name> orchestrate dark-mode
   to finalize.
   ```

### Second run (re-entry, runs step 7 only)

After the operator merges both PRs by hand, they re-run the umbrella:

```text
$ /change:plan <name> orchestrate dark-mode shape new-feature from ./docs/dark-mode-spec.md
```

The umbrella inspects on-disk state, sees the brief present, the plan terminal, and every PR `MERGED` on remote, and skips to step 7:

7. **Finalize.** `specify change finalize` runs the four guards, sweeps `plan.yaml` + `change.md` + `.specify/plans/dark-mode/` into `.specify/archive/plans/<YYYYMMDD>-dark-mode/`.

## Layout

| File | Pins |
|---|---|
| [`inputs/registry.yaml`](inputs/registry.yaml) | Pre-populated registry — two existing projects with descriptions. Unchanged across the run. |
| [`inputs/project.yaml`](inputs/project.yaml) | Hub `project.yaml` with `hub: true` (the `capability:` field is omitted on hubs). |
| [`inputs/dark-mode-spec.md`](inputs/dark-mode-spec.md) | The documentation input forwarded to `/change:plan` via `from`. |
| [`expected/registry.yaml.after`](expected/registry.yaml.after) | Byte-identical to `inputs/registry.yaml` — the `new-feature` shape never mutates the registry. |
| [`expected/plan.yaml.after`](expected/plan.yaml.after) | Terminal plan: three entries `done`. |
| [`expected/change.md.after`](expected/change.md.after) | Brief as scaffolded by step 1 plus the operator-confirmed default body. |
| [`expected/archive-summary.md`](expected/archive-summary.md) | Post-finalize archive shape after the second run. |
| [`transcript.md`](transcript.md) | Full skill dialogue across the two runs: pre-flight, steps 1–6 in run 1, halt-at-6, re-entry, step 7 in run 2. |

## Key invariants

- **No registry mutation under `new-feature`.** Both projects exist in the registry at start; assignment routes work to them without any `specify registry add` shell-out. The 2B registry-proposal sub-step does not fire.
- **Step 6 stops for operator merge.** The umbrella surfaces the list of open PRs and exits zero — the operator merges by hand and re-runs to finalize.
- **Re-entry is idempotent.** The second run skips steps 1–6 (each shell-out underneath is idempotent: `specify change create` refuses on populated brief, `/change:plan` would refuse without `extend` but the umbrella never re-enters `/change:plan` because the plan is already terminal, `specify workspace push` reports `up-to-date`) and lands directly at step 7.
- **Verb hygiene.** Every shell-out in [`transcript.md`](transcript.md) uses current verbs (`specify change {create, finalize}`, `specify change plan {add, amend, validate}`, `specify registry validate`, `specify workspace {sync, push}`, `gh pr list`, `gh pr view`).

## Counter-examples (not pinned)

- A halt at step 4 with `registry-amendment-required` (operator decides one of the slices needs a new project mid-execute) — recovery is documented in the SKILL but not pinned here.
- A `--dry-run` rendering — see [§`--dry-run` semantics](../../SKILL.md#--dry-run-semantics).
