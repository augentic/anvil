# Scenario #1 -- Pure intent, one slice

> **Release blocker.** Single-release collapse means N=1 `/spec:plan` ergonomics surface to every operator at once.

## Source

[`rfcs/rfc-25-workflow.md` §Acceptance scenarios](../../../../rfcs/rfc-25-workflow.md#acceptance-scenarios), row #1.

> Operator runs `/spec:plan fix-typo "fix typo in user.rs"`.

**Stress-tests:** Degenerate `intent.enumerate`; Gate 1 ergonomics on trivial work; `change.md` + `plan.yaml` justifiability at N=1; `Sources: [intent]` provenance; `/spec:plan` exits at `pending` and prints the literal `specify plan transition fix-typo reviewed` command -- the operator runs it, then `/spec:execute`. The skill never auto-stamps `reviewed`.

## Run-summary

Status: **pending**

Operator: copy the field-set from [`tests/cross-repo/run-summary-template.md`](../../run-summary-template.md) into this file, fill every section against the live run, and update the **Status:** line above to `passed` / `failed` / `deferred`. Halt all subsequent scenarios on `failed` per the queue rule in [`README.md`](README.md).
