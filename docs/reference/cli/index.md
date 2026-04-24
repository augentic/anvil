# CLI Reference

The `specify` CLI is the Layer 1 foundation that every skill builds on. It owns all deterministic operations: creating and transitioning changes, validating artifacts, parsing tasks, merging specs, and managing initiative plans.

## Installation

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
| [specify status](status.md) | Show active changes and progress | Top-level convenience |
| [specify change](change.md) | Create, inspect, transition, and archive individual changes | Layer 2 operations |
| [specify plan](plan.md) | Scaffold, populate, validate, and transition initiative plans | Layer 3 operations |
| [specify initiative](initiative.md) | Manage initiative brief and platform registry | Layer 3 setup |
| [specify schema](schema.md) | Schema resolution and brief pipeline queries | Schema infrastructure |
| [specify spec](spec.md) | Merge preview and baseline drift detection | Pre-merge checks |
| [specify validate](validate.md) | Structural and semantic artifact validation | Quality gates |
| [specify task](task.md) | Task progress tracking and checkbox manipulation | Build phase support |
| [specify merge](merge.md) | Commit delta merge and archive | Terminal merge operation |
| [specify workspace](workspace.md) | Materialise, inspect, and push workspace peer clones | Multi-repo operations |
| [specify init](init.md) | Project scaffold | One-time setup |
| [specify vectis](vectis.md) | Cross-platform Crux project scaffold and verification | Vectis-specific tooling |
