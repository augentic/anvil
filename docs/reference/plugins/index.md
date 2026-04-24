# Plugins

Specify ships as a Cursor plugin marketplace containing five plugins. Each plugin provides specialist skills and reference documentation for a specific domain.

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
| **Specify** | `/spec:` | Core workflow orchestration | [Change Skills](../change-skills/index.md), [Initiative Skills](../initiative-skills/index.md) |
| **Omnia** | `/omnia:` | Rust WASM crate generation and review | [Omnia](omnia.md) |
| **Vectis** | `/vectis:` | Cross-platform Crux app generation | [Vectis](vectis.md) |
| **RT** | `/rt:` | Migration fixtures and regression testing | [RT](rt.md) |
| **Plan** | `/plan:` | Statement of Work generation | [Plan](plan.md) |

## How plugins compose with schemas

The **Specify** plugin provides the workflow skeleton. Schemas determine which specialist plugin skills are invoked during the build phase:

- **Omnia schema** invokes `/omnia:*` skills.
- **Vectis schema** invokes `/vectis:*` skills.

The RT and Plan plugins are schema-independent -- they support migration and business analysis regardless of the target platform.

## Artifact flow

```text
/spec:define  -->  generates artifacts using schema briefs
/spec:build   -->  delegates tasks to specialist plugin skills
/spec:merge   -->  merges specs into baseline (schema-agnostic)
```

Specialist skills read the artifacts produced by `/spec:define` and generate code. The artifacts are the interface between the core workflow and the specialist plugins.
