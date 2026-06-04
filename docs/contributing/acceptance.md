# Running Acceptance

The acceptance surface has two layers:

1. **Static repository checks** — `make lint` runs `specify lint framework --framework-root .` against the live tree. This is the only deterministic surface this repo owns; it validates skill frontmatter, adapter manifests, rule shape, links, marketplace consistency, and scenario frontmatter. The `specify-standards` framework predicate *regression* suite (broken-fixture tests that prove each predicate fires correctly) lives in and is run by `augentic/specify-cli` — its `cargo make test` covers the whole workspace, including `specify-standards` framework — so this repo does not re-run it.
2. **Manual scenario sweep** — The cross-repo scenario pack at [`tests/cross-repo/`](../../tests/cross-repo/) and the plan-generation pack at [`tests/plan/`](../../tests/plan/) are operator-driven scripts that exercise the full `/spec:plan` → `/spec:execute` → `/spec:finalize` rhythm against live `cursor-agent`. They remain manual because they involve LLM-emitted prose; `specify lint framework` does **not** pin synthesised bytes.

## Running checks locally

```bash
make lint
```

Set `SPECIFY_FRAMEWORK_ROOT` only when invoking `specify lint framework` directly without `--framework-root`. To run the predicate regression suite, use `cargo make test` from a `specify-cli` checkout.

## Targets

- `make lint` runs `specify lint framework` — static repository checks, including scenario frontmatter validation.
- `make ci` runs `make lint` plus the `check-schemas` target (`scripts/check-schema-mirror.sh`, which verifies the `.cursor/schemas/` mirrors match the CLI). Bare `make lint` does not run the schema-mirror check.
- The `specify-standards` framework predicate regression suite is run by `cargo make test` in the `specify-cli` repo.
- The cross-repo scenario is run manually from [`tests/cross-repo/scenario.md`](../../tests/cross-repo/scenario.md).
- The plan-generation scenarios are run manually from [`tests/plan/`](../../tests/plan/).

## Synthesis byte-replay (deferred)

The harness in the `specify-standards` crate covers checker regressions and repo consistency, but does **not** assert on the bytes a `/spec:refine` or `/spec:build` skill body emits. The skill bodies are agent-driven markdown and the byte-equivalent of "synthesis golden" requires either:

- a **recorded-transcript layer** that captures a `cursor-agent` run via `@cursor/sdk` and replays the persisted output back through the harness, or
- a **structured-trace assertion library** that compares the *shape* of synthesised artifacts (sections, IDs, Sources, Status enums) rather than the bytes.

Both options are out of scope for the 2.0 cutover. A follow-up RFC will pick one. Until then, the manual scenario sweep below is the source of truth for end-to-end LLM-driven correctness.

## What The Cross-Repo Scenario Proves

The manual scenario asks an operator to create a fresh temporary workspace with:

- a registry-only `shop-platform` workspace,
- `shop-backend` and `shop-mobile` projects,
- an OAuth login fixture brief.

It then checks the durable cross-repo behavior directly: registry setup, a three-entry contract-first plan, Gate 1 stamping, routed execution on `specify/oauth-login` branches, workspace push, external operator merge, `/spec:finalize` PR observation, `specify plan archive`, archived plan state, and already-archived re-entry handling.

This repository does not add an automated runner, fake forge, transcript replay, CI acceptance target, or golden output comparison for this scenario yet. The goal is to run the manual script a few times, learn which checks are stable, and automate only after the simple testing shape is clear.

## What The Plan Scenarios Prove

The plan-generation scenarios ask an operator to create disposable workspaces and run `/spec:plan` only. They check durable plan-authoring outcomes: `plan.yaml` exists with `lifecycle: pending`, `specify plan add` and the propose substep produce coherent slice rows, generated entries have coherent roles and dependencies, and multi-project routing follows the registry descriptions deterministically.

These scenarios deliberately stop at Gate 1 — before `specify plan transition <name> approved`, `/spec:execute`, workspace push, finalize, transcript replay, or golden output comparison. They are shared planning scenarios; per-target slice-loop scenarios stay under `adapters/targets/<name>/tests/`.

## Fan-in / fan-out acceptance

The cross-source fan-in / cross-slice fan-out acceptance splits across two distinct surfaces, and **both** must pass before a release is complete:

