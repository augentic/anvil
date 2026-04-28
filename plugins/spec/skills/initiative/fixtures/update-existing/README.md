# `--shape update-existing` — `polish-pass` end-to-end

This fixture pins the **happy path** of `/spec:initiative` driving the `update-existing` shape against a populated multi-project hub. No `--from`, no `--source`, no `--against` — sources are unused and `/spec:plan` reads `.specify/workspace/<peer>/specs/` baselines as the dominant signal during discovery. `--auto-merge` is set so the umbrella runs through to `specify initiative finalize` in one pass.

## Scenario

The platform hub `shop-platform/` has been driving cross-repo work for a while; both registered projects already carry baseline specs:

| Project | Baseline specs (excerpt) |
|---|---|
| `omnia-backend` | `user-auth`, `theme-preference`, `account-settings` |
| `vectis-mobile` | `settings-screen`, `theme-tokens`, `auth-flow` |

The operator wants a polish pass: tighten error messages on the auth flow, fix a missing accessibility label on the theme picker, and round off the documentation block in the theme-preference API. None of this is a new feature — it extends existing capabilities on both sides. They invoke:

```text
/spec:initiative create polish-pass \
    --shape update-existing \
    --auto-merge
```

The umbrella runs all seven steps in one pass:

1. **Brief.** `specify initiative create polish-pass` scaffolds `.specify/initiative.md` with **empty** `inputs:` (no `--from` / `--source` / `--against`). The operator writes one paragraph naming the capabilities being polished.
2. **Registry.** `specify registry validate` passes (multi-project, descriptions complete). No mutation.
3. **Plan.** `/spec:plan polish-pass` runs discovery against the empty input set; the discovery brief falls back to **baseline accumulation** in `.specify/workspace/<peer>/specs/` and surfaces the polish opportunities. Sync-peers refreshes both clones; propose decomposes into two slices (one per project, no contract change because the polish does not change the API surface); assignment routes each slice to its existing project; validate passes.
4. **Execute.** `/spec:execute --loop` drives both changes to `done`. Terminal classification: `all-done`.
5. **Push.** `specify workspace push` opens two PRs on `specify/polish-pass`.
6. **Land.** `--auto-merge` → `specify workspace merge` waits for CI, sees both PRs green, and squash-merges them.
7. **Finalize.** `specify initiative finalize` confirms both PRs `MERGED` on remote and archives `plan.yaml` + `initiative.md` + `.specify/plans/polish-pass/` into `.specify/archive/plans/<YYYYMMDD>-polish-pass/`.

## Layout

| File | Pins |
|---|---|
| [`inputs/registry.yaml`](inputs/registry.yaml) | Pre-populated multi-project registry. Unchanged across the run. |
| [`inputs/project.yaml`](inputs/project.yaml) | Hub `project.yaml` with `schema: hub` and `hub: true`. |
| [`expected/registry.yaml.after`](expected/registry.yaml.after) | Byte-identical to `inputs/registry.yaml`. |
| [`expected/plan.yaml.after`](expected/plan.yaml.after) | Terminal plan: two entries `done`. |
| [`expected/initiative.md.after`](expected/initiative.md.after) | Brief as scaffolded by step 1 with empty `inputs:` and operator-supplied prose. |
| [`expected/archive-summary.md`](expected/archive-summary.md) | Post-finalize archive shape. |
| [`transcript.md`](transcript.md) | Full skill dialogue: pre-flight, all seven steps, terminal summary. |

## Key invariants

- **No `--from` / `--source` / `--against`.** Pre-flight rejects any of them under `--shape update-existing` with a hard exit; the diagnostic names the offending flag and points at switching to `new-feature` or `migrate-legacy`.
- **`inputs:` is empty in the brief.** The discovery brief detects the empty input set and falls back to baseline accumulation. `/spec:plan` does not read any external file beyond `.specify/workspace/<peer>/specs/`.
- **No registry mutation.** Both projects exist; neither the discovery brief nor the assignment step proposes a new entry. The 2B registry-proposal sub-step does not fire.
- **No contract change.** The polish pass does not cross the API boundary, so propose surfaces only two slices (one per project). This contrasts with the migrate-legacy and new-feature fixtures, both of which include a cross-project `<name>-contract` slice.
- **Re-entry is idempotent.** A second run of the same invocation reports `plan-not-found` from `specify initiative finalize` and exits zero.

## Counter-examples (not pinned)

- A run where the discovery brief surfaces no polish opportunities — the propose step would skip every slice and `specify plan validate` would fail with an empty-plan diagnostic; the umbrella halts at step 3.
- A `--dry-run` rendering — see [§`--dry-run` semantics](../../SKILL.md#--dry-run-semantics).
- A run that surfaces a `registry-amendment-required` outcome at step 4 (operator decides one of the polish slices needs a new project mid-execute) — recovery is documented in the SKILL but not pinned here.
