# Contracts Schema

- **URL**: `https://github.com/augentic/specify/schemas/contracts`
- **Purpose**: Dedicated API contract changes — defining or importing machine-readable interface shapes (JSON Schema, OpenAPI 3.1, AsyncAPI 3.0) without generating implementation code
- **Source**: Manual
- **Target**: Contract artifacts (`.specify/contracts/`)
- **Workflow**: `proposal` -> `specs` -> `contracts` -> `tasks` -> `build` (validation only)

## Contents

| File | Description |
|------|-------------|
| `schema.yaml` | Pipeline stages, domain context, and per-stage brief references |
| `briefs/proposal.md` | Generation brief for the proposal stage |
| `briefs/specs.md` | Generation brief for the specs stage |
| `briefs/contracts.md` | Generation brief for the contracts stage |
| `briefs/tasks.md` | Generation brief for the tasks stage |
| `briefs/build.md` | Validation brief for the build stage (no code generation) |
| `briefs/merge.md` | Merge brief for finalizing a change |

## Pipeline

### Define

| Stage | Brief | Purpose |
|-------|-------|---------|
| proposal | briefs/proposal.md | Interface scope and motivation |
| specs | briefs/specs.md | Interface-level behavioral requirements |
| contracts | briefs/contracts.md | Derive contract artifacts from specs |
| tasks | briefs/tasks.md | Validation task list |

### Build

| Stage | Brief | Purpose |
|-------|-------|---------|
| build | briefs/build.md | Validate contract artifacts (no code generation) |

### Merge

| Stage | Brief | Purpose |
|-------|-------|---------|
| merge | briefs/merge.md | Standard merge into baseline |

## Blueprints

The schema declares four blueprints in dependency order:

1. **proposal** — interface scope and motivation (`proposal.md`)
2. **specs** — interface-level behavioral requirements (`specs/**/*.md`), requires proposal
3. **contracts** — contract artifacts from specs (`contracts/**/*.yaml`), requires specs
4. **tasks** — validation task list (`tasks.md`), requires specs + contracts

Build requires tasks to be complete and is tracked via `tasks.md`.

## When to Use

Use the `contracts` schema when:
- Defining a new API contract before implementation begins (contract-first pattern)
- Importing an external or legacy API contract (contract-given pattern)
- Modifying existing platform contracts independently of implementation changes

Use Omnia or Vectis schemas when:
- Implementing code that conforms to existing contracts
- The `contracts` brief in those schemas validates alignment automatically

## Schema Framework

For general schema concepts — directory structure, field reference for `schema.yaml`, schema resolution, composition, caching, and rules override — see the [Schemas README](../README.md).
