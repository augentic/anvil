---
id: pure-intent
owner: scenarios
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - intent-single-lead
  - gate-1-not-auto-stamped
  - sources-intent-only
  - refine-reaches-refined
expected-artifacts:
  - plan.yaml
  - discovery.md
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
---

# Pure intent, one slice

Scenario ID: `pure-intent`

> **Release blocker.** Single-release collapse means N=1 `/spec:plan` ergonomics surface to every operator at once. If this fails, halt the whole sweep, triage, and resume from here once green.

## Intent

Prove the degenerate N=1 path: a one-line intent becomes one justifiable slice, Gate 1 is ergonomic on trivial work, provenance is `Sources: [intent]`, and `/spec:plan` exits at `pending` printing the literal transition command — the skill never auto-stamps `approved`.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../shared/setup.md) with `specify init omnia@v1`. No brief file — the intent is the `/spec:plan` argument.

## Invocation

1. **Plan** — `/spec:plan fix-typo "fix typo in user.rs"`. Confirm it writes `change.md` + `plan.yaml`, validates, and stops at `pending` printing the literal `specify plan transition fix-typo approved`.
2. **Stamp Gate 1** — run that literal transition command.
3. **Refine** — `/spec:execute` drives the approved entry through `/spec:refine`; confirm the slice synthesizes and transitions to `refined` with `Sources: [intent]` provenance. Stop after `refined` — `build` / `merge` are out of scope (see [Scope](#scope)).

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly.
- `intent-single-lead`: the degenerate `intent` survey produces exactly one lead / one slice.
- `gate-1-not-auto-stamped`: `/spec:plan` exits at `pending` and prints the transition command; it does not stamp `approved` itself.
- `sources-intent-only`: the slice's provenance is `Sources: [intent]`.
- `refine-reaches-refined`: after the operator stamps Gate 1, `/spec:execute` drives the entry through `/spec:refine`, the slice validates cleanly, and it transitions to `refined`.

## Scope

This scenario is the **N=1 planning-and-synthesis gate**, not a codegen gate. Its `stages` stop at `refine` deliberately: every in-scope assertion is deterministic structure (plan shape, Gate-1 ergonomics, `Sources:` provenance, lifecycle `refined`), so the N=1 hard halt never depends on a non-deterministic surface.

`build` / `merge` are excluded on purpose. Driving them here would force Omnia WASM create-mode codegen from a deliberately degenerate "fix typo" intent and then grade *generated-output correctness* — the framework's thinnest, irreducibly non-deterministic surface ([RFC-40 §Capability coverage map, row 2](../../rfcs/rfc-40-acceptance-capability.md)). That gate belongs to a dedicated per-target build scenario ([RFC-40 Phase 2](../../rfcs/rfc-40-acceptance-capability.md#phase-2--the-generated-output-gate-capability-2)); the orchestration half (`/spec:execute` reaching `all-done`) graduates separately via the `shape` / trace tier ([RFC-39](../../rfcs/future/rfc-39-acceptance-shape-traces.md)).

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../shared/run-summary-template.md) under [`acceptance/runs/`](../runs/README.md).
