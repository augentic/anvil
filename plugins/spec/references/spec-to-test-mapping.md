# Spec-to-Test Mapping (shared discipline)

Target-neutral rules for mapping Specify spec scenarios to test functions. Both the Omnia and Vectis targets layer their concrete setup/assertion vocabularies on top of this base; see each target's own mapping reference for the target-specific deltas (test location, test attribute, WHEN/THEN translation tables, and worked examples).

The mapping is **deterministic** — the same spec always produces the same test structure.

## Scenario to test function

Each `#### Scenario:` under a `### Requirement:` maps to exactly one test function. The requirement's stable `ID: REQ-XXX` line is the traceability key.

Naming convention: `test_<unit_snake>_<scenario_snake>`, where `<unit_snake>` is the spec directory name converted to snake_case (replace `-` with `_`) and `<scenario_snake>` is the scenario title converted to snake_case.

```text
#### Scenario: Successful item fetch
  →  fn test_<unit_snake>_successful_item_fetch()
```

The REQ-ID, not the title, is the durable link. If a requirement title is renamed but keeps the same ID, the test stays linked; if a scenario title changes, update the comment but keep the REQ-ID reference.

## Requirement coverage

A requirement with N scenarios produces N test functions — one per scenario:

```markdown
### Requirement: Item management
ID: REQ-001
#### Scenario: Add new item
#### Scenario: Delete existing item
#### Scenario: Delete non-existent item
```

produces three test functions named `test_<unit_snake>_add_new_item`, `test_<unit_snake>_delete_existing_item`, and `test_<unit_snake>_delete_non_existent_item`.

**Validation requirements** follow the same rule: construct the invalid input described by the scenario's WHEN clause, then assert the error/rejection described by its THEN clause. The concrete construction and assertion idioms are target-specific.

## Traceability comments

Every spec-mapped test carries a doc comment linking it back to the source requirement and scenario via the stable REQ-ID:

```rust
/// Spec: specs/<domain>/spec.md > REQ-001 > Scenario: Add new item
```

Tests without a `/// Spec:` traceability comment are treated as manually added and are not flagged by drift detection.

## Drift detection mechanics

Traceability comments enable automated drift detection between specs and tests:

- **Missing coverage** — parse every `### Requirement:` / `ID: REQ-XXX` / `#### Scenario:` from the spec, parse every spec-mapped test, then report scenarios with no corresponding test (matched on REQ-ID + scenario title).
- **Stale tests** — for each test carrying a `/// Spec:` comment, verify the referenced requirement ID and scenario still exist in the spec; report tests referencing removed scenarios.
- **Assertion drift** — compare the spec scenario's THEN clauses against the test's assertions and report mismatches.

Assertion-drift comparison is approximate: it catches obvious divergences but may not detect subtle logic changes.
