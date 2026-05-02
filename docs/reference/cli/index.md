# CLI Reference

The `specify` CLI is the Layer 1 foundation that every skill builds on. It owns all deterministic operations: creating and transitioning changes, validating artifacts, parsing tasks, merging specs, and managing initiative plans.

## Installation

When setting up a project through the Specify plugin, `/spec:init` can bootstrap a missing CLI after confirmation with:

```bash
cargo install --git https://github.com/augentic/specify-cli
```

Manual install paths remain available:

```bash
brew install augentic/tap/specify           # macOS + Linux (primary)
cargo install specify                       # any platform with Rust toolchain
curl -sSfL https://specify.sh/install.sh | sh   # pre-built binary
```

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
| Correctness is verifiable (schema validation) | Correctness requires semantic understanding |
| The operation is repeated across many skills | The operation is unique to one skill |
| Failure modes are enumerable | Failure modes are open-ended |

## Command families

| Family | Purpose | Reference |
|--------|---------|-----------|
| [specify status](status.md) | Project dashboard -- registry summary, plan progress, active changes | Top-level convenience |
| [specify change](change.md) | Per-change CRUD, validation, merge, task tracking, outcome, journal | Layer 2 operations |
| [specify plan](plan.md) | Scaffold, populate, validate, and transition initiative plans | Layer 3 operations |
| [specify initiative](initiative.md) | Manage the operator-authored initiative brief and finalize landed initiatives | Layers 3–4 closure |
| [specify registry](registry.md) | Manage the platform registry at `.specify/registry.yaml` | Multi-repo platform |
| [specify schema](schema.md) | Schema resolution and brief pipeline queries | Schema infrastructure |
| [specify workspace](workspace.md) | Materialise, inspect, and push workspace peer clones | Multi-repo operations |
| [specify contract](contract.md) | Inspect and validate baseline contracts under `contracts/` | RFC-12 baseline gate |
| [specify init](init.md) | Project scaffold | One-time setup |
| [specify vectis](vectis.md) | Cross-platform Crux project scaffold and verification | Vectis-specific tooling |

The previous standalone families `specify validate`, `specify spec`, `specify task`, and `specify merge` were absorbed into [`specify change`](change.md) (`change validate`, `change merge {preview, conflict-check, run}`, `change task {progress, mark}`). See [Migrating CLI v1](../../explanation/migrating-cli-v1.md) for the full rename map.
