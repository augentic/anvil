# Running Acceptance

The acceptance surface is intentionally manual at this stage. RM-01 is covered
by the scenario pack at [`tests/rm-01/`](../../tests/rm-01/), which gives
operators a repeatable script for the cross-repo happy path without adding an
automated harness.

## Targets

- `make checks` runs static repository checks, including scenario frontmatter
  validation.
- RM-01 is run manually from [`tests/rm-01/scenario.md`](../../tests/rm-01/scenario.md).

## What RM-01 Proves

The manual scenario asks an operator to create a fresh temporary workspace with:

- a registry-only `shop-platform` hub,
- `shop-backend` and `shop-mobile` projects,
- an OAuth login fixture brief.

It then checks the durable RM-01 behavior directly: registry setup, a
three-entry contract-first plan, routed execution on `specify/oauth-login`
branches, workspace push, external operator merge, `change finalize`, archived
plan state, and `plan-not-found` on a second finalize.

This repository does not add a Deno/Rust runner, fake forge, transcript replay,
CI acceptance target, or golden output comparison for RM-01 yet. The goal is to
run the manual script a few times, learn which checks are stable, and automate
only after the simple testing shape is clear.

## Evidence

Each manual run should fill out
[`tests/rm-01/run-summary-template.md`](../../tests/rm-01/run-summary-template.md).
On failure, preserve the hub state, `plan.yaml`, `registry.yaml`, workspace
status, push/finalize output, and branch or PR/MR identifiers.
