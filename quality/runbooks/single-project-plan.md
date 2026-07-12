---
id: single-project-plan
owner: scenarios
kind: suite
entrypoint: /spec:plan
stages: [plan]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - slices-match-expected-shape
  - no-project-routing-required
expected-artifacts:
  - plan.yaml
  - discovery.md
negative-expectations:
  - live-model-ci-required
  - semantic-byte-golden-required
---

# Single-project plan generation

Scenario ID: `single-project-plan`

## Intent

Prove that `/spec:plan` turns a short brief in one initialized project into a small, valid `plan.yaml` with local slice entries only — no invented project routing. The scenario checks durable plan structure and stops at Gate 1.

## Setup

Follow the **single-project setup** in [`reference/setup.md`](../reference/setup.md) with `specify init omnia@1.0.0`. Create a short feature brief at `docs/inventory-adjustments.md`:

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

1. **Plan** — `/spec:plan inventory-adjustments from docs/inventory-adjustments.md`, asking for one or more local Omnia slices, no project routing, and dependencies only where one local slice genuinely depends on another.
2. **Validate + inspect** — `specify plan validate`; inspect `plan.yaml`. Do not run `/spec:execute`; the scenario ends after plan validation.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly.
- `slices-match-expected-shape`: entries are named, scoped, and ordered consistently with the brief.
- `no-project-routing-required`: entries do not include project routing fields or registry-derived assignments.

## Negative expectations

Live profiles remain outside per-commit CI, and semantic output is rubric-graded rather than byte-golden tested.

## Recording

Capture with [`reference/run-template.md`](../reference/run-template.md) as [`quality/runs/archive/<id>.<result>.md`](../runs/README.md).