1. **Deterministic CLI proof (automated).** The end-to-end fan-in-twice / fan-out-once fixture lives in `augentic/specify-cli` at [`tests/fan_in_fan_out.rs`](https://github.com/augentic/specify-cli/blob/main/tests/fan_in_fan_out.rs). It runs under `cargo make test` and asserts the **envelope, ordering, and determinism** of the whole path — `source survey` → `plan propose --dry-run | --from` → per-slice `source extract` → `slice synthesize` → `slice build` → `slice merge`, plus `depends-on` ordering and byte-identical kernel re-projection. It does **not** execute real target codegen.

2. **Generated-output-correctness release gate (manual / CI).** Each target build must pass the target's own **replay/golden suite** plus `cargo check` / `cargo test` for any generated crates (and the equivalent verification for non-Rust targets). A slice whose generated output fails these checks **is not done — regardless of build-envelope validity**. A schema-valid `build/report.yaml` with `status: success` only proves the envelope contract held; it does not prove the emitted code compiles or replays. This gate is manual/CI because it exercises agent-generated code, which `specify lint framework` and the deterministic CLI proof do not pin.

## Scenario IDs

The 2.0 manual run stubs use stable scenario IDs instead of historical RFC row numbers. The canonical queue lives in [`tests/cross-repo/runs/2.0.0/`](../../tests/cross-repo/runs/2.0.0/); each stub links back here so acceptance references survive archive cleanup.

| Scenario ID | Meaning | Stub |
| --- | --- | --- |
| `1` | Pure intent, one slice | [`01-pure-intent.md`](../../tests/cross-repo/runs/2.0.0/01-pure-intent.md) |
| `2` | Documentation, one slice | [`02-documentation-one-slice.md`](../../tests/cross-repo/runs/2.0.0/02-documentation-one-slice.md) |
| `3` | Documentation, multi-slice | [`03-documentation-multi-slice.md`](../../tests/cross-repo/runs/2.0.0/03-documentation-multi-slice.md) |
| `4` | Code, multi-slice | [`04-code-multi-slice.md`](../../tests/cross-repo/runs/2.0.0/04-code-multi-slice.md) |
| `5` | Intra-Evidence `[conflict]` | [`05-intra-evidence-conflict.md`](../../tests/cross-repo/runs/2.0.0/05-intra-evidence-conflict.md) |
| `5a` | Combined evidence from code and documentation | [`05a-combined-evidence.md`](../../tests/cross-repo/runs/2.0.0/05a-combined-evidence.md) |
| `5b` | `[divergence]` from authority resolution | [`05b-divergence-authority.md`](../../tests/cross-repo/runs/2.0.0/05b-divergence-authority.md) |
| `5c` | `[conflict]` from same-authority disagreement | [`05c-same-authority-conflict.md`](../../tests/cross-repo/runs/2.0.0/05c-same-authority-conflict.md) |
| `5e` | Cross-source propose-time merge | [`05e-cross-source-merge.md`](../../tests/cross-repo/runs/2.0.0/05e-cross-source-merge.md) |
| `5f` | Extract failure | [`05f-extract-failure.md`](../../tests/cross-repo/runs/2.0.0/05f-extract-failure.md) |
| `5g` | Invalid Evidence schema rejection | [`05g-invalid-evidence.md`](../../tests/cross-repo/runs/2.0.0/05g-invalid-evidence.md) |
| `5h` | Target `shape` injection | [`05h-target-shape-injection.md`](../../tests/cross-repo/runs/2.0.0/05h-target-shape-injection.md) |
| `5j` | Source-adapter sandbox path-denied | [`05j-source-sandbox-denied.md`](../../tests/cross-repo/runs/2.0.0/05j-source-sandbox-denied.md) |
| `6` | Multi-repo assignment from a workspace | [`06-multi-repo-workspace.md`](../../tests/cross-repo/runs/2.0.0/06-multi-repo-workspace.md) |
| `7` | Operator amends one-slice plan into two slices at Gate 1 | [`07-amend-into-two.md`](../../tests/cross-repo/runs/2.0.0/07-amend-into-two.md) |
| `8` | Step-through breakout mid-execute | [`08-stepthrough-breakout.md`](../../tests/cross-repo/runs/2.0.0/08-stepthrough-breakout.md) |
| `9` | `/spec:execute` parks on a build failure, operator fixes, resumes | [`09-execute-build-failure.md`](../../tests/cross-repo/runs/2.0.0/09-execute-build-failure.md) |
| `10` | Workspace `/spec:execute` across two projects | [`10-workspace-execute-two-projects.md`](../../tests/cross-repo/runs/2.0.0/10-workspace-execute-two-projects.md) |
| `11` | Workspace breakout after build failure in a slot | [`11-workspace-breakout.md`](../../tests/cross-repo/runs/2.0.0/11-workspace-breakout.md) |
| `12` | Dual-driving refused | [`12-dual-driving-refused.md`](../../tests/cross-repo/runs/2.0.0/12-dual-driving-refused.md) |
| `13` | Stale-workspace recovery | [`13-stale-workspace-recovery.md`](../../tests/cross-repo/runs/2.0.0/13-stale-workspace-recovery.md) |

## Evidence

Each cross-repo manual run should fill out [`tests/cross-repo/run-summary-template.md`](../../tests/cross-repo/run-summary-template.md). On failure, preserve the workspace state, `plan.yaml`, `registry.yaml`, workspace status, push/finalize output, and branch or PR/MR identifiers.

Each plan-generation run should fill out [`tests/plan/run-summary-template.md`](../../tests/plan/run-summary-template.md). On failure, preserve the workspace state, exact `/spec:plan` prompt, `plan.yaml`, `.specify/discovery.md` lead inventory, validation output, and any `specify plan show` output.
