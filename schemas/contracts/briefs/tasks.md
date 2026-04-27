---
id: tasks
description: Create the task list for contract validation
generates: tasks.md
needs: [specs, contracts]
---

Follow the task format conventions defined in the define skill for checkbox format, grouping, ordering, and skill directive tags.

## Available Skills

| Directive | Skill | When to Use |
|-----------|-------|-------------|
| `contracts:writer` | Generate/validate contract artifacts | Contract generation tasks |
| `contracts:validator` | Validate contract consistency | Validation tasks |

## Standard Task Groups

Contract changes produce a fixed set of validation tasks. Generate one group per interface in the specs, plus a cross-cutting validation group:

### Per-interface tasks

For each interface in `specs/`:

- [ ] `contracts:writer` — Generate contract artifacts for `<interface>`
- [ ] `contracts:validator` — Validate `<interface>` contract artifacts

### Cross-cutting validation

- [ ] `contracts:validator` — Validate `$ref` resolution across all contract files
- [ ] `contracts:validator` — Verify schema metadata completeness (`$id`, `title`, `description`)
- [ ] `contracts:validator` — Verify binding completeness (every spec-referenced schema has a protocol binding)
- [ ] `contracts:validator` — Review alignment report for warnings
