# Running Acceptance

The acceptance surface is intentionally manual at this stage. The cross-repo scenario pack at [`tests/cross-repo/`](../../tests/cross-repo/) gives operators a repeatable script for the cross-repo happy path, and the plan-generation pack at [`tests/plan/`](../../tests/plan/) gives operators reusable `/spec:plan` scenarios focused on durable plan structure. Neither pack adds an automated harness.

## Targets

- `make checks` runs static repository checks, including scenario frontmatter validation.
- The cross-repo scenario is run manually from [`tests/cross-repo/scenario.md`](../../tests/cross-repo/scenario.md).
- The plan-generation scenarios are run manually from [`tests/plan/`](../../tests/plan/).

## What The Cross-Repo Scenario Proves

The manual scenario asks an operator to create a fresh temporary workspace with:

- a registry-only `shop-platform` workspace,
- `shop-backend` and `shop-mobile` projects,
- an OAuth login fixture brief.

It then checks the durable cross-repo behavior directly: registry setup, a three-entry contract-first plan, Gate 1 stamping, routed execution on `specify/oauth-login` branches, workspace push, external operator merge, `specify plan finalize`, archived plan state, and `plan-not-found` on a second finalize.

This repository does not add a Deno/Rust runner, fake forge, transcript replay, CI acceptance target, or golden output comparison for this scenario yet. The goal is to run the manual script a few times, learn which checks are stable, and automate only after the simple testing shape is clear.

## What The Plan Scenarios Prove

The plan-generation scenarios ask an operator to create disposable workspaces and run `/spec:plan` only. They check durable plan-authoring outcomes: `plan.yaml` exists with `lifecycle: pending`, `specify plan add` and the propose substep produce coherent slice rows, generated entries have coherent roles and dependencies, and multi-project routing follows the registry descriptions deterministically.

These scenarios deliberately stop at Gate 1 — before `specify plan transition <name> reviewed`, `/spec:execute`, workspace push, finalize, transcript replay, or golden output comparison. They are shared planning scenarios; per-target slice-loop scenarios stay under `targets/<name>/tests/`.

## Evidence

Each cross-repo manual run should fill out [`tests/cross-repo/run-summary-template.md`](../../tests/cross-repo/run-summary-template.md). On failure, preserve the workspace state, `plan.yaml`, `registry.yaml`, workspace status, push/finalize output, and branch or PR/MR identifiers.

Each plan-generation run should fill out [`tests/plan/run-summary-template.md`](../../tests/plan/run-summary-template.md). On failure, preserve the workspace state, exact `/spec:plan` prompt, `plan.yaml`, `.specify/discovery.md` candidate inventory, validation output, and any `specify plan show` output.
