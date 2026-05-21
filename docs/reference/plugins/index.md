# Plugins

Specify ships as a Cursor plugin marketplace containing six plugins. Each plugin provides specialist skills and reference documentation for a specific domain.

## Plugin model

Plugins are installed from the Cursor marketplace (Settings > Plugins > search for "Augentic"). Each plugin bundles:

- **Skills** — agent-driven orchestrators invoked with a slash-command prefix (e.g. `/omnia:crate-writer`).
- **Rules** — `.mdc` files that provide context to the agent.
- **References** — markdown documents that skills read for domain knowledge.

This repository is the Cursor-shaped distribution of the skills. The plugin manifests, slash-command routing, and `<!-- skill: plugin:skill -->` delegation directives are Cursor-specific.

## Workspace rules

Installing plugins from the marketplace gives you each plugin's rules and skills. For cross-plugin coordination, copy the workspace rule from the Specify repository (`.cursor/rules/project.mdc`) into your project's `.cursor/rules/` directory.

## Plugin overview

| Plugin       | Prefix      | Purpose                                                                                                            | Reference            |
| ------------ | ----------- | ------------------------------------------------------------------------------------------------------------------ | -------------------- |
| **Specify**  | `/spec:`    | Workflow orchestration: `init`, `plan`, `refine`, `execute`, `build`, `merge`, `finalize`, `drop`.                  | [Slice Skills](../slice-skills/index.md) |
| **Omnia**    | `/omnia:`   | Rust WASM crate generation and review                                                                              | [Omnia](omnia.md)    |
| **Vectis**   | `/vectis:`  | Cross-platform Crux app generation                                                                                 | [Vectis](vectis.md)  |
| **Contract** | `/contract:` | API contract generation, validation, and import (OpenAPI, AsyncAPI, JSON Schema)                                 | [Contract](contract.md) |
| **RT**       | `/rt:`      | Migration fixtures and regression testing                                                                          | [RT](rt.md)          |
| **Client**   | `/client:`  | Client-facing deliverables (SoW, proposals, pricing)                                                               | [Client](client.md)  |

## How plugins compose with target adapters

The **Specify** plugin provides the workflow skeleton. **Target adapters** (under `targets/<name>/`) own `shape`, `build`, and `merge` briefs — the build brief is what determines which specialist plugin skills are invoked during the `/spec:build` phase:

- **Omnia target** invokes `/omnia:*` skills.
- **Vectis target** invokes `/vectis:*` skills.
- **Contracts target** invokes `/contract:*` skills.

The RT and Client plugins are target-independent — RT supports legacy migration regardless of the target platform; Client supports operator-facing deliverables.

## Artifact flow

```text
/spec:plan     →  enumerates each bound source, proposes slices[]
/spec:refine   →  extracts evidence per source, synthesizes proposal/spec/design/tasks via core
/spec:build    →  delegates tasks to specialist plugin skills via target build brief
/spec:merge    →  applies deltas to baseline (target-agnostic)
```

Specialist skills read the artifacts produced by `/spec:refine` and generate code. The artifacts are the interface between core synthesis and the specialist plugins.
