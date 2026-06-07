---
id: cross-repo-contract-flow
owner: scenarios
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine, build, merge]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - contract-slice-first
  - implementation-slices-routed
  - dependencies-contract-before-implementations
  - draft-stops-at-handoff
  - review-step-no-op
  - execute-loop-all-done
  - workspace-branches-prepared
  - finalize-halts-on-unmerged-prs
  - finalize-archives-plan
  - archived-plan-path-recorded
  - archived-change-md-present
  - merged-pr-list-recorded
  - rerun-finalize-plan-not-found
expected-artifacts:
  - plan.yaml
  - registry.yaml
  - .specify/workspace
  - .specify/archive/plans
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
---

# Cross-Repo Contract Flow

Scenario ID: `cross-repo-contract-flow`

The full cross-repo happy path: a short feature brief becomes one contract slice and two routed implementation slices, the operator reviews the draft plan, the change executes, and `/spec:finalize` drives push, PR observation, and archive after the project branches are merged externally.

## Intent

Prove the operator-facing Specify workflow can drive the cross-repo contract-first path end to end through `/spec:plan → Gate 1 → /spec:execute → /spec:finalize`, producing a durable end-state: archived plan path under `.specify/archive/plans/`, one merged PR per routed project, and the archived `change.md` next to the archived `plan.yaml`. The scenario checks durable structure and state transitions only — it must not fail because generated prose or implementation code differs from a previous run.

```text
feature brief
  -> /spec:plan oauth-login
  -> contract slice + routed backend and mobile implementation slices
  -> operator review pause (inspect plan.yaml)
  -> specify plan transition oauth-login approved
  -> /spec:execute loop
  -> /spec:finalize oauth-login   (halts on unmerged PRs)
  -> operator merges PRs externally
  -> /spec:finalize oauth-login   (archives the plan)
  -> /spec:finalize oauth-login   (no active plan)
```

## Setup

Follow [`shared/setup.md`](../shared/setup.md): the **cross-repo workspace setup** (workspace `shop-platform` plus registered `shop-backend` / `shop-mobile`) and the **OAuth login brief** at `docs/oauth-login.md`.

## Invocation

1. **Draft** — from the workspace, run `/spec:plan oauth-login source brief=docs/oauth-login.md`, asking for one contract slice plus backend and mobile implementation slices that both depend on the contract slice. The skill writes `change.md` + `plan.yaml`, validates, and stops at the hand-off (`pending`) printing the literal `specify plan transition oauth-login approved`. It must not proceed into execution.
2. **Review (operator pause)** — `specify plan validate` and inspect `plan.yaml` read-only; confirm the slice shape. No `specify plan amend` for the parity run.
3. **Stamp Gate 1** — run the literal `specify plan transition oauth-login approved`.
4. **Execute** — `/spec:execute loop`; answer only genuine clarification prompts. The loop exits because the plan is complete (`all-done`).
5. **Finalize (first)** — `/spec:finalize oauth-login`. Push the prepared `specify/oauth-login` branches; `gh pr list` shows the fresh PRs unmerged, so the skill halts with `pr-not-merged`, naming each PR + URL.
6. **Merge externally** — merge the backend and mobile PRs through the normal operator forge workflow. `/spec:finalize` never merges PRs itself.
7. **Finalize (second)** — `/spec:finalize oauth-login`. Push reports `up-to-date`, PRs report `MERGED`, the skill runs `specify plan archive` (archiving `plan.yaml` + `change.md` under `.specify/archive/plans/oauth-login-<date>/`), and prints the merged-PR list and archived path.
8. **Finalize (third)** — `/spec:finalize oauth-login` reports no active plan remains and exits 0.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly after draft and during review.
- `contract-slice-first`: the dependency graph makes the contract slice the first executable slice.
- `implementation-slices-routed`: exactly two implementation slices route to `shop-backend` and `shop-mobile`.
- `dependencies-contract-before-implementations`: each implementation slice depends on the contract slice.
- `draft-stops-at-handoff`: `/spec:plan` exits at the hand-off without executing, pushing, or finalizing.
- `review-step-no-op`: inspecting `plan.yaml` between draft and execute reports the plan as authored.
- `execute-loop-all-done`: `/spec:execute loop` exits because the plan is complete, not stuck/failed/interrupted.
- `workspace-branches-prepared`: routed project work happens on `specify/oauth-login` branches.
- `finalize-halts-on-unmerged-prs`: the first `/spec:finalize` runs push and halts with `pr-not-merged` naming both PR URLs.
- `finalize-archives-plan`: after external merges, the second `/spec:finalize` archives the plan via `specify plan archive`.
- `archived-plan-path-recorded`: the wrap-up names the archived plan path under `.specify/archive/plans/`.
- `archived-change-md-present`: the archived directory contains the archived `change.md`.
- `merged-pr-list-recorded`: the wrap-up lists exactly two merged PRs (one per routed project) with numbers and URLs.
- `rerun-finalize-plan-not-found`: a third `/spec:finalize` reports no active plan and exits 0.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../docs/contributing/acceptance.md). This scenario must not add an automated runner, fake forge, recorded transcript, CI target, or required byte-for-byte golden comparison. Parity is asserted on durable structure (archive path shape, merged-PR count and project mapping, archive directory contents).

## Recording

Capture the run with [`shared/run-summary-template.md`](../shared/run-summary-template.md) under [`acceptance/runs/`](../runs/README.md). Preserve on failure: `plan.yaml` (or archived path), `registry.yaml`, every `/spec:finalize` output, workspace status, and branch/PR identifiers.
