# Reference

The reference section is the lookup table for Specify. Use it when you need precise detail on a specific skill, CLI command, artifact format, or configuration file.

## Sections

### Core concepts

- [Artifact Format](artifact-format.md) — full specification of the core artifacts (proposal, spec, design, tasks) and target-specific structured outputs, including delta formats, tags, and validation checklists.
- [Lifecycle](lifecycle.md) — the three stacked lifecycles (plan, per-entry, slice), transitions, `metadata.yaml` shape.
- [Directory Layout](directory-layout.md) — annotated `.specify/` tree with explanations.

### Skills

- [Change Skills](change-skills/index.md) — `/spec:plan`, `specify plan execute`, `/spec:finalize`
- [Slice Skills](slice-skills/index.md) — `/spec:init`, `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`

### CLI

- [CLI Reference](cli/index.md) — all `specify` subcommands grouped by family.
- [Declared Tool Helper Inventory](declared-tool-helper-inventory.md) — historical record of the first-party helper migration (helpers are in-guest adapter library code today).

### Plugins

- [Plugins](plugins/index.md) — specialist skills organized by plugin (Specify, Capture, Client).

### Adapters

- [Source Adapters](sources/index.md) — `survey` + `extract` per first-party source adapter (intent, documentation, typescript, screenshots, captures).
- [Target Adapters](targets/index.md) — `guidance` + `build` + `merge` per first-party target adapter (Omnia, Vectis, Contracts).

### Configuration

- [Configuration Files](configuration.md) — `project.yaml`, `plan.yaml`, `registry.yaml`, `change.md`, `metadata.yaml`.

### Engineering standards

- [Ignore Directives](ignore-directives.md) — in-source `specify-ignore` grammar and status taxonomy.

## Finding what you need

| I want to...                            | Go to...                                              |
| --------------------------------------- | ----------------------------------------------------- |
| Understand how a skill works             | [Change Skills](change-skills/index.md) or [Slice Skills](slice-skills/index.md) |
| Look up a CLI command                    | [CLI Reference](cli/index.md)                        |
| Check artifact format                    | [Artifact Format](artifact-format.md)                |
| Understand lifecycle states              | [Lifecycle](lifecycle.md)                            |
| Configure my project                     | [Configuration Files](configuration.md)              |
| See what a plugin provides               | [Plugins](plugins/index.md)                          |
| Understand the source/target split       | [Anatomy of an adapter](../explanation/adapter-anatomy.md) |
| Look up a term                            | [Glossary](../appendices/glossary.md)                |

## Design decisions

