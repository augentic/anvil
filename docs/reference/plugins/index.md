# Plugins

Specify ships as a Cursor plugin marketplace containing seven plugins. Each plugin provides specialist skills and reference documentation for a specific domain.

## Plugin model

Plugins are installed from the Cursor marketplace (Settings > Plugins > search for "Augentic"). Each plugin bundles:

- **Skills** -- agent-driven orchestrators invoked with a slash-command prefix (e.g. `/omnia:crate-writer`).
- **Rules** -- `.mdc` files that provide context to the agent.
- **References** -- markdown documents that skills read for domain knowledge.

## Workspace rules

Installing plugins from the marketplace gives you each plugin's rules and skills. For cross-plugin coordination, copy the workspace rule from the Specify repository (`.cursor/rules/project.mdc`) into your project's `.cursor/rules/` directory.

## Plugin overview

| Plugin | Prefix | Purpose | Reference |
|--------|--------|---------|-----------|
| **Specify** | `/spec:` | Per-slice workflow orchestration: init, define, build, merge, drop, extract, analyze. Carries deprecation shims for the historical `/change:plan` and `/change:execute` commands while RFC-13 §3.9 lands. | [Change Skills](../slice-skills/index.md) |
| **Change** | `/change:` | Cross-repo change orchestration: `/change:plan` (Layer 3 authoring + Layer 4 umbrella under `--orchestrate`) and `/change:execute` (Layer 2 driver). | [Initiative Skills](../change-skills/index.md), [Change](change.md) |
| **Omnia** | `/omnia:` | Rust WASM crate generation and review | [Omnia](omnia.md) |
| **Vectis** | `/vectis:` | Cross-platform Crux app generation | [Vectis](vectis.md) |
| **Contract** | `/contract:` | API contract generation, validation, and import (OpenAPI, AsyncAPI, JSON Schema) | [Contract](contract.md) |
| **RT** | `/rt:` | Migration fixtures and regression testing | [RT](rt.md) |
| **Client** | `/client:` | Client-facing deliverables (SoW, proposals, pricing) | [Client](client.md) |

## How plugins compose with capabilities

The **Specify** plugin provides the workflow skeleton. Capabilities determine which specialist plugin skills are invoked during the build phase:

- **Omnia capability** invokes `/omnia:*` skills.
- **Vectis capability** invokes `/vectis:*` skills.

The Contract, RT, and Client plugins are capability-independent. The Contract plugin is invoked by the `contracts` brief in every capability's define pipeline (Omnia, Vectis, and Contracts) — the brief id, capability name, and `contracts/` baseline directory keep their original names; the Cursor plugin and slash-command surface live under `/contract:*`. RT and Client support migration and client-facing deliverables regardless of the target platform.

## Artifact flow

```text
/spec:define  -->  generates artifacts using capability briefs
/spec:build   -->  delegates tasks to specialist plugin skills
/spec:merge   -->  merges specs into baseline (capability-agnostic)
```

Specialist skills read the artifacts produced by `/spec:define` and generate code. The artifacts are the interface between the core workflow and the specialist plugins.
