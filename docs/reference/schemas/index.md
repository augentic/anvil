# Schemas

A schema is a configuration package that tells Specify how to generate artifacts and build code for a specific target platform. Schemas are selected at `/spec:init` time and cached locally in `.specify/.cache/`.

## What a schema provides

Each schema declares:

| Component | Purpose |
|-----------|---------|
| `schema.yaml` | Pipeline stages, domain context, and per-stage brief references |
| `briefs/` | Markdown brief files for each phase (define, build, merge) |

## Brief pipelines

Schemas declare **brief pipelines** for each phase of the define-build-merge loop. A brief pipeline is an ordered sequence of prompts that the agent executes:

### Define pipeline (artifact generation)

Briefs run in dependency order to produce the change artifacts. The core pipeline (shared by Omnia and Vectis):

1. **proposal.md** -- generates `proposal.md`
2. **specs.md** -- generates `specs/<capability>/spec.md` (requires proposal)
3. **contracts.md** -- validates spec alignment with baseline contracts and generates delta (requires specs)
4. **design.md** -- generates `design.md` (requires proposal, contracts)
5. **tasks.md** -- generates `tasks.md` (requires specs + design)

The Vectis schema inserts an additional stage between contracts and design:

- **composition.md** -- generates `composition.yaml` (requires specs, proposal) -- screen layout with regions, groups, bindings, and event wiring

The Contracts schema uses a reduced pipeline: proposal, specs, contracts, tasks (no design or composition stages). Its build phase delegates to the format-appropriate `/interfaces:*` skill (verifier intent: `/interfaces:openapi`, `/interfaces:asyncapi`, or `/interfaces:json-schema`) rather than code-generation skills.

### Build pipeline (implementation)

A single brief that drives the implementation phase:

1. **build.md** -- reads tasks, delegates to specialist skills, verifies output

### Merge pipeline (finalisation)

A single brief that drives the merge phase:

1. **merge.md** -- previews, conflict-checks, and commits the merge

## Schema-agnostic vs schema-specific

The **lifecycle** (states, transitions, archiving) and the **core artifacts** (proposal, specs, contracts, design, tasks) are schema-agnostic. Schemas may add additional stages to the pipeline (e.g. Vectis adds `composition` between contracts and design). The Contracts schema omits design entirely, since contract changes define interface shapes rather than implementation.

What varies between schemas:

- The **content** of each brief (prompts, domain context, constraints).
- The **specialist skills** invoked during build (Omnia skills vs Vectis skills).
- The **domain context** injected into briefs (Omnia patterns vs Crux patterns).

## Schema resolution

Schema URLs follow the format:

```
https://github.com/augentic/specify/schemas/<name>[@<ref>]
```

The `@ref` suffix pins a specific version. Without it, the latest version is used.

The CLI resolves schemas via `specify schema resolve <url>`, which returns the local cache path. Skills use this to locate brief files at runtime.

## Available schemas

| Schema | Reference |
|--------|-----------|
| [Omnia](omnia.md) | Rust WASM services |
| [Vectis](vectis.md) | Cross-platform Crux applications |
| [Contracts](contracts.md) | API contract definition and validation |
