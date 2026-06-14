---
id: contract-lifecycle
owner: scenarios
kind: suite
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
  - finalize-pushes-branches
  - finalize-archives-plan
  - archived-plan-path-recorded
  - archived-change-md-present
  - pushed-branch-list-recorded
  - rerun-finalize-plan-not-found
expected-artifacts:
  - plan.yaml
  - registry.yaml
  - workspace
  - .specify/archive/plans
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
---

# Cross-Repo Contract Flow

Scenario ID: `contract-lifecycle`

The full cross-repo happy path: a short feature brief becomes one contract slice and two routed implementation slices, the operator reviews the draft plan, the change executes, and `/spec:finalize` pushes the prepared branches to each project's local bare-repo `origin` and archives the plan in one invocation. Opening and merging the pull requests is an operator action done by hand outside Specify.

## Intent

Prove the operator-facing Specify workflow can drive the cross-repo contract-first path end to end through `/spec:plan → Gate 1 → /spec:execute → /spec:finalize`, producing a durable end-state: each routed project's `specify/oauth-login` branch published to its `origin`, the archived plan path under `.specify/archive/plans/`, and the archived `change.md` next to the archived `plan.yaml`. The scenario checks durable structure and state transitions only — it must not fail because generated prose or implementation code differs from a previous run. It runs entirely against local bare-repo remotes; no forge client (`gh`) or network is involved.

```text
feature brief
  -> /spec:plan oauth-login
  -> contract slice + routed backend and mobile implementation slices
  -> operator review pause (inspect plan.yaml)
  -> specify plan transition oauth-login approved
  -> /spec:execute loop
  -> /spec:finalize oauth-login   (pushes branches, then archives the plan)
  -> /spec:finalize oauth-login   (no active plan)
```

## Setup

Follow [`shared/setup.md`](../shared/setup.md): the **cross-repo workspace setup** (workspace `platform` plus registered `backend` / `mobile`, each with a local bare-repo `origin`) and the **OAuth login brief** at `docs/oauth-login.md`.

## Invocation

1. **Draft** — from the workspace, run `/spec:plan oauth-login source brief=docs/oauth-login.md`, asking for one contract slice plus backend and mobile implementation slices that both depend on the contract slice. The skill writes `change.md` + `plan.yaml`, validates, and stops at the hand-off (`pending`) printing the literal `specify plan transition oauth-login approved`. It must not proceed into execution.
2. **Review (operator pause)** — `specify plan validate` and inspect `plan.yaml` read-only; confirm the slice shape. No `specify plan amend` for the parity run.
3. **Stamp Gate 1** — run the literal `specify plan transition oauth-login approved`.
4. **Execute** — `/spec:execute loop`; answer only genuine clarification prompts. The loop exits because the plan is complete (`all-done`).
5. **Finalize** — `/spec:finalize oauth-login`. The skill pushes the prepared `specify/oauth-login` branches to each project's bare-repo `origin` (per-project status `pushed`), then runs `specify plan archive` (archiving `plan.yaml` + `change.md` under `.specify/archive/plans/oauth-login-<date>/`), and prints the pushed-branch list, a reminder to open PRs by hand, and the archived path.
6. **Open PRs externally (out of scope for the assertions)** — the operator opens and merges the backend and mobile pull requests by hand outside Specify; the scenario does not drive or assert this.
7. **Finalize (re-run)** — `/spec:finalize oauth-login` reports no active plan remains and exits 0.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly after draft and during review.
- `contract-slice-first`: the dependency graph makes the contract slice the first executable slice.
- `implementation-slices-routed`: exactly two implementation slices route to `backend` and `mobile`.
- `dependencies-contract-before-implementations`: each implementation slice depends on the contract slice.
- `draft-stops-at-handoff`: `/spec:plan` exits at the hand-off without executing, pushing, or finalizing.
- `review-step-no-op`: inspecting `plan.yaml` between draft and execute reports the plan as authored.
- `execute-loop-all-done`: `/spec:execute loop` exits because the plan is complete, not stuck/failed/interrupted.
- `workspace-branches-prepared`: routed project work happens on `specify/oauth-login` branches.
- `finalize-pushes-branches`: `/spec:finalize` pushes the `specify/oauth-login` branch to each project's `origin` (per-project status `pushed`); it creates, observes, and merges no PRs.
- `finalize-archives-plan`: the same `/spec:finalize` run archives the plan via `specify plan archive` after the push succeeds.
- `archived-plan-path-recorded`: the wrap-up names the archived plan path under `.specify/archive/plans/`.
- `archived-change-md-present`: the archived directory contains the archived `change.md`.
- `pushed-branch-list-recorded`: the wrap-up lists exactly two pushed branches (one per routed project) and reminds the operator to open PRs by hand.
- `rerun-finalize-plan-not-found`: a second `/spec:finalize` reports no active plan and exits 0.

## Negative expectations

Manual by design — see [`docs/contributing/evals.md`](../../docs/contributing/evals.md). This scenario must not add an automated runner, fake forge, recorded transcript, CI target, or required byte-for-byte golden comparison. Parity is asserted on durable structure (archive path shape, pushed-branch count and project mapping, archive directory contents).

## Recording

Capture the run with [`shared/run-template.md`](../shared/run-template.md) as [`evals/runs/<id>.<result>.md`](../runs/README.md). Preserve on failure: `plan.yaml` (or archived path), `registry.yaml`, every `/spec:finalize` output, workspace status, and branch identifiers.
