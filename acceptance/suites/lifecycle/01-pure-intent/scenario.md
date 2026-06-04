---
id: pure-intent
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine, build, merge]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - intent-single-lead
  - gate-1-not-auto-stamped
  - sources-intent-only
  - execute-loop-all-done
expected-artifacts:
  - plan.yaml
  - .specify/plans/fix-typo/discovery.md
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

Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`. No brief file — the intent is the `/spec:plan` argument.

## Invocation

1. **Plan** — `/spec:plan fix-typo "fix typo in user.rs"`. Confirm it writes `change.md` + `plan.yaml`, validates, and stops at `pending` printing the literal `specify plan transition fix-typo approved`.
2. **Stamp Gate 1** — run that literal transition command.
3. **Execute** — `/spec:execute` (or the loop); confirm it reaches `all-done`.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly.
- `intent-single-lead`: the degenerate `intent` survey produces exactly one lead / one slice.
- `gate-1-not-auto-stamped`: `/spec:plan` exits at `pending` and prints the transition command; it does not stamp `approved` itself.
- `sources-intent-only`: the slice's provenance is `Sources: [intent]`.
- `execute-loop-all-done`: after the operator stamps Gate 1, `/spec:execute` reaches `all-done`.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
