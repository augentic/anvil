# Reference

The reference section is the lookup table for Emery. Use it when you need precise detail on a specific skill, CLI command, artifact format, or configuration file.

## Sections

### Core concepts

- [Artifact Format](artifact-format.md) — full specification of the core artifacts (proposal, spec, design, tasks) and target-specific structured outputs, including delta formats, tags, and validation checklists.
- [Lifecycle](lifecycle.md) — the three stacked lifecycles (plan, per-entry, slice), transitions, `metadata.yaml` shape.
- [Directory Layout](directory-layout.md) — annotated `.emery/` tree with explanations.

### Skills

- [Change Skills](change-skills/index.md) — `/emery:plan`, `emery plan execute`, `/emery:finalize`
- [Slice Skills](slice-skills/index.md) — `/emery:init`, `/emery:refine`, `/emery:build`, `/emery:merge`, `/emery:drop`

### CLI

- [CLI Reference](cli/index.md) — all `emery` subcommands grouped by family.
### Plugins

- [Plugins](plugins/index.md) — Cursor operator plugin (Emery) and its `/emery:*` skill wrappers.

### Adapters

- [Source Adapters](sources/index.md) — `survey` + `extract` per first-party source adapter (intent, documentation, typescript, screenshots, captures).
- [Target Adapters](targets/index.md) — `guidance` + `build` + `merge` per first-party target adapter (Omnia, Vectis, Contracts).

### Configuration

- [Configuration Files](configuration.md) — `project.yaml`, `plan.yaml`, `registry.yaml`, `change.md`, `metadata.yaml`.

### Engineering standards

## Finding what you need

| I want to...                            | Go to...                                              |
| --------------------------------------- | ----------------------------------------------------- |
| Understand how a skill works             | [Change Skills](change-skills/index.md) or [Slice Skills](slice-skills/index.md) |
| Look up a CLI command                    | [CLI Reference](cli/index.md)                        |
| Check artifact format                    | [Artifact Format](artifact-format.md)                |
| Understand lifecycle states              | [Lifecycle](lifecycle.md)                            |
| Configure my project                     | [Configuration Files](configuration.md)              |
| See what Cursor plugins provide          | [Plugins](plugins/index.md)                          |
| Understand the source/target split       | [Anatomy of an adapter](../explanation/adapter-anatomy.md) |
| Look up a term                            | [Glossary](../appendices/glossary.md)                |

## Design decisions

