# Reference

The reference section is the lookup table for Specify. Use it when you need precise detail on a specific skill, CLI command, artifact format, or configuration file.

## Sections

### Core concepts

- [Artifact Format](artifact-format.md) -- full specification of the core artifacts (proposal, spec, design, tasks) and capability-specific artifacts (composition), including delta formats, tags, and validation checklists.
- [Lifecycle](lifecycle.md) -- state diagram, transitions, `.metadata.yaml` shape, plan entry states.
- [Directory Layout](directory-layout.md) -- annotated `.specify/` tree with explanations.

### Skills

- [Slice Skills](slice-skills/index.md) -- the define-build-merge loop and supporting skills (init, drop, extract).
- [Change Skills](change-skills/index.md) -- `/change:plan`, `/change:execute`, and `/spec:analyze` for multi-slice changes; `/change:plan <name> orchestrate` for cross-repo changes end-to-end.

### CLI

- [CLI Reference](cli/index.md) -- all `specify` subcommands grouped by family (status, slice, change, registry, workspace, capability, codex, tool, init, plus the Vectis WASI tools run through `specify tool`).
- [Declared Tool Helper Inventory](declared-tool-helper-inventory.md) -- first-party helper migration boundary for `specify tool run`.

### Plugins

- [Plugins](plugins/index.md) -- specialist skills organized by plugin (Omnia, Vectis, RT, Client).

### Capabilities

- [Capabilities](capabilities/index.md) -- brief pipelines, specialist skills, and domain context for each first-party capability (Omnia, Vectis, Contracts).

### Configuration

- [Configuration Files](configuration.md) -- `project.yaml`, `plan.yaml`, `registry.yaml`, `change.md`, `.metadata.yaml`.

## Finding what you need

| I want to... | Go to... |
|-------------|---------|
| Understand how a skill works | [Slice Skills](slice-skills/index.md) or [Change Skills](change-skills/index.md) |
| Look up a CLI command | [CLI Reference](cli/index.md) |
| Check artifact format | [Artifact Format](artifact-format.md) |
| Understand lifecycle states | [Lifecycle](lifecycle.md) |
| Configure my project | [Configuration Files](configuration.md) |
| See what a plugin provides | [Plugins](plugins/index.md) |
| Understand capability differences | [Capabilities](capabilities/index.md) |
| Look up a term | [Glossary](../appendices/glossary.md) |
| Troubleshoot an error | [Troubleshooting](../how-to/troubleshooting/index.md) |

## Design decisions

For the reasoning behind Specify's architectural choices, see the [Decision Log](../explanation/decision-log.md).
