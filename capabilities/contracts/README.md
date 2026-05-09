# Contracts Schema

- **URL**: `https://github.com/augentic/specify/capabilities/contracts`
- **Purpose**: Dedicated API contract changes — defining or importing machine-readable interface shapes (JSON Schema, OpenAPI 3.1, AsyncAPI 3.0) without generating implementation code
- **Source**: Manual
- **Target**: Contract artifacts (`contracts/`)
- **Workflow**: `proposal` -> `specs` -> `tasks` -> `build` (author/import + validation)

## Contents

| File | Description |
|------|-------------|
| `capability.yaml` | Pipeline stages and per-stage brief references |
| `briefs/proposal.md` | Generation brief for the proposal stage |
| `briefs/specs.md` | Generation brief for the specs stage |
| `briefs/tasks.md` | Generation brief for the tasks stage |
| `briefs/build.md` | Build brief for authoring, importing, repairing, and validating contract artifacts |
| `briefs/merge.md` | Merge brief for finalizing a change |
| `codex/*.md` | Reviewer-facing interface compatibility rules for contract evolution |

## Pipeline

### Define

| Stage | Brief | Purpose |
|-------|-------|---------|
| proposal | briefs/proposal.md | Interface scope and motivation |
| specs | briefs/specs.md | Interface-level behavioral requirements or import-mode scope |
| tasks | briefs/tasks.md | Contract build and validation task list |

### Build

| Stage | Brief | Purpose |
|-------|-------|---------|
| build | briefs/build.md | Author or import contract artifacts, then validate them |

### Merge

| Stage | Brief | Purpose |
|-------|-------|---------|
| merge | briefs/merge.md | Standard merge into baseline |

## Blueprints

The schema declares three define blueprints in dependency order:

1. **proposal** — interface scope and motivation (`proposal.md`)
2. **specs** — interface-level behavioral requirements (`specs/**/*.md`), requires proposal
3. **tasks** — contract build and validation task list (`tasks.md`), requires specs

Build requires tasks to be complete and is tracked via `tasks.md`.

## Codex

The Contracts codex pack owns stable `IFACE-*` rules for reviewer-facing interface evolution checks:

- OpenAPI and AsyncAPI consumer compatibility.
- JSON Schema evolution safety.
- SemVer versioning and consumer-impact classification hooks.

## When to Use

Use the `contracts` schema when:
- Defining a new API contract before implementation begins (contract-first pattern)
- Importing an external or legacy API contract (contract-given pattern)
- Modifying existing platform contracts independently of implementation changes
- Reverse-engineering contracts from an existing implementation whose API surface has been identified by `/spec:analyze` (extract-from-source pattern)

Use Omnia or Vectis capabilities when:
- Implementing code that conforms to existing contracts
- The `contracts` brief in those capabilities validates alignment automatically

## Capability Framework

For general capability concepts — directory structure, field reference for `capability.yaml`, capability resolution, composition, caching, and rules override — see the [Capabilities README](../README.md).
