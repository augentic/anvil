# CLI Reference

The `specify` CLI is the Layer 1 foundation that every skill builds on. It owns all deterministic operations: creating and transitioning slices, validating artifacts, parsing tasks, merging specs, and managing change plans.

## Installation

When setting up a project through the Specify plugin, `/spec:init` can bootstrap a missing CLI after confirmation. For manual setup:

```bash
brew install augentic/tap/specify
```

See [Prerequisites](../../orientation/prerequisites.md) for all install paths and capability-specific tooling.

## Conventions

- All commands return structured JSON on stdout and use exit codes for success/failure.
- Commands that modify `.specify/` state are idempotent where possible.
- The CLI enforces the legal set of lifecycle transitions and validates inputs.
- Skills delegate to the CLI for all structural operations -- they never hand-edit `.metadata.yaml` or manipulate the directory structure directly.

## Design principle

The CLI owns operations that require understanding `.specify/` directory structure or spec format. Operations that require semantic understanding or context (like deciding what to build) stay with the agent.

For the rationale behind this split, see [CLI owns correctness, agent owns judgment](../../explanation/decision-log.md#cli-owns-correctness-agent-owns-judgment) in the Decision Log.

| Use CLI when | Use agent when |
|-------------|---------------|
| The operation must be idempotent | The response depends on context |
| The output is structured (JSON, exit codes) | The output is natural language |
| Correctness is verifiable (schema validation, manifest checks) | Correctness requires semantic understanding |
| The operation is repeated across many skills | The operation is unique to one skill |
| Failure modes are enumerable | Failure modes are open-ended |

## Command families

| Family | Purpose | Reference |
|--------|---------|-----------|
| [specify status](status.md) | Project dashboard -- registry summary, plan progress, active slices | Top-level convenience |
| [specify slice](slice.md) | Per-slice CRUD, validation, merge, task tracking, outcome, journal | Layer 2 operations |
| [specify change plan](plan.md) | Scaffold, populate, validate, and transition change plans | Layer 3 operations (subresource of `specify change`) |
| [specify change](change.md) | Manage the operator-authored change brief and finalize landed changes | Layers 3–4 closure |
| [specify registry](registry.md) | Manage the platform registry at `registry.yaml` | Multi-repo platform |
| [specify capability](capability.md) | Capability resolution and brief pipeline queries | Capability infrastructure |
| [specify context](context.md) | Generate and check refreshable `AGENTS.md` guidance | Agent context |
| [specify tool](tool.md) | Resolve, cache, and run declared WASI helper tools | Deterministic extension runner |
| [specify workspace](workspace.md) | Materialise, inspect, and push workspace peer clones | Multi-repo operations |
| [specify init](init.md) | Project scaffold | One-time setup |
| [specify migrate](migrate.md) | One-shot layout migrations (currently `v2-layout`) | Upgrade path |
| [Vectis WASI tools](vectis.md) | Declared `vectis-validate` and `vectis-scaffold` tools run through `specify tool run` | Capability-owned validation + render-only scaffolding |

The previous standalone families `specify validate`, `specify spec`, `specify task`, and `specify merge` were absorbed into [`specify slice`](slice.md) (`slice validate`, `slice merge {preview, conflict-check, run}`, `slice task {progress, mark}`). See [Migrating CLI v1](../../explanation/migrating-cli-v1.md) for the full rename map.
