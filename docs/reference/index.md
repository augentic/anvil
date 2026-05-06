# Reference

The reference section is the lookup table for Specify. Use it when you need precise detail on a specific skill, CLI command, artifact format, or configuration file.

## Sections

### Core concepts

- [Artifact Format](artifact-format.md) -- full specification of the core artifacts (proposal, spec, design, tasks) and capability-specific artifacts (composition), including delta formats, tags, and validation checklists.
- [Lifecycle](lifecycle.md) -- state diagram, transitions, `.metadata.yaml` shape, plan entry states.
- [Directory Layout](directory-layout.md) -- annotated `.specify/` tree with explanations.

### Skills

- [Change Skills (Layer 2)](change-skills/index.md) -- the define-build-merge loop and supporting skills (init, drop, status, verify, explore, extract).
- [Initiative Skills (Layers 3 & 4)](change-skills/index.md) -- `/change:plan`, `/change:execute`, and `/spec:analyze` for multi-slice changes (Layer 3); the `/change:plan --orchestrate` umbrella mode (formerly `/spec:initiative`, then `/change:plan --orchestrate` until RFC-13 §3.9) for cross-repo changes end-to-end (Layer 4).

### CLI

- [CLI Reference](cli/index.md) -- all `specify` subcommands grouped by family (status, change, plan, initiative, registry, workspace, capability, init, vectis).

### Plugins

- [Plugins](plugins/index.md) -- specialist skills organized by plugin (Omnia, Vectis, RT, Client).

### Capabilities

- [Capabilities](capabilities/index.md) -- brief pipelines, specialist skills, and domain context for each first-party capability (Omnia, Vectis, Contracts).

### Configuration

- [Configuration Files](configuration.md) -- `project.yaml`, `plan.yaml`, `registry.yaml`, `change.md`, `.metadata.yaml`.

## Finding what you need

| I want to... | Go to... |
|-------------|---------|
| Understand how a skill works | [Change Skills](change-skills/index.md) or [Initiative Skills](change-skills/index.md) |
| Look up a CLI command | [CLI Reference](cli/index.md) |
| Check artifact format | [Artifact Format](artifact-format.md) |
| Understand lifecycle states | [Lifecycle](lifecycle.md) |
| Configure my project | [Configuration Files](configuration.md) |
| See what a plugin provides | [Plugins](plugins/index.md) |
| Understand capability differences | [Capabilities](capabilities/index.md) |
| Look up a term | [Glossary](../appendices/glossary.md) |
| Troubleshoot an error | [Troubleshooting](../appendices/troubleshooting.md) |

## Design decisions

For the reasoning behind Specify's architectural choices, see the [Decision Log](../explanation/decision-log.md).
