# Running Acceptance

The acceptance surface has two layers:

1. **Deterministic boundary harness** — `make test` runs a compact Rust regression suite under [`tooling/tests/`](../../tooling/tests/). It exercises the checker code with targeted broken fixtures plus one real-repo smoke test that runs the registered `tooling check` predicates. It does not replay source/target/skill fixtures or invoke the live `specify` binary.
2. **Manual scenario sweep** — The cross-repo scenario pack at [`tests/cross-repo/`](../../tests/cross-repo/) and the plan-generation pack at [`tests/plan/`](../../tests/plan/) are operator-driven scripts that exercise the full `/spec:plan` → `/spec:execute` → `/spec:finalize` rhythm against live `cursor-agent`. They remain manual because they involve LLM-emitted prose; the deterministic-boundary harness above does **not** pin synthesised bytes.

## Running the harness locally

```bash
cargo test --manifest-path tooling/Cargo.toml
```

Set `SPECIFY_CLI_DIR` to a checkout of [`augentic/specify-cli`](https://github.com/augentic/specify-cli) when adapter schema validation needs runtime schemas (defaults to `../specify-cli`).

## Targets

- `make check` runs `tooling check` — static repository checks, including scenario frontmatter validation.
- `make test` runs the same acceptance tests as `cargo test --manifest-path tooling/Cargo.toml`.
- `make ci` runs both sequentially.
- The cross-repo scenario is run manually from [`tests/cross-repo/scenario.md`](../../tests/cross-repo/scenario.md).
- The plan-generation scenarios are run manually from [`tests/plan/`](../../tests/plan/).

## Synthesis byte-replay (deferred)

The harness in `tooling/tests/` covers checker regressions and repo consistency, but does **not** assert on the bytes a `/spec:refine` or `/spec:build` skill body emits. The skill bodies are agent-driven markdown and the byte-equivalent of "synthesis golden" requires either:

- a **recorded-transcript layer** that captures a `cursor-agent` run via `@cursor/sdk` and replays the persisted output back through the harness, or
- a **structured-trace assertion library** that compares the *shape* of synthesised artifacts (sections, IDs, Sources, Status enums) rather than the bytes.

Both options are out of scope for the 2.0 cutover. A follow-up RFC will pick one. Until then, the manual scenario sweep below is the source of truth for end-to-end LLM-driven correctness.

## What The Cross-Repo Scenario Proves

The manual scenario asks an operator to create a fresh temporary workspace with:

- a registry-only `shop-platform` workspace,
- `shop-backend` and `shop-mobile` projects,
- an OAuth login fixture brief.

It then checks the durable cross-repo behavior directly: registry setup, a three-entry contract-first plan, Gate 1 stamping, routed execution on `specify/oauth-login` branches, workspace push, external operator merge, `specify plan finalize`, archived plan state, and `plan-not-found` on a second finalize.

This repository does not add an automated runner, fake forge, transcript replay, CI acceptance target, or golden output comparison for this scenario yet. The goal is to run the manual script a few times, learn which checks are stable, and automate only after the simple testing shape is clear.

## What The Plan Scenarios Prove

The plan-generation scenarios ask an operator to create disposable workspaces and run `/spec:plan` only. They check durable plan-authoring outcomes: `plan.yaml` exists with `lifecycle: pending`, `specify plan add` and the propose substep produce coherent slice rows, generated entries have coherent roles and dependencies, and multi-project routing follows the registry descriptions deterministically.

These scenarios deliberately stop at Gate 1 — before `specify plan transition <name> reviewed`, `/spec:execute`, workspace push, finalize, transcript replay, or golden output comparison. They are shared planning scenarios; per-target slice-loop scenarios stay under `adapters/targets/<name>/tests/`.

## Evidence

Each cross-repo manual run should fill out [`tests/cross-repo/run-summary-template.md`](../../tests/cross-repo/run-summary-template.md). On failure, preserve the workspace state, `plan.yaml`, `registry.yaml`, workspace status, push/finalize output, and branch or PR/MR identifiers.

Each plan-generation run should fill out [`tests/plan/run-summary-template.md`](../../tests/plan/run-summary-template.md). On failure, preserve the workspace state, exact `/spec:plan` prompt, `plan.yaml`, `.specify/discovery.md` candidate inventory, validation output, and any `specify plan show` output.
