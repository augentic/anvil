---
id: tasks
description: Create the task list for contract validation
generates: tasks.md
needs: [specs, contracts]
---

Follow the task format conventions defined in the define skill for checkbox format, grouping, ordering, and skill directive tags.

## Agent-Completable Constraint

Generate only tasks that an agent can complete and verify with contract artifacts and local validators. Do not generate manual review, external service, production credentials, or user-confirmation tasks.

When alignment needs review, express it as a validator or writer task that produces machine-readable output. Use `contracts:validator` for `$ref` resolution, schema metadata, binding completeness, and warning-free alignment checks.

## Available Skills

| Directive | Skill | When to Use |
|-----------|-------|-------------|
| `contracts:writer` | Generate/validate contract artifacts | Contract generation tasks |
| `contracts:validator` | Validate contract consistency | Validation tasks |

## Standard Task Groups

Contract changes produce a fixed set of validation tasks. Generate one group per interface in the specs, plus a cross-cutting validation group:

### Per-interface tasks

For each interface in `specs/`, assign a sequential group number `<N>` (starting at 1):

- [ ] `<N>`.1 Generate contract artifacts for `<interface>` <!-- skill: contracts:writer -->
- [ ] `<N>`.2 Validate `<interface>` contract artifacts <!-- skill: contracts:validator -->

### Cross-cutting validation

Use the next group number after the last per-interface group (i.e. `<N+1>`):

- [ ] `<N+1>`.1 Validate `$ref` resolution across all contract files <!-- skill: contracts:validator -->
- [ ] `<N+1>`.2 Verify schema metadata completeness (`$id`, `title`, `description`) <!-- skill: contracts:validator -->
- [ ] `<N+1>`.3 Verify binding completeness for every spec-referenced schema <!-- skill: contracts:validator -->
- [ ] `<N+1>`.4 Verify the alignment report has no unresolved warnings <!-- skill: contracts:validator -->
