---
id: plan-single-project
owner: plan
kind: suite
backend: manual
entrypoint: /change:plan
stages: [define]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - slices-match-expected-shape
  - no-project-routing-required
expected-artifacts:
  - plan.yaml
  - .specify/plans/inventory-adjustments/discovery.md
  - .specify/plans/inventory-adjustments/proposal.md
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
---

# Single-Project Plan Generation

Scenario ID: `plan-single-project`

Use this scenario to manually verify that a short brief in one initialized project produces a small, valid `plan.yaml` with local slice entries only.

This scenario is deliberately manual. It does not introduce a test runner, fake forge, recorded transcript, CI target, or golden output comparison.

## Intent

Prove that `/change:plan` can turn a simple documentation input into a reviewable plan for a single project:

```text
feature brief
  -> /change:plan
  -> local slice entries
  -> specify change plan validate
```

The scenario checks durable plan structure only. It should not fail because the generated proposal prose or slice descriptions differ from a previous run.

## Workspace

- **Suite:** plan.
- **Project shape:** one temporary project.
- **Project capability:** `omnia@v1`.
- **Registry shape:** not applicable; this scenario does not use `registry.yaml`.
- **Isolation:** `fresh-project`. Use a disposable directory and start with empty Specify state.
- **Backend:** `manual` - a human or agent follows this script and records results in the [run summary](run-summary-template.md).

Prerequisites:

- A current `specify` binary available on `PATH`, or `SPECIFY_BIN` documented in the run summary if the operator uses an explicit binary.
- The `omnia@v1` capability is resolvable in the local development environment.

## Inputs

Create a short feature brief in the project workspace at `docs/inventory-adjustments.md`:

```markdown
# Inventory Adjustments

The inventory service needs a controlled way for operations staff to adjust
stock counts when a warehouse audit finds a mismatch.

## Goals

- Record manual stock adjustments for a SKU and warehouse.
- Require an adjustment reason and operator identifier.
- Reject adjustments that would make available stock negative.
- Emit an audit event after a successful adjustment.

## Scope

Keep the first release small. Do not add bulk imports, approval workflows, or
warehouse transfer logic.
```

## Invocation

### 1. Prepare a disposable project

Create a disposable directory:

```text
plan-inventory-service/
```

Initialize it:

```bash
cd plan-inventory-service
specify init omnia@v1
```

Create `docs/inventory-adjustments.md` from the **Inputs** section.

### 2. Plan the change

Run `/change:plan` from the project root:

```text
/change:plan inventory-adjustments from docs/inventory-adjustments.md

Plan a small single-project change from docs/inventory-adjustments.md.

Expected shape:
- one or more local Omnia slices for inventory adjustment behavior
- no project routing fields because this workspace is not registered as a
  multi-project hub
- dependencies only where one local slice genuinely depends on another

Keep the plan small and happy-path only.
```

After planning, validate and inspect the plan:

```bash
specify change plan validate
specify change plan status
```

Do not run `/change:execute`. This scenario ends after plan validation and inspection.

## Expected Artifacts

The run should leave these artifacts or states for inspection:

- `plan.yaml` exists after `/change:plan` and validates cleanly.
- `.specify/plans/inventory-adjustments/discovery.md` records the supplied documentation input.
- `.specify/plans/inventory-adjustments/proposal.md` records the proposed local slice entries.
- The plan has one or more named entries with coherent local scopes.
- Plan entries do not invent `project` assignments in this single-project workspace.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/change:plan`.
- `plan-validates`: `specify change plan validate` exits cleanly.
- `slices-match-expected-shape`: entries are named, scoped, and ordered consistently with the inventory adjustment brief.
- `no-project-routing-required`: entries do not include project routing fields or registry-derived assignments.

## Negative Expectations

These are the guardrails for this first plan-generation pass:

- `automated-runner-added`: this scenario pack must not add a Deno, Rust, shell, Cursor SDK, or other automated test runner.
- `fake-forge-added`: this scenario pack must not add fake `gh`, fake GitHub, or fake forge behavior.
- `transcript-replay-added`: this scenario pack must not require recorded agent transcripts or replay fixtures.
- `ci-target-added`: this scenario pack must not add a CI job, `make` target, or required automated acceptance check.
- `golden-output-required`: this scenario pack must not require byte-for-byte generated prose, code, plan text, or transcript comparisons.

## Cleanup

Use a disposable directory and remove it when the run is complete unless a failure needs investigation. Preserve these items on failure:

- completed [run summary](run-summary-template.md)
- `docs/inventory-adjustments.md`
- `plan.yaml`
- `.specify/plans/inventory-adjustments/`
- `specify change plan validate` output
- `specify change plan status` output
