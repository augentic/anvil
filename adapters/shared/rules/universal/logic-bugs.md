---
id: UNI-004
title: Logic Bugs
severity: critical
trigger: Control flow, conditions, or state transitions can produce behavior that contradicts the intended domain logic.
---

## Rule

Reason through the control flow and state transitions that implement each behavior. Missing edges, inverted conditions, impossible branches, and boundary mistakes should be fixed or traced back to a missing scenario in the spec.

## Look For

- State machine transitions with missing edges, where state A can reach state B but the handler for that transition is absent.
- Inverted boolean conditions, such as `if !condition` where `if condition` was intended.
- Off-by-one errors in index arithmetic or boundary checks.
- Conditions that are always true or always false, making one branch unreachable.

## Spec Guidance

When a state transition is missing because the spec never defined that edge case, propose a new scenario rather than an ad hoc code fix.
