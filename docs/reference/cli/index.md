# CLI Reference

The `specify` CLI is the foundation every skill builds on. It owns all deterministic operations: creating and transitioning slices, validating artifacts, parsing tasks, merging specs, and managing change plans.

## Installation

When setting up a project through the Specify plugin, `/spec:init` can bootstrap a missing CLI after confirmation. For manual setup:

```bash
cargo install --git https://github.com/augentic/specify
```

or download a platform archive (with `.sha256` verification) from the GitHub Releases page. See [Prerequisites](../../orientation/prerequisites.md) for all install paths and adapter-specific tooling.

## Conventions

- All commands return structured JSON on stdout and use exit codes for success/failure.
- Commands that modify `.specify/` state are idempotent where possible.
- The CLI enforces the legal set of lifecycle transitions and validates inputs.
- Skills delegate to the CLI for all structural operations -- they never hand-edit `metadata.yaml` or manipulate the directory structure directly.

## Design principle

The CLI owns operations that require understanding `.specify/` directory structure or spec format. Operations that require semantic understanding or context (like deciding what to build) stay with the agent.

The CLI owns correctness (deterministic structural invariants); the agent owns judgment (semantic evaluation).

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
| [specify slice](slice.md) | Per-slice CRUD, synthesis, build, validation, merge, task tracking, and touched-spec tracking | Single-slice operations |
| [specify plan](plan.md) | Scaffold, populate, validate, transition, and finalize change plans | Multi-slice operations and cross-repo closure |
| [specify registry](registry.md) | Manage the platform registry at `registry.yaml` | Multi-repo platform |
| [specify adapter / source / target resolve](adapter.md) | Seed the project component cache and resolve adapters by axis | Adapter infrastructure |
| [specify workspace](workspace.md) | Materialise, prepare, and push workspace peer clones | Multi-repo operations |
| [specify init](init.md) | Project scaffold | One-time setup |
| [Vectis in-guest tools](vectis.md) | In-guest `vectis` behaviours (`validate`, `scaffold`, `sync`) inside the adapter guest | Adapter-owned validation, render-only scaffolding, and iOS scaffold repair |

Per-slice validation, spec preview/conflict checks, task progress, and merging all live under [`specify slice`](slice.md).
